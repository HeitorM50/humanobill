//! Rotas HTTP (docs/04 — API).

use std::{path::PathBuf, sync::Arc};

use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Multipart, Path, State, multipart::MultipartRejection},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use chrono::{Duration, Utc};
use humanobill_core::{Analysis, AnalysisSource, Answers, ExtractedBill, validate_bill};
use humanobill_extract::BillExtractor;
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

use crate::{error::ApiError, repo::Repo, upload};

mod compare;
pub mod reminders;
mod stats;
pub mod whatsapp;

pub const RETENTION_DAYS: i64 = 30;

#[derive(Clone)]
pub struct AppState {
    pub repo: Arc<dyn Repo>,
    pub extractor: Option<Arc<dyn BillExtractor>>,
    pub fixtures_dir: PathBuf,
    /// `None` desliga o rate limit (testes que não exercitam o limite).
    pub limits: Option<crate::limits::Limits>,
}

pub fn router(state: AppState) -> Router {
    let upload_route =
        post(create_upload).layer(DefaultBodyLimit::max(upload::MAX_BYTES + 64 * 1024));
    let (upload_route, writes) = match state.limits {
        Some(l) => {
            let (up, up_cfg) = crate::limits::layer(l.upload_burst, l.upload_refill);
            let (wr, wr_cfg) = crate::limits::layer(l.write_burst, l.write_refill);
            if tokio::runtime::Handle::try_current().is_ok() {
                crate::limits::spawn_cleanup(vec![up_cfg, wr_cfg]);
            }
            (upload_route.layer(up), Some(wr))
        }
        None => (upload_route, None),
    };
    let writes_router = Router::new()
        .route("/analyses/demo/{sample}", post(create_demo))
        .route("/analyses/{id}", get(get_analysis).patch(patch_analysis))
        .route("/analyses/{id}/dispute", post(create_dispute))
        .route("/comparisons", post(compare::create_comparison));
    let writes_router = match writes {
        Some(l) => writes_router.layer(l),
        None => writes_router,
    };
    Router::new()
        .route("/health", get(health))
        .route("/stats", stats::route(state.repo.clone()))
        .route("/analyses", upload_route)
        .merge(writes_router)
        .with_state(state)
}

/// Keep-alive e warm-up. Faz `select 1` para manter o banco ativo.
async fn health(State(s): State<AppState>) -> impl IntoResponse {
    let version = env!("CARGO_PKG_VERSION");
    match s.repo.ping().await {
        Ok(()) => (
            StatusCode::OK,
            Json(json!({"status": "ok", "version": version, "db": "ok"})),
        ),
        Err(e) => {
            tracing::error!(error = %e, "health: banco indisponível");
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({"status": "degraded", "version": version, "db": "down"})),
            )
        }
    }
}

pub(crate) fn new_analysis(bill: ExtractedBill, source: AnalysisSource) -> Analysis {
    let answers = Answers::new();
    let eval = humanobill_rules::evaluate(&bill, &answers);
    let recoverable_cents = humanobill_rules::recoverable_cents(&eval.findings, None);
    let now = Utc::now();
    Analysis {
        id: Uuid::new_v4().to_string(),
        created_at: now.to_rfc3339(),
        expires_at: (now + Duration::days(RETENTION_DAYS)).to_rfc3339(),
        source,
        bill,
        answers,
        paid: None,
        findings: eval.findings,
        questions: eval.questions,
        recoverable_cents,
        rules_version: humanobill_rules::RULES_VERSION.into(),
    }
}

/// Lê e valida uma fatura de exemplo pré-extraída (sem IA) e monta a análise, sem gravar.
/// Compartilhada pela rota de demo e pelo bot do WhatsApp (e seu simulador).
pub(crate) async fn analysis_from_sample(
    fixtures_dir: &std::path::Path,
    sample: &str,
) -> Result<Analysis, ApiError> {
    // Evita path traversal: só [a-z0-9-].
    if sample.is_empty()
        || !sample
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
    {
        return Err(ApiError::new(
            StatusCode::NOT_FOUND,
            "not_found",
            "Exemplo não encontrado",
        ));
    }
    let path = fixtures_dir
        .join("extracted")
        .join(format!("{sample}.json"));
    let raw = tokio::fs::read_to_string(&path)
        .await
        .map_err(|_| ApiError::new(StatusCode::NOT_FOUND, "not_found", "Exemplo não encontrado"))?;
    let bill: ExtractedBill = serde_json::from_str(&raw).map_err(|e| {
        tracing::error!(%sample, error = %e, "fixture inválida");
        ApiError::internal()
    })?;
    if !validate_bill(&bill).is_empty() {
        tracing::error!(%sample, "fixture inconsistente");
        return Err(ApiError::internal());
    }
    Ok(new_analysis(bill, AnalysisSource::Demo))
}

/// RF01: cria uma análise a partir de uma fatura de exemplo pré-extraída (sem IA).
async fn create_demo(
    State(s): State<AppState>,
    Path(sample): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let analysis = analysis_from_sample(&s.fixtures_dir, &sample).await?;
    s.repo.insert(&analysis).await?;
    tracing::info!(id = %analysis.id, %sample, "análise demo criada");
    Ok((StatusCode::CREATED, Json(analysis)))
}

/// Traduz o erro do multipart pela causa real, que o próprio Axum já classifica:
/// limite de tamanho excedido vira 413; corpo malformado ou truncado vira 400.
fn multipart_error(e: axum::extract::multipart::MultipartError) -> ApiError {
    if e.status() == StatusCode::PAYLOAD_TOO_LARGE {
        ApiError::new(
            StatusCode::PAYLOAD_TOO_LARGE,
            "too_large",
            "O arquivo deve ter até 10 MB",
        )
    } else {
        ApiError::new(
            StatusCode::BAD_REQUEST,
            "invalid_file",
            "Não consegui ler o envio. Tente enviar o arquivo de novo",
        )
    }
}

/// RF02 + RF03: upload de fatura real com consentimento obrigatório.
/// O arquivo existe só em memória durante a requisição (RNF04).
async fn create_upload(
    State(s): State<AppState>,
    form: Result<Multipart, MultipartRejection>,
) -> Result<impl IntoResponse, ApiError> {
    // Sem multipart válido (Content-Type errado ou sem boundary), o Axum responderia
    // texto puro; aqui vira o erro JSON padrão da API.
    let mut form = form.map_err(|_| {
        ApiError::new(
            StatusCode::BAD_REQUEST,
            "invalid_file",
            "Envie a fatura como arquivo, em um formulário (multipart)",
        )
    })?;
    let mut file: Option<Vec<u8>> = None;
    let mut consent = false;
    while let Some(field) = form.next_field().await.map_err(multipart_error)? {
        match field.name() {
            Some("file") => {
                let bytes = field.bytes().await.map_err(multipart_error)?;
                file = Some(bytes.to_vec());
            }
            Some("consent") => consent = field.text().await.map(|t| t == "true").unwrap_or(false),
            _ => {}
        }
    }
    if !consent {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "consent_required",
            "É preciso concordar com o aviso antes de enviar",
        ));
    }
    let bytes = file.ok_or_else(|| {
        ApiError::new(
            StatusCode::BAD_REQUEST,
            "invalid_file",
            "Envie um PDF ou uma foto (JPG, PNG)",
        )
    })?;
    let analysis = upload::analyze_upload(&s.repo, s.extractor.as_ref(), bytes).await?;
    Ok((StatusCode::CREATED, Json(analysis)))
}

/// Corpo do PATCH: respostas às perguntas (RF09) e se a fatura já foi paga (RF10).
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PatchBody {
    /// Substitui as respostas atuais. Só ids de perguntas existentes são aceitos.
    #[serde(default)]
    answers: Option<Answers>,
    /// `null` explícito volta para "não informado".
    #[serde(default, deserialize_with = "deserialize_some")]
    paid: Option<Option<bool>>,
    /// RF05: fatura corrigida pelo usuário (substitui a leitura da IA).
    #[serde(default)]
    bill: Option<ExtractedBill>,
}

/// Distingue campo ausente (`None`) de `null` (`Some(None)`).
fn deserialize_some<'de, D, T>(d: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    T::deserialize(d).map(Some)
}

/// RF09 + RF10: salva respostas/pagamento e reavalia SÓ o motor (sem IA).
async fn patch_analysis(
    State(s): State<AppState>,
    Path(id): Path<String>,
    body: Result<Json<PatchBody>, axum::extract::rejection::JsonRejection>,
) -> Result<impl IntoResponse, ApiError> {
    let Json(body) = body.map_err(|_| {
        ApiError::new(
            StatusCode::BAD_REQUEST,
            "invalid_body",
            "Dados enviados inválidos",
        )
    })?;
    let id = Uuid::parse_str(&id).map_err(|_| ApiError::not_found())?;
    let stored = s.repo.get(id).await?.ok_or_else(ApiError::not_found)?;
    if stored.expires_at <= Utc::now() {
        return Err(ApiError::new(
            StatusCode::GONE,
            "expired",
            "Este resultado expirou. Analise de novo",
        ));
    }
    let mut a = stored.analysis;

    if let Some(answers) = body.answers {
        // Perguntas possíveis: as da avaliação sem respostas + as já emitidas.
        let base = humanobill_rules::evaluate(&a.bill, &Answers::new());
        let known: std::collections::BTreeSet<&str> = base
            .questions
            .iter()
            .chain(a.questions.iter())
            .map(|q| q.id.as_str())
            .collect();
        if let Some(bad) = answers.keys().find(|k| !known.contains(k.as_str())) {
            tracing::warn!(question = %bad, "resposta para pergunta inexistente");
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "unknown_question",
                "Resposta para uma pergunta que não existe nesta análise",
            ));
        }
        a.answers = answers;
    }
    if let Some(paid) = body.paid {
        a.paid = paid;
    }
    if let Some(bill) = body.bill {
        if bill.kind != a.bill.kind {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "invalid_correction",
                "Não é possível trocar o tipo da conta",
            ));
        }
        // Soma divergente é permitida (vira o achado de soma); o resto precisa ser consistente.
        let blocking: Vec<_> = validate_bill(&bill)
            .into_iter()
            .filter(|i| !matches!(i, humanobill_core::ValidationIssue::SumMismatch { .. }))
            .collect();
        if !blocking.is_empty() {
            tracing::warn!(issues = blocking.len(), "correção inconsistente");
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "invalid_correction",
                "Os dados corrigidos estão inconsistentes. Confira os campos",
            ));
        }
        a.bill = bill;
    }

    let eval = humanobill_rules::evaluate(&a.bill, &a.answers);
    a.recoverable_cents = humanobill_rules::recoverable_cents(&eval.findings, a.paid);
    a.findings = eval.findings;
    a.questions = eval.questions;
    a.rules_version = humanobill_rules::RULES_VERSION.into();
    s.repo.update(&a).await?;
    tracing::info!(id = %a.id, answers = a.answers.len(), "análise reavaliada");
    Ok(Json(a))
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DisputeBody {
    finding_ids: Vec<String>,
    #[serde(default)]
    protocol: Option<String>,
}

/// RF11 + RF12: contestação pronta (template determinístico, sem IA) + onde reclamar.
async fn create_dispute(
    State(s): State<AppState>,
    Path(id): Path<String>,
    body: Result<Json<DisputeBody>, axum::extract::rejection::JsonRejection>,
) -> Result<impl IntoResponse, ApiError> {
    let Json(body) = body.map_err(|_| {
        ApiError::new(
            StatusCode::BAD_REQUEST,
            "invalid_body",
            "Dados enviados inválidos",
        )
    })?;
    // O protocolo nunca vai para log nem para a mensagem de erro (RNF13).
    if let Some(p) = &body.protocol
        && !humanobill_rules::dispute::protocol_is_valid(p.trim())
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "invalid_protocol",
            "Protocolo inválido: use só números e hífen (até 30)",
        ));
    }
    let id = Uuid::parse_str(&id).map_err(|_| ApiError::not_found())?;
    let stored = s.repo.get(id).await?.ok_or_else(ApiError::not_found)?;
    if stored.expires_at <= Utc::now() {
        return Err(ApiError::new(
            StatusCode::GONE,
            "expired",
            "Este resultado expirou. Analise de novo",
        ));
    }
    let a = stored.analysis;
    let mut selected = Vec::with_capacity(body.finding_ids.len());
    for fid in &body.finding_ids {
        let f = a.findings.iter().find(|f| &f.id == fid).ok_or_else(|| {
            ApiError::new(
                StatusCode::BAD_REQUEST,
                "unknown_finding",
                "Um dos problemas selecionados não existe nesta análise",
            )
        })?;
        if !selected
            .iter()
            .any(|x: &&humanobill_core::Finding| x.id == f.id)
        {
            selected.push(f);
        }
    }
    use humanobill_rules::dispute::{DisputeError, build, call_script};
    let dispute =
        build(&a.bill, &selected, a.paid, body.protocol.as_deref()).map_err(|e| match e {
            DisputeError::Empty => ApiError::new(
                StatusCode::BAD_REQUEST,
                "empty_selection",
                "Selecione pelo menos um problema para contestar",
            ),
            DisputeError::NotDisputable(_) => ApiError::new(
                StatusCode::BAD_REQUEST,
                "not_disputable",
                "Um item marcado como \"tudo certo\" não pode ser contestado",
            ),
        })?;
    // RF17: o roteiro de ligação sai junto da contestação.
    let call_script = call_script(&a.bill, &selected, a.paid);
    tracing::info!(id = %a.id, findings = selected.len(), "contestação gerada");
    Ok(Json(humanobill_core::DisputeResult {
        dispute,
        call_script,
    }))
}

/// RF13 + RF14: reabrir resultado; 410 se expirou.
async fn get_analysis(
    State(s): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let id = Uuid::parse_str(&id).map_err(|_| ApiError::not_found())?;
    let stored = s.repo.get(id).await?.ok_or_else(ApiError::not_found)?;
    if stored.expires_at <= Utc::now() {
        return Err(ApiError::new(
            StatusCode::GONE,
            "expired",
            "Este resultado expirou. Analise de novo",
        ));
    }
    Ok(Json(stored.analysis))
}

#[cfg(test)]
mod tests {
    use async_trait::async_trait;
    use axum::body::Body;
    use axum::http::{Request, header};
    use http_body_util::BodyExt;
    use humanobill_extract::{ExtractError, InputMime};
    use serde_json::Value;
    use tower::ServiceExt;

    use super::*;
    use crate::repo::MemRepo;

    fn fixtures() -> PathBuf {
        PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures"))
    }

    struct FakeExtractor;
    #[async_trait]
    impl BillExtractor for FakeExtractor {
        async fn extract(&self, _: &[u8], _: InputMime) -> Result<ExtractedBill, ExtractError> {
            let raw = std::fs::read_to_string(fixtures().join("extracted/telecom-1.json")).unwrap();
            Ok(serde_json::from_str(&raw).unwrap())
        }
    }

    fn app(extractor: bool) -> (Router, Arc<MemRepo>) {
        let repo = Arc::new(MemRepo::default());
        let state = AppState {
            repo: repo.clone(),
            extractor: extractor.then(|| Arc::new(FakeExtractor) as Arc<dyn BillExtractor>),
            fixtures_dir: fixtures(),
            limits: None,
        };
        (router(state), repo)
    }

    async fn send(app: Router, req: Request<Body>) -> (StatusCode, Value) {
        let resp = app.oneshot(req).await.unwrap();
        let status = resp.status();
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        (
            status,
            serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        )
    }

    fn multipart(consent: Option<&str>, file: Option<&[u8]>) -> Request<Body> {
        let b = "XBOUNDARYX";
        let mut body = Vec::new();
        if let Some(c) = consent {
            body.extend(
                format!("--{b}\r\nContent-Disposition: form-data; name=\"consent\"\r\n\r\n{c}\r\n")
                    .as_bytes(),
            );
        }
        if let Some(f) = file {
            body.extend(
                format!("--{b}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"fatura.pdf\"\r\nContent-Type: application/pdf\r\n\r\n")
                    .as_bytes(),
            );
            body.extend_from_slice(f);
            body.extend(b"\r\n");
        }
        body.extend(format!("--{b}--\r\n").as_bytes());
        Request::post("/analyses")
            .header(
                header::CONTENT_TYPE,
                format!("multipart/form-data; boundary={b}"),
            )
            .body(Body::from(body))
            .unwrap()
    }

    const PDF: &[u8] = b"%PDF-1.7\n1 0 obj << /Type /Page >> endobj\n";

    #[tokio::test]
    async fn health_ok() {
        let (app, _) = app(false);
        let (st, v) = send(app, Request::get("/health").body(Body::empty()).unwrap()).await;
        assert_eq!(st, StatusCode::OK);
        assert_eq!(v["db"], "ok");
    }

    // RF01: exemplo funciona sem IA e pode ser reaberto.
    #[tokio::test]
    async fn demo_cria_e_reabre_sem_ia() {
        let (app, _) = app(false);
        let (st, v) = send(
            app.clone(),
            Request::post("/analyses/demo/telecom-1")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(st, StatusCode::CREATED);
        assert_eq!(v["source"], "demo");
        assert_eq!(v["bill"]["provider_name"], "Vivaz Telecom");
        let id = v["id"].as_str().unwrap();
        let (st, v2) = send(
            app,
            Request::get(format!("/analyses/{id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(st, StatusCode::OK);
        assert_eq!(v2["id"], id);
    }

    #[tokio::test]
    async fn demo_inexistente_ou_traversal_e_404() {
        let (app, _) = app(false);
        for s in ["nao-existe", "..%2F..%2Fetc%2Fpasswd", "Telecom-1"] {
            let (st, _) = send(
                app.clone(),
                Request::post(format!("/analyses/demo/{s}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await;
            assert_eq!(st, StatusCode::NOT_FOUND, "{s}");
        }
    }

    // RF03: API rejeita envio sem consentimento.
    #[tokio::test]
    async fn upload_sem_consentimento_e_400() {
        let (app, _) = app(true);
        let (st, v) = send(app, multipart(None, Some(PDF))).await;
        assert_eq!(st, StatusCode::BAD_REQUEST);
        assert_eq!(v["error"]["code"], "consent_required");
    }

    // RF02: tipo inválido.
    #[tokio::test]
    async fn upload_tipo_invalido_e_415() {
        let (app, _) = app(true);
        let (st, v) = send(app, multipart(Some("true"), Some(b"PK\x03\x04docx-falso"))).await;
        assert_eq!(st, StatusCode::UNSUPPORTED_MEDIA_TYPE);
        assert_eq!(v["error"]["code"], "invalid_file");
    }

    #[tokio::test]
    async fn upload_valido_cria_analise() {
        let (app, _) = app(true);
        let (st, v) = send(app, multipart(Some("true"), Some(PDF))).await;
        assert_eq!(st, StatusCode::CREATED);
        assert_eq!(v["source"], "upload");
    }

    #[tokio::test]
    async fn upload_sem_ia_configurada_e_503() {
        let (app, _) = app(false);
        let (st, v) = send(app, multipart(Some("true"), Some(PDF))).await;
        assert_eq!(st, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(v["error"]["code"], "extraction_unavailable");
    }

    // RF13: link inexistente.
    #[tokio::test]
    async fn analise_inexistente_e_404() {
        let (app, _) = app(false);
        let (st, _) = send(
            app.clone(),
            Request::get(format!("/analyses/{}", Uuid::new_v4()))
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(st, StatusCode::NOT_FOUND);
        let (st, _) = send(
            app,
            Request::get("/analyses/nao-e-uuid")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(st, StatusCode::NOT_FOUND);
    }

    fn patch(id: &str, body: &str) -> Request<Body> {
        Request::patch(format!("/analyses/{id}"))
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body.to_string()))
            .unwrap()
    }

    async fn demo(app: &Router, sample: &str) -> Value {
        let (st, v) = send(
            app.clone(),
            Request::post(format!("/analyses/demo/{sample}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(st, StatusCode::CREATED);
        v
    }

    // RF07 "Conta sem problemas": a demo telecom-3 não tem achado indevido nem suspeito.
    #[tokio::test]
    async fn demo_telecom_3_nao_tem_problemas() {
        let (app, _) = app(false);
        let v = demo(&app, "telecom-3").await;
        let problemas = v["findings"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|f| f["severity"] == "indevido" || f["severity"] == "suspeito")
            .count();
        assert_eq!(problemas, 0);
        assert_eq!(v["recoverable_cents"], 0);
    }

    // RF09: "Não contratei" vira indevido e aumenta o recuperável, sem IA.
    #[tokio::test]
    async fn patch_resposta_nao_vira_indevido() {
        let (app, _) = app(false);
        let v = demo(&app, "telecom-1").await;
        let id = v["id"].as_str().unwrap();
        assert_eq!(v["recoverable_cents"], 0);
        let (st, v) = send(
            app.clone(),
            patch(id, r#"{"answers":{"TEL-01:i2":{"type":"no"}}}"#),
        )
        .await;
        assert_eq!(st, StatusCode::OK);
        let f = v["findings"]
            .as_array()
            .unwrap()
            .iter()
            .find(|f| f["rule_id"] == "TEL-01" && f["item_refs"][0] == "i2")
            .unwrap();
        assert_eq!(f["severity"], "indevido");
        assert_eq!(v["recoverable_cents"], 1290);
        // Persistiu: o GET devolve o mesmo estado.
        let (_, v2) = send(
            app,
            Request::get(format!("/analyses/{id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(v2["recoverable_cents"], 1290);
        assert_eq!(v2["answers"]["TEL-01:i2"]["type"], "no");
    }

    // RF09: "Contratei" vira ok.
    #[tokio::test]
    async fn patch_resposta_sim_vira_ok() {
        let (app, _) = app(false);
        let id = demo(&app, "telecom-1").await["id"]
            .as_str()
            .unwrap()
            .to_string();
        let (_, v) = send(
            app,
            patch(&id, r#"{"answers":{"TEL-01:i2":{"type":"yes"}}}"#),
        )
        .await;
        let f = v["findings"]
            .as_array()
            .unwrap()
            .iter()
            .find(|f| f["item_refs"][0] == "i2")
            .unwrap();
        assert_eq!(f["severity"], "ok");
    }

    // RF10: já pago = em dobro.
    #[tokio::test]
    async fn patch_paid_dobra_o_recuperavel() {
        let (app, _) = app(false);
        let id = demo(&app, "telecom-2").await["id"]
            .as_str()
            .unwrap()
            .to_string();
        let (_, v) = send(app.clone(), patch(&id, r#"{"paid":false}"#)).await;
        let simples = v["recoverable_cents"].as_i64().unwrap();
        assert!(simples > 0);
        let (_, v) = send(app.clone(), patch(&id, r#"{"paid":true}"#)).await;
        assert_eq!(v["recoverable_cents"].as_i64().unwrap(), simples * 2);
        let (_, v) = send(app, patch(&id, r#"{"paid":null}"#)).await;
        assert_eq!(v["paid"], Value::Null);
        assert_eq!(v["recoverable_cents"].as_i64().unwrap(), simples);
    }

    // Valor do plano (TEL-05) via resposta do tipo amount.
    #[tokio::test]
    async fn patch_valor_do_plano_dispara_tel05() {
        let (app, _) = app(false);
        let id = demo(&app, "telecom-2").await["id"]
            .as_str()
            .unwrap()
            .to_string();
        let (_, v) = send(
            app,
            patch(
                &id,
                r#"{"answers":{"TEL-05:plano":{"type":"amount","value":11990}}}"#,
            ),
        )
        .await;
        assert!(
            v["findings"]
                .as_array()
                .unwrap()
                .iter()
                .any(|f| f["rule_id"] == "TEL-05" && f["severity"] == "suspeito")
        );
    }

    #[tokio::test]
    async fn patch_pergunta_inexistente_e_400() {
        let (app, _) = app(false);
        let id = demo(&app, "telecom-3").await["id"]
            .as_str()
            .unwrap()
            .to_string();
        let (st, v) = send(app, patch(&id, r#"{"answers":{"XYZ":{"type":"no"}}}"#)).await;
        assert_eq!(st, StatusCode::BAD_REQUEST);
        assert_eq!(v["error"]["code"], "unknown_question");
    }

    #[tokio::test]
    async fn patch_corpo_invalido_e_400() {
        let (app, _) = app(false);
        let id = demo(&app, "telecom-3").await["id"]
            .as_str()
            .unwrap()
            .to_string();
        let (st, v) = send(app, patch(&id, r#"{"hack":1}"#)).await;
        assert_eq!(st, StatusCode::BAD_REQUEST);
        assert_eq!(v["error"]["code"], "invalid_body");
    }

    #[tokio::test]
    async fn patch_expirado_e_410_e_inexistente_e_404() {
        let (app, repo) = app(false);
        let raw = std::fs::read_to_string(fixtures().join("extracted/telecom-3.json")).unwrap();
        let mut a = new_analysis(serde_json::from_str(&raw).unwrap(), AnalysisSource::Demo);
        a.expires_at = (Utc::now() - Duration::days(1)).to_rfc3339();
        repo.insert(&a).await.unwrap();
        let (st, _) = send(app.clone(), patch(&a.id, r#"{"paid":true}"#)).await;
        assert_eq!(st, StatusCode::GONE);
        let (st, _) = send(app, patch(&Uuid::new_v4().to_string(), r#"{"paid":true}"#)).await;
        assert_eq!(st, StatusCode::NOT_FOUND);
    }

    fn dispute(id: &str, body: &str) -> Request<Body> {
        Request::post(format!("/analyses/{id}/dispute"))
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body.to_string()))
            .unwrap()
    }

    // RF11 + RF12 via API.
    #[tokio::test]
    async fn dispute_gera_texto_passos_e_prazos() {
        let (app, _) = app(false);
        let v = demo(&app, "telecom-2").await;
        let id = v["id"].as_str().unwrap();
        let ids: Vec<&str> = v["findings"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|f| f["severity"] == "indevido")
            .map(|f| f["id"].as_str().unwrap())
            .collect();
        let body = serde_json::json!({"finding_ids": ids, "protocol": "2026111012345"}).to_string();
        let (st, d) = send(app, dispute(id, &body)).await;
        assert_eq!(st, StatusCode::OK);
        let text = d["text"].as_str().unwrap();
        assert!(text.contains("Total contestado: R$ 6,00"));
        assert!(text.contains("protocolo 2026111012345"));
        assert_eq!(d["total_cents"], 600);
        assert_eq!(d["steps"][0]["title"], "SAC da operadora");
        assert_eq!(
            d["disclaimer"],
            "Conteúdo informativo, não é orientação jurídica."
        );
    }

    // RF17 "Roteiro com os achados": o roteiro numerado vem junto da contestação.
    #[tokio::test]
    async fn dispute_devolve_o_roteiro_de_ligacao() {
        let (app, _) = app(false);
        let v = demo(&app, "telecom-2").await;
        let id = v["id"].as_str().unwrap();
        let ids: Vec<&str> = v["findings"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|f| f["severity"] == "indevido")
            .map(|f| f["id"].as_str().unwrap())
            .collect();
        let body = serde_json::json!({ "finding_ids": ids }).to_string();
        let (st, d) = send(app, dispute(id, &body)).await;
        assert_eq!(st, StatusCode::OK);
        let titulos: Vec<&str> = d["call_script"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| p["title"].as_str().unwrap())
            .collect();
        assert_eq!(
            titulos,
            [
                "Apresente-se",
                "Diga o mês da fatura",
                "Cite o item 1 de 2",
                "Cite o item 2 de 2",
                "Peça o estorno",
                "Peça o número do protocolo",
            ]
        );
        assert!(
            d["call_script"][2]["say"]
                .as_str()
                .unwrap()
                .contains("R$ 3,60")
        );
        // A contestação continua no mesmo nível do JSON.
        assert!(
            d["text"]
                .as_str()
                .unwrap()
                .contains("Total contestado: R$ 6,00")
        );
    }

    // RF17 "Protocolo anotado aparece na contestação", com espaços nas pontas e hífen.
    #[tokio::test]
    async fn dispute_aceita_protocolo_com_hifen_e_espacos_nas_pontas() {
        let (app, _) = app(false);
        let id = demo(&app, "telecom-2").await["id"]
            .as_str()
            .unwrap()
            .to_string();
        let body = r#"{"finding_ids":["TEL-03:i2"],"protocol":"  2026-1110-12345 "}"#;
        let (st, d) = send(app, dispute(&id, body)).await;
        assert_eq!(st, StatusCode::OK);
        assert!(
            d["text"]
                .as_str()
                .unwrap()
                .contains("protocolo 2026-1110-12345")
        );
    }

    #[tokio::test]
    async fn dispute_protocolo_so_aceita_digitos_e_hifen_ate_30() {
        let (app, _) = app(false);
        let id = demo(&app, "telecom-2").await["id"]
            .as_str()
            .unwrap()
            .to_string();
        let longo = "9".repeat(31);
        for ruim in ["ABC123", "2026 111", "2026/111", "12.345", longo.as_str()] {
            let body =
                serde_json::json!({"finding_ids":["TEL-03:i2"],"protocol": ruim}).to_string();
            let (st, v) = send(app.clone(), dispute(&id, &body)).await;
            assert_eq!(st, StatusCode::BAD_REQUEST, "{ruim}");
            assert_eq!(v["error"]["code"], "invalid_protocol", "{ruim}");
            // A mensagem de erro nunca devolve o protocolo enviado.
            assert!(!v.to_string().contains(ruim), "{ruim}");
        }
    }

    #[tokio::test]
    async fn dispute_selecao_vazia_ou_invalida_e_400() {
        let (app, _) = app(false);
        let id = demo(&app, "telecom-2").await["id"]
            .as_str()
            .unwrap()
            .to_string();
        for (body, code) in [
            (r#"{"finding_ids":[]}"#, "empty_selection"),
            (r#"{"finding_ids":["XYZ:i9"]}"#, "unknown_finding"),
            (
                r#"{"finding_ids":["TEL-03:i2"],"protocol":"<script>"}"#,
                "invalid_protocol",
            ),
            (r#"{"nada":1}"#, "invalid_body"),
        ] {
            let (st, v) = send(app.clone(), dispute(&id, body)).await;
            assert_eq!(st, StatusCode::BAD_REQUEST, "{body}");
            assert_eq!(v["error"]["code"], code, "{body}");
        }
    }

    // RNF06: o 11º upload na mesma janela, do mesmo IP, recebe 429 em JSON.
    #[tokio::test]
    async fn upload_acima_do_limite_e_429() {
        let repo = Arc::new(MemRepo::default());
        let app = router(AppState {
            repo,
            extractor: Some(Arc::new(FakeExtractor) as Arc<dyn BillExtractor>),
            fixtures_dir: fixtures(),
            limits: Some(crate::limits::Limits::PRODUCTION),
        });
        let with_ip = |ip: &str| {
            let mut r = multipart(Some("true"), Some(PDF));
            r.headers_mut()
                .insert("x-forwarded-for", ip.parse().unwrap());
            r
        };
        for i in 0..10 {
            let (st, _) = send(app.clone(), with_ip("203.0.113.7")).await;
            assert_eq!(st, StatusCode::CREATED, "upload {i}");
        }
        let (st, v) = send(app.clone(), with_ip("203.0.113.7")).await;
        assert_eq!(st, StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(v["error"]["code"], "rate_limited");
        // Outro IP não é afetado.
        let (st, _) = send(app, with_ip("203.0.113.8")).await;
        assert_eq!(st, StatusCode::CREATED);
    }

    // RNF06 / P14: POST /comparisons entra no mesmo limite das demais escritas.
    #[tokio::test]
    async fn comparacoes_acima_do_limite_de_escritas_e_429() {
        let repo = Arc::new(MemRepo::default());
        // Mesma rajada da produção, mas sem recarga útil durante o teste: o 61º pedido
        // não depende da velocidade da máquina.
        let limits = crate::limits::Limits {
            write_refill: std::time::Duration::from_secs(3600),
            ..crate::limits::Limits::PRODUCTION
        };
        let app = router(AppState {
            repo,
            extractor: None,
            fixtures_dir: fixtures(),
            limits: Some(limits),
        });
        let comparar = |ip: &str| {
            Request::post("/comparisons")
                .header("content-type", "application/json")
                .header("x-forwarded-for", ip)
                .body(Body::from("{}"))
                .unwrap()
        };
        // O corpo é inválido de propósito: o limite vale antes da validação.
        for i in 0..limits.write_burst {
            let (st, _) = send(app.clone(), comparar("203.0.113.9")).await;
            assert_ne!(st, StatusCode::TOO_MANY_REQUESTS, "pedido {i}");
        }
        let (st, v) = send(app.clone(), comparar("203.0.113.9")).await;
        assert_eq!(st, StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(v["error"]["code"], "rate_limited");
        // Outro IP não é afetado.
        let (st, _) = send(app, comparar("203.0.113.10")).await;
        assert_ne!(st, StatusCode::TOO_MANY_REQUESTS);
    }

    // RF05: correção de valor reavalia sem IA.
    #[tokio::test]
    async fn patch_correcao_de_valor_reavalia() {
        let (app, _) = app(false);
        let v = demo(&app, "telecom-2").await;
        let id = v["id"].as_str().unwrap();
        let mut bill = v["bill"].clone();
        // Multa corrigida para R$ 2,40 (exatamente 2%): TEL-03 deixa de acusar.
        bill["late_fees"]["fine_cents"] = serde_json::json!(240);
        bill["items"][1]["amount_cents"] = serde_json::json!(240);
        bill["total_cents"] = serde_json::json!(12990 + 240 + 360);
        let body = serde_json::json!({ "bill": bill }).to_string();
        let (st, v2) = send(app, patch(id, &body)).await;
        assert_eq!(st, StatusCode::OK);
        assert_eq!(v2["bill"]["late_fees"]["fine_cents"], 240);
        assert!(
            !v2["findings"]
                .as_array()
                .unwrap()
                .iter()
                .any(|f| f["rule_id"] == "TEL-03")
        );
    }

    // RF05: soma divergente é aceita e vira o achado de consistência (TEL-07).
    #[tokio::test]
    async fn patch_correcao_com_soma_divergente_gera_tel07() {
        let (app, _) = app(false);
        let v = demo(&app, "telecom-3").await;
        let id = v["id"].as_str().unwrap();
        let mut bill = v["bill"].clone();
        bill["items"][0]["amount_cents"] = serde_json::json!(9_990);
        let (st, v2) = send(
            app,
            patch(id, &serde_json::json!({ "bill": bill }).to_string()),
        )
        .await;
        assert_eq!(st, StatusCode::OK);
        assert!(
            v2["findings"]
                .as_array()
                .unwrap()
                .iter()
                .any(|f| f["rule_id"] == "TEL-07")
        );
    }

    #[tokio::test]
    async fn patch_correcao_inconsistente_ou_troca_de_tipo_e_400() {
        let (app, _) = app(false);
        let v = demo(&app, "telecom-3").await;
        let id = v["id"].as_str().unwrap().to_string();
        let mut sem_itens = v["bill"].clone();
        sem_itens["items"] = serde_json::json!([]);
        let mut outro_tipo = v["bill"].clone();
        outro_tipo["kind"] = serde_json::json!("energy");
        for bill in [sem_itens, outro_tipo] {
            let (st, e) = send(
                app.clone(),
                patch(&id, &serde_json::json!({ "bill": bill }).to_string()),
            )
            .await;
            assert_eq!(st, StatusCode::BAD_REQUEST);
            assert_eq!(e["error"]["code"], "invalid_correction");
        }
    }

    /// Writer que acumula os logs em memória (para inspecionar no teste).
    #[derive(Clone, Default)]
    pub(super) struct LogBuf(pub(super) Arc<std::sync::Mutex<Vec<u8>>>);
    impl std::io::Write for LogBuf {
        fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(b);
            Ok(b.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    /// Captura de logs do processo inteiro. Um subscriber por thread (`set_default`) corria com o
    /// cache de interesse dos eventos quando muitos testes rodavam juntos, e o teste ficava
    /// intermitente. Um subscriber global, instalado uma vez, não tem essa corrida.
    pub(super) fn logs_do_processo() -> &'static LogBuf {
        static LOGS: std::sync::OnceLock<LogBuf> = std::sync::OnceLock::new();
        LOGS.get_or_init(|| {
            let buf = LogBuf::default();
            let w = buf.clone();
            let subscriber = tracing_subscriber::fmt()
                .json()
                .with_max_level(tracing::Level::TRACE)
                .with_writer(move || w.clone())
                .finish();
            tracing::subscriber::set_global_default(subscriber)
                .expect("nenhum outro teste instala subscriber global");
            buf
        })
    }

    // RNF13: os logs de upload, PATCH, correção e contestação não têm conteúdo da fatura.
    // Como o subscriber é do processo, os logs dos outros testes também entram na checagem.
    #[tokio::test]
    async fn logs_nao_contem_conteudo_da_fatura() {
        let buf = logs_do_processo();

        let (app, _) = app(true);
        let (_, v) = send(app.clone(), multipart(Some("true"), Some(PDF))).await;
        let id = v["id"].as_str().unwrap().to_string();
        send(
            app.clone(),
            patch(
                &id,
                r#"{"answers":{"TEL-01:i2":{"type":"no"}},"paid":true}"#,
            ),
        )
        .await;
        let ids: Vec<String> = v["findings"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| f["id"].as_str().unwrap().to_string())
            .collect();
        send(
            app.clone(),
            dispute(
                &id,
                &serde_json::json!({ "finding_ids": [ids[0]], "protocol": "2026111012345" })
                    .to_string(),
            ),
        )
        .await;
        let mut bill = v["bill"].clone();
        bill["items"][0]["amount_cents"] = serde_json::json!(9_000);
        send(
            app,
            patch(&id, &serde_json::json!({ "bill": bill }).to_string()),
        )
        .await;

        let logs = String::from_utf8_lossy(&buf.0.lock().unwrap()).into_owned();
        assert!(
            logs.contains(&id),
            "o id da análise deveria aparecer nos logs"
        );
        for proibido in [
            "Clube Vivaz",
            "CLUBE VZ",
            "Seguro Tela",
            "PLANO VZ",
            "12,90",
            "1290",
            "Vivaz Telecom",
            "Contestação",
            "Olá.",
            "2026111012345",
        ] {
            assert!(
                !logs.contains(proibido),
                "conteúdo da fatura vazou nos logs: {proibido}"
            );
        }
    }

    // RNF05: todo resultado nasce com validade de exatamente 30 dias.
    #[tokio::test]
    async fn resultado_expira_em_30_dias() {
        let (app, _) = app(false);
        let v = demo(&app, "telecom-1").await;
        let created =
            chrono::DateTime::parse_from_rfc3339(v["created_at"].as_str().unwrap()).unwrap();
        let expires =
            chrono::DateTime::parse_from_rfc3339(v["expires_at"].as_str().unwrap()).unwrap();
        assert_eq!(expires - created, Duration::days(RETENTION_DAYS));
        assert_eq!(RETENTION_DAYS, 30);
    }

    // RF01 "Visitante testa com exemplo de telecom": há pelo menos um achado suspeito.
    #[tokio::test]
    async fn demo_telecom_1_tem_achado_suspeito() {
        let (app, _) = app(false);
        let v = demo(&app, "telecom-1").await;
        assert!(
            v["findings"]
                .as_array()
                .unwrap()
                .iter()
                .any(|f| f["severity"] == "suspeito")
        );
    }

    // RF01 "Exemplo funciona mesmo com a IA indisponível": as 3 demos de energia sem extrator.
    #[tokio::test]
    async fn demos_de_energia_funcionam_sem_ia() {
        let (app, _) = app(false);
        for s in ["energia-1", "energia-2", "energia-3"] {
            let v = demo(&app, s).await;
            assert_eq!(v["bill"]["kind"], "energy", "{s}");
            assert!(!v["findings"].as_array().unwrap().is_empty(), "{s}");
        }
    }

    // #54: o extrato de exemplo abre sem IA, pergunta, reavalia e gera a contestação.
    #[tokio::test]
    async fn demo_de_extrato_bancario_funciona_de_ponta_a_ponta_sem_ia() {
        let (app, _) = app(false);
        let v = demo(&app, "extrato-1").await;
        assert_eq!(v["bill"]["kind"], "bank_statement");
        let regras: Vec<&str> = v["findings"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| f["rule_id"].as_str().unwrap())
            .collect();
        assert_eq!(regras, ["BAN-01", "BAN-03", "BAN-02"]);
        assert!(
            v["findings"]
                .as_array()
                .unwrap()
                .iter()
                .all(|f| f["severity"] == "suspeito")
        );
        let id = v["id"].as_str().unwrap().to_string();
        let (st, v) = send(
            app.clone(),
            patch(
                &id,
                r#"{"answers":{"BAN-01:i1":{"type":"no"},"BAN-02:saque":{"type":"no"}}}"#,
            ),
        )
        .await;
        assert_eq!(st, StatusCode::OK);
        assert_eq!(v["recoverable_cents"], 3990 + 650);
        let (st, d) = send(
            app,
            dispute(&id, r#"{"finding_ids":["BAN-01:i1","BAN-02:i3"]}"#),
        )
        .await;
        assert_eq!(st, StatusCode::OK);
        let texto = d["text"].as_str().unwrap();
        assert!(texto.contains("extrato da minha conta corrente"), "{texto}");
        assert!(texto.contains("Res. CMN 3.919/2010"), "{texto}");
        assert_eq!(d["steps"][2]["title"], "Banco Central");
    }

    // #55: a fatura do cartão de exemplo abre sem IA, com a multa acima de 2% já indevida.
    #[tokio::test]
    async fn demo_de_fatura_do_cartao_funciona_sem_ia() {
        let (app, _) = app(false);
        let v = demo(&app, "cartao-1").await;
        assert_eq!(v["bill"]["kind"], "card_bill");
        let achados: Vec<(&str, &str)> = v["findings"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| {
                (
                    f["rule_id"].as_str().unwrap(),
                    f["severity"].as_str().unwrap(),
                )
            })
            .collect();
        assert_eq!(
            achados,
            [
                ("CAR-04", "indevido"),
                ("CAR-01", "suspeito"),
                ("CAR-02", "suspeito")
            ]
        );
        assert_eq!(v["recoverable_cents"], 1700);
        let perguntas: Vec<&str> = v["questions"]
            .as_array()
            .unwrap()
            .iter()
            .map(|q| q["id"].as_str().unwrap())
            .collect();
        assert_eq!(perguntas, ["CAR-01:i3", "CAR-03:anuidade"]);
    }

    // RF14: resultado expirado responde 410.
    #[tokio::test]
    async fn analise_expirada_e_410() {
        let (app, repo) = app(false);
        let raw = std::fs::read_to_string(fixtures().join("extracted/telecom-3.json")).unwrap();
        let mut a = new_analysis(serde_json::from_str(&raw).unwrap(), AnalysisSource::Demo);
        a.expires_at = (Utc::now() - Duration::days(1)).to_rfc3339();
        repo.insert(&a).await.unwrap();
        let (st, v) = send(
            app,
            Request::get(format!("/analyses/{}", a.id))
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(st, StatusCode::GONE);
        assert_eq!(v["error"]["code"], "expired");
    }
}

#[cfg(test)]
mod edge_tests;
