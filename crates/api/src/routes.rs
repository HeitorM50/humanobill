//! Rotas HTTP (docs/04 — API).

use std::{path::PathBuf, sync::Arc};

use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Multipart, Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use chrono::{Duration, Utc};
use humanobill_core::{Analysis, AnalysisSource, Answers, ExtractedBill, validate_bill};
use humanobill_extract::BillExtractor;
use serde_json::json;
use uuid::Uuid;

use crate::{error::ApiError, repo::Repo, upload};

pub const RETENTION_DAYS: i64 = 30;

#[derive(Clone)]
pub struct AppState {
    pub repo: Arc<dyn Repo>,
    pub extractor: Option<Arc<dyn BillExtractor>>,
    pub fixtures_dir: PathBuf,
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route(
            "/analyses",
            post(create_upload).layer(DefaultBodyLimit::max(upload::MAX_BYTES + 64 * 1024)),
        )
        .route("/analyses/demo/{sample}", post(create_demo))
        .route("/analyses/{id}", get(get_analysis))
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

fn new_analysis(bill: ExtractedBill, source: AnalysisSource) -> Analysis {
    let answers = Answers::new();
    let eval = humanobill_rules::evaluate(&bill, &answers, &humanobill_rules::Context::default());
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

/// RF01: cria uma análise a partir de uma fatura de exemplo pré-extraída (sem IA).
async fn create_demo(
    State(s): State<AppState>,
    Path(sample): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
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
    let path = s
        .fixtures_dir
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
    let analysis = new_analysis(bill, AnalysisSource::Demo);
    s.repo.insert(&analysis).await?;
    tracing::info!(id = %analysis.id, %sample, "análise demo criada");
    Ok((StatusCode::CREATED, Json(analysis)))
}

/// RF02 + RF03: upload de fatura real com consentimento obrigatório.
/// O arquivo existe só em memória durante a requisição (RNF04).
async fn create_upload(
    State(s): State<AppState>,
    mut form: Multipart,
) -> Result<impl IntoResponse, ApiError> {
    let mut file: Option<Vec<u8>> = None;
    let mut consent = false;
    while let Some(field) = form
        .next_field()
        .await
        .map_err(|_| ApiError::new(StatusCode::BAD_REQUEST, "invalid_file", "Envio inválido"))?
    {
        match field.name() {
            Some("file") => {
                let bytes = field.bytes().await.map_err(|_| {
                    ApiError::new(
                        StatusCode::PAYLOAD_TOO_LARGE,
                        "too_large",
                        "O arquivo deve ter até 10 MB",
                    )
                })?;
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
    let mime = upload::validate_file(&bytes)?;
    let extractor = s.extractor.as_ref().ok_or_else(|| {
        ApiError::from(humanobill_extract::ExtractError::Unavailable(
            "sem chave".into(),
        ))
    })?;

    let started = std::time::Instant::now();
    let result = extractor.extract(&bytes, mime).await;
    drop(bytes); // descarta o arquivo assim que a IA responde
    let bill = result.map_err(|e| {
        tracing::warn!(error = %e, "extração falhou");
        ApiError::from(e)
    })?;

    let analysis = new_analysis(bill, AnalysisSource::Upload);
    s.repo.insert(&analysis).await?;
    tracing::info!(id = %analysis.id, ms = started.elapsed().as_millis() as u64, "análise criada");
    Ok((StatusCode::CREATED, Json(analysis)))
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
