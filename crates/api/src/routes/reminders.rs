//! RF18/RF19: lembrete opcional por e-mail no fim do prazo de resposta da empresa.
//!
//! Privacidade: o e-mail nunca aparece em log, em erro ou em resposta. Ele é
//! apagado ao enviar o aviso e ao cancelar, e some junto com a análise (30 dias).

use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Path, State, rejection::JsonRejection},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post},
};
use chrono::{DateTime, Duration, FixedOffset, Utc};
use humanobill_core::{ReminderCreated, ReminderRequest};
use serde_json::json;
use subtle::ConstantTimeEq;
use uuid::Uuid;

use crate::{
    error::ApiError,
    limits::Limits,
    mailer::{Mailer, Message},
    repo::{Reminder, Repo},
};

/// Prazo de resposta da empresa (docs/02, RF18).
const COMPANY_DEADLINE_DAYS: i64 = 30;
/// O aviso sai pelo menos 1 dia antes de a análise expirar: o link ainda abre.
const SAFETY_MARGIN_DAYS: i64 = 1;
/// Quantos lembretes a rotina diária envia por chamada.
const DISPATCH_BATCH: i64 = 200;
const EMAIL_MAX_LEN: usize = 254;

/// Estado só do lembrete. Fica fora de `AppState` para a API seguir subindo
/// (e as outras rotas funcionando) sem as variáveis do Resend.
#[derive(Clone)]
pub struct ReminderState {
    pub repo: Arc<dyn Repo>,
    /// `None` = sem `RESEND_API_KEY`/`RESEND_FROM`: criar lembrete responde 503.
    pub mailer: Option<Arc<dyn Mailer>>,
    /// `None` = sem `CRON_SECRET`: a rotina diária fica desligada (503).
    pub cron_secret: Option<String>,
    /// Endereço público do front, sem barra final.
    pub web_url: String,
}

pub fn router(state: ReminderState, limits: Option<Limits>) -> Router {
    let public = Router::new()
        .route("/reminders", post(create_reminder))
        .route("/reminders/{token}/cancel", get(cancel_reminder));
    let public = match limits {
        Some(l) => {
            let (layer, cfg) = crate::limits::layer(l.write_burst, l.write_refill);
            if tokio::runtime::Handle::try_current().is_ok() {
                crate::limits::spawn_cleanup(vec![cfg]);
            }
            public.layer(layer)
        }
        None => public,
    };
    Router::new()
        .route("/internal/reminders/dispatch", post(dispatch))
        .merge(public)
        .with_state(state)
}

/// Validação simples e estrita o bastante para não mandar e-mail a lixo.
pub(super) fn valid_email(email: &str) -> bool {
    if email.len() > EMAIL_MAX_LEN || email != email.trim() {
        return false;
    }
    let Some((local, domain)) = email.split_once('@') else {
        return false;
    };
    let local_ok = !local.is_empty()
        && local.len() <= 64
        && !local.starts_with('.')
        && !local.ends_with('.')
        && !local.contains("..")
        && local
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "!#$%&'*+/=?^_`{|}~.-".contains(c));
    let labels: Vec<&str> = domain.split('.').collect();
    let domain_ok = labels.len() >= 2
        && labels.iter().all(|l| {
            !l.is_empty()
                && l.len() <= 63
                && !l.starts_with('-')
                && !l.ends_with('-')
                && l.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
        })
        && labels.last().is_some_and(|tld| tld.len() >= 2);
    local_ok && domain_ok
}

/// Data do aviso: antes do fim do prazo da empresa e antes de a análise sumir.
pub(super) fn due_date(now: DateTime<Utc>, analysis_expires_at: DateTime<Utc>) -> DateTime<Utc> {
    let by_deadline = now + Duration::days(COMPANY_DEADLINE_DAYS);
    let by_retention = analysis_expires_at - Duration::days(SAFETY_MARGIN_DAYS);
    by_deadline.min(by_retention).max(now)
}

/// dd/mm/aaaa em horário de Brasília (UTC-3, sem horário de verão desde 2019).
fn br_date(t: DateTime<Utc>) -> String {
    let brt = FixedOffset::west_opt(3 * 3600).expect("offset válido");
    t.with_timezone(&brt).format("%d/%m/%Y").to_string()
}

fn new_token() -> String {
    // Dois UUID v4 (aleatórios) em hex: 244 bits, impossível de adivinhar.
    format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple())
}

fn unavailable() -> ApiError {
    ApiError::new(
        StatusCode::SERVICE_UNAVAILABLE,
        "reminders_unavailable",
        "O lembrete por e-mail não está disponível agora",
    )
}

fn confirmation_message(to: &str, web_url: &str, token: &str, due_at: DateTime<Utc>) -> Message {
    Message {
        to: to.to_string(),
        subject: "Lembrete agendado no HumanoBill".into(),
        text: format!(
            "Olá!\n\nVamos te avisar em {} sobre o fim do prazo de resposta da empresa.\n\n\
             Para cancelar e apagar seu e-mail agora, abra este link:\n{web_url}/lembrete/cancelar/{token}\n\n\
             Depois do aviso, seu e-mail também é apagado dos nossos registros.\n",
            br_date(due_at)
        ),
    }
}

fn reminder_message(to: &str, web_url: &str, analysis_id: Uuid) -> Message {
    Message {
        to: to.to_string(),
        subject: "O prazo de resposta da empresa chegou ao fim".into(),
        text: format!(
            "Olá!\n\nVocê pediu este lembrete no HumanoBill. O prazo da empresa para responder \
             à sua contestação chegou ao fim.\n\n\
             Seu resultado: {web_url}/r/{analysis_id}\n\n\
             Próximos passos:\n\
             1. Veja se a empresa respondeu e se corrigiu a cobrança.\n\
             2. Se não respondeu, registre a reclamação na Anatel (telefone 1331) para telefone e \
             internet, ou na ANEEL (telefone 167) para energia.\n\
             3. Você também pode abrir o caso em consumidor.gov.br.\n\n\
             Seu e-mail já foi apagado dos nossos registros e não vamos escrever de novo.\n\n\
             Conteúdo informativo, não é orientação jurídica.\n"
        ),
    }
}

/// RF18: agenda o aviso. Exige consentimento próprio, separado do envio da fatura.
async fn create_reminder(
    State(s): State<ReminderState>,
    body: Result<Json<ReminderRequest>, JsonRejection>,
) -> Result<impl IntoResponse, ApiError> {
    let mailer = s.mailer.clone().ok_or_else(unavailable)?;
    let Json(body) = body.map_err(|_| {
        ApiError::new(
            StatusCode::BAD_REQUEST,
            "invalid_body",
            "Informe o e-mail e marque o consentimento",
        )
    })?;
    if !body.consent {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "consent_required",
            "Marque o consentimento para receber o lembrete",
        ));
    }
    if !valid_email(&body.email) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "invalid_email",
            "Informe um e-mail válido",
        ));
    }
    let id = Uuid::parse_str(&body.analysis_id).map_err(|_| ApiError::not_found())?;
    let stored = s.repo.get(id).await?.ok_or_else(ApiError::not_found)?;
    let now = Utc::now();
    if stored.expires_at <= now {
        return Err(ApiError::new(
            StatusCode::GONE,
            "expired",
            "Este resultado expirou. Analise de novo",
        ));
    }
    let due_at = due_date(now, stored.expires_at);
    let reminder = Reminder {
        token: new_token(),
        analysis_id: id,
        email: body.email.clone(),
        due_at,
    };
    s.repo.upsert_reminder(&reminder).await?;
    // O e-mail de confirmação leva o link de cancelamento e já prova que o
    // endereço recebe mensagens. Se falhar, desfaz tudo e apaga o e-mail.
    let confirm = confirmation_message(&body.email, &s.web_url, &reminder.token, due_at);
    if let Err(e) = mailer.send(&confirm).await {
        tracing::warn!(analysis_id = %id, error = %e, "lembrete: confirmação não enviada");
        s.repo.delete_reminder(&reminder.token).await?;
        return Err(ApiError::new(
            StatusCode::BAD_GATEWAY,
            "email_failed",
            "Não consegui enviar o e-mail agora. Confira o endereço e tente de novo",
        ));
    }
    tracing::info!(analysis_id = %id, due_at = %due_at.to_rfc3339(), "lembrete agendado");
    Ok((
        StatusCode::CREATED,
        Json(ReminderCreated {
            due_at: due_at.to_rfc3339(),
        }),
    ))
}

/// RF19: apaga o lembrete e o e-mail. Token inválido e já cancelado têm a mesma resposta.
async fn cancel_reminder(
    State(s): State<ReminderState>,
    Path(token): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    if s.repo.delete_reminder(&token).await? {
        tracing::info!("lembrete cancelado");
        Ok(Json(
            json!({"message": "Lembrete cancelado e e-mail apagado"}),
        ))
    } else {
        Err(ApiError::new(
            StatusCode::NOT_FOUND,
            "not_found",
            "Este lembrete não existe ou já foi cancelado",
        ))
    }
}

/// Interna, chamada 1×/dia pelo GitHub Actions: envia os vencidos e apaga cada linha enviada.
async fn dispatch(
    State(s): State<ReminderState>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    // Segredo vazio ou só com espaços = não configurado: nunca compara.
    let secret = s
        .cron_secret
        .as_deref()
        .filter(|v| !v.trim().is_empty())
        .ok_or_else(unavailable)?;
    let sent_secret = headers
        .get("x-cron-secret")
        .map(|v| v.as_bytes())
        .unwrap_or_default();
    if !constant_time_eq(secret.as_bytes(), sent_secret) {
        return Err(ApiError::new(
            StatusCode::UNAUTHORIZED,
            "unauthorized",
            "Não autorizado",
        ));
    }
    let mailer = s.mailer.clone().ok_or_else(unavailable)?;
    let due = s.repo.due_reminders(Utc::now(), DISPATCH_BATCH).await?;
    let (mut sent, mut failed, mut dropped) = (0u32, 0u32, 0u32);
    for r in due {
        let msg = reminder_message(&r.email, &s.web_url, r.analysis_id);
        match mailer.send(&msg).await {
            Ok(()) => {
                // Apaga logo após enviar. Se a exclusão falhar, o próximo ciclo
                // reenvia: melhor repetir um aviso do que perder e guardar o e-mail.
                if let Err(e) = s.repo.delete_reminder(&r.token).await {
                    tracing::error!(analysis_id = %r.analysis_id, error = %e, "lembrete enviado, mas não apagado");
                }
                sent += 1;
            }
            // Recusa permanente (ex.: endereço inválido): repetir não adianta, então apaga
            // a linha e o e-mail agora, em vez de reter e tentar todo dia.
            Err(e) if e.is_permanent() => {
                if let Err(del) = s.repo.delete_reminder(&r.token).await {
                    tracing::error!(analysis_id = %r.analysis_id, error = %del, "lembrete recusado, mas não apagado");
                    failed += 1;
                } else {
                    tracing::warn!(analysis_id = %r.analysis_id, error = %e, "lembrete recusado e apagado");
                    dropped += 1;
                }
            }
            Err(e) => {
                // Falha transitória (rede, timeout, 408, 429, 5xx): mantém a linha
                // e tenta de novo na próxima rodada.
                tracing::warn!(analysis_id = %r.analysis_id, error = %e, "lembrete não enviado");
                failed += 1;
            }
        }
    }
    tracing::info!(sent, failed, dropped, "rotina de lembretes");
    Ok(Json(
        json!({"sent": sent, "failed": failed, "dropped": dropped}),
    ))
}

/// Compara sem encerrar cedo no primeiro byte diferente (evita ataque de tempo).
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    a.ct_eq(b).into()
}

#[cfg(test)]
mod tests {
    use axum::{body::Body, http::Request};
    use chrono::TimeZone;
    use http_body_util::BodyExt;
    use humanobill_core::{Analysis, AnalysisSource, ExtractedBill};
    use serde_json::Value;
    use std::path::PathBuf;
    use tower::ServiceExt;

    use super::super::new_analysis;
    use super::*;
    use crate::mailer::{FakeMailer, MailError};
    use crate::repo::MemRepo;

    const SEGREDO: &str = "segredo-do-cron";
    const EMAIL: &str = "maria@exemplo.com";

    struct Ctx {
        app: Router,
        repo: Arc<MemRepo>,
        mailer: Arc<FakeMailer>,
    }

    fn montar(mailer: Arc<FakeMailer>, com_mailer: bool, segredo: Option<&str>) -> Ctx {
        let repo = Arc::new(MemRepo::default());
        let m: Option<Arc<dyn Mailer>> = if com_mailer {
            Some(mailer.clone())
        } else {
            None
        };
        let app = router(
            ReminderState {
                repo: repo.clone(),
                mailer: m,
                cron_secret: segredo.map(String::from),
                web_url: "https://humanobill.exemplo".into(),
            },
            None,
        );
        Ctx { app, repo, mailer }
    }

    fn ctx() -> Ctx {
        montar(Arc::new(FakeMailer::default()), true, Some(SEGREDO))
    }

    fn bill() -> ExtractedBill {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/extracted/telecom-1.json");
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
    }

    async fn guardar(c: &Ctx) -> Analysis {
        let a = new_analysis(bill(), AnalysisSource::Demo);
        crate::repo::Repo::insert(&*c.repo, &a).await.unwrap();
        a
    }

    async fn enviar(c: &Ctx, req: Request<Body>) -> (StatusCode, Value, String) {
        let resp = c.app.clone().oneshot(req).await.unwrap();
        let status = resp.status();
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let raw = String::from_utf8_lossy(&bytes).to_string();
        (
            status,
            serde_json::from_str(&raw).unwrap_or(Value::Null),
            raw,
        )
    }

    async fn agendar(c: &Ctx, corpo: Value) -> (StatusCode, Value, String) {
        let req = Request::post("/reminders")
            .header("content-type", "application/json")
            .body(Body::from(corpo.to_string()))
            .unwrap();
        enviar(c, req).await
    }

    fn pedido(a: &Analysis, email: &str, consent: bool) -> Value {
        json!({"analysis_id": a.id, "email": email, "consent": consent})
    }

    async fn cancelar(c: &Ctx, token: &str) -> (StatusCode, Value, String) {
        enviar(
            c,
            Request::get(format!("/reminders/{token}/cancel"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
    }

    async fn rodar_rotina(c: &Ctx, segredo: Option<&str>) -> (StatusCode, Value, String) {
        let mut req = Request::post("/internal/reminders/dispatch");
        if let Some(s) = segredo {
            req = req.header("x-cron-secret", s);
        }
        enviar(c, req.body(Body::empty()).unwrap()).await
    }

    /// Faz o único lembrete guardado vencer hoje.
    fn vencer_hoje(c: &Ctx) {
        let mut all = c.repo.reminders();
        assert_eq!(all.len(), 1);
        let mut r = all.remove(0);
        r.due_at = Utc::now() - Duration::hours(1);
        let repo = c.repo.clone();
        // MemRepo troca pelo analysis_id.
        futures_lite_block(async move {
            crate::repo::Repo::upsert_reminder(&*repo, &r)
                .await
                .unwrap()
        });
    }

    fn futures_lite_block<F: std::future::Future>(f: F) -> F::Output {
        tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(f))
    }

    // ---- RF18 ----

    #[tokio::test(flavor = "multi_thread")]
    async fn lembrete_agendado_responde_a_data_e_guarda_o_pedido() {
        let c = ctx();
        let a = guardar(&c).await;
        let (st, v, _) = agendar(&c, pedido(&a, EMAIL, true)).await;
        assert_eq!(st, StatusCode::CREATED);
        let due = DateTime::parse_from_rfc3339(v["due_at"].as_str().unwrap()).unwrap();
        let esperado = Utc::now() + Duration::days(29);
        assert!((due.with_timezone(&Utc) - esperado).num_minutes().abs() < 5);
        let guardados = c.repo.reminders();
        assert_eq!(guardados.len(), 1);
        assert_eq!(guardados[0].email, EMAIL);
        assert_eq!(guardados[0].analysis_id.to_string(), a.id);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn confirmacao_leva_o_link_de_cancelamento_e_a_data() {
        let c = ctx();
        let a = guardar(&c).await;
        agendar(&c, pedido(&a, EMAIL, true)).await;
        let enviados = c.mailer.sent();
        assert_eq!(enviados.len(), 1);
        let token = &c.repo.reminders()[0].token;
        assert_eq!(enviados[0].to, EMAIL);
        assert!(enviados[0].text.contains(&format!(
            "https://humanobill.exemplo/lembrete/cancelar/{token}"
        )));
        assert!(enviados[0].text.contains("Vamos te avisar em"));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn envio_do_lembrete_manda_o_link_e_apaga_o_email() {
        let c = ctx();
        let a = guardar(&c).await;
        agendar(&c, pedido(&a, EMAIL, true)).await;
        vencer_hoje(&c);
        let (st, v, _) = rodar_rotina(&c, Some(SEGREDO)).await;
        assert_eq!(st, StatusCode::OK);
        assert_eq!(v, json!({"sent": 1, "failed": 0, "dropped": 0}));
        let enviados = c.mailer.sent();
        assert_eq!(enviados.len(), 2); // confirmação + aviso
        let aviso = &enviados[1];
        assert_eq!(aviso.to, EMAIL);
        assert!(
            aviso
                .text
                .contains(&format!("https://humanobill.exemplo/r/{}", a.id))
        );
        assert!(aviso.text.contains("Próximos passos"));
        assert!(c.repo.reminders().is_empty(), "e-mail precisa ser apagado");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn rotina_ignora_lembrete_que_ainda_nao_venceu() {
        let c = ctx();
        let a = guardar(&c).await;
        agendar(&c, pedido(&a, EMAIL, true)).await;
        let (_, v, _) = rodar_rotina(&c, Some(SEGREDO)).await;
        assert_eq!(v, json!({"sent": 0, "failed": 0, "dropped": 0}));
        assert_eq!(c.mailer.sent().len(), 1); // só a confirmação
        assert_eq!(c.repo.reminders().len(), 1);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn email_invalido_mostra_a_mensagem_e_nao_guarda() {
        let c = ctx();
        let a = guardar(&c).await;
        let (st, v, _) = agendar(&c, pedido(&a, "maria@", true)).await;
        assert_eq!(st, StatusCode::BAD_REQUEST);
        assert_eq!(v["error"]["code"], "invalid_email");
        assert_eq!(v["error"]["message"], "Informe um e-mail válido");
        assert!(c.repo.reminders().is_empty() && c.mailer.sent().is_empty());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn sem_consentimento_recusa_e_nao_guarda() {
        let c = ctx();
        let a = guardar(&c).await;
        let (st, v, _) = agendar(&c, pedido(&a, EMAIL, false)).await;
        assert_eq!(st, StatusCode::BAD_REQUEST);
        assert_eq!(v["error"]["code"], "consent_required");
        let (st, v, _) = agendar(&c, json!({"analysis_id": a.id, "email": EMAIL})).await;
        assert_eq!(st, StatusCode::BAD_REQUEST);
        assert_eq!(v["error"]["code"], "invalid_body");
        assert!(c.repo.reminders().is_empty() && c.mailer.sent().is_empty());
    }

    // ---- RF19 ----

    #[tokio::test(flavor = "multi_thread")]
    async fn cancelamento_pelo_link_apaga_o_registro() {
        let c = ctx();
        let a = guardar(&c).await;
        agendar(&c, pedido(&a, EMAIL, true)).await;
        let token = c.repo.reminders()[0].token.clone();
        let (st, v, _) = cancelar(&c, &token).await;
        assert_eq!(st, StatusCode::OK);
        assert_eq!(v["message"], "Lembrete cancelado e e-mail apagado");
        assert!(c.repo.reminders().is_empty());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn token_invalido_e_token_ja_cancelado_tem_a_mesma_resposta() {
        let c = ctx();
        let a = guardar(&c).await;
        agendar(&c, pedido(&a, EMAIL, true)).await;
        let token = c.repo.reminders()[0].token.clone();
        cancelar(&c, &token).await;
        let (st1, v1, raw1) = cancelar(&c, &token).await; // já cancelado
        let (st2, v2, raw2) = cancelar(&c, "nao-existe").await; // inexistente
        assert_eq!(st1, StatusCode::NOT_FOUND);
        assert_eq!((st1, &v1, &raw1), (st2, &v2, &raw2));
        assert_eq!(
            v1["error"]["message"],
            "Este lembrete não existe ou já foi cancelado"
        );
    }

    // ---- segurança da rotina diária ----

    #[tokio::test(flavor = "multi_thread")]
    async fn rotina_exige_o_segredo_certo() {
        let c = ctx();
        let a = guardar(&c).await;
        agendar(&c, pedido(&a, EMAIL, true)).await;
        vencer_hoje(&c);
        for s in [
            None,
            Some(""),
            Some("errado"),
            Some("segredo-do-cron "),
            Some("segredo-do-cro"),
        ] {
            let (st, _, _) = rodar_rotina(&c, s).await;
            assert_eq!(st, StatusCode::UNAUTHORIZED, "{s:?}");
        }
        assert_eq!(c.mailer.sent().len(), 1, "nada enviado sem o segredo");
        assert_eq!(c.repo.reminders().len(), 1);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn rotina_fica_desligada_sem_cron_secret() {
        let c = montar(Arc::new(FakeMailer::default()), true, None);
        // Mesmo mandando um cabeçalho vazio, não passa.
        let (st, _, _) = rodar_rotina(&c, Some("")).await;
        assert_eq!(st, StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn segredo_configurado_vazio_nunca_autoriza() {
        for configurado in ["", "   ", "\t"] {
            let c = montar(Arc::new(FakeMailer::default()), true, Some(configurado));
            let a = guardar(&c).await;
            agendar(&c, pedido(&a, EMAIL, true)).await;
            vencer_hoje(&c);
            // Sem cabeçalho, vazio ou igual ao segredo configurado: nunca 200.
            for enviado in [None, Some(""), Some(configurado)] {
                let (st, _, _) = rodar_rotina(&c, enviado).await;
                assert!(
                    st == StatusCode::SERVICE_UNAVAILABLE || st == StatusCode::UNAUTHORIZED,
                    "{configurado:?} / {enviado:?} => {st}"
                );
            }
            assert_eq!(c.mailer.sent().len(), 1, "nada enviado além da confirmação");
            assert_eq!(c.repo.reminders().len(), 1);
        }
    }

    #[test]
    fn comparacao_em_tempo_constante() {
        assert!(constant_time_eq(b"abc", b"abc"));
        assert!(!constant_time_eq(b"abc", b"abd"));
        assert!(!constant_time_eq(b"abc", b"abcd"));
        assert!(!constant_time_eq(b"abc", b""));
    }

    // ---- configuração e falhas ----

    #[tokio::test(flavor = "multi_thread")]
    async fn sem_servico_de_email_o_erro_e_claro_e_a_api_segue() {
        let c = montar(Arc::new(FakeMailer::default()), false, Some(SEGREDO));
        let a = guardar(&c).await;
        let (st, v, _) = agendar(&c, pedido(&a, EMAIL, true)).await;
        assert_eq!(st, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(v["error"]["code"], "reminders_unavailable");
        assert!(c.repo.reminders().is_empty());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn falha_na_confirmacao_desfaz_e_nao_guarda_o_email() {
        let c = montar(Arc::new(FakeMailer::failing()), true, Some(SEGREDO));
        let a = guardar(&c).await;
        let (st, v, raw) = agendar(&c, pedido(&a, EMAIL, true)).await;
        assert_eq!(st, StatusCode::BAD_GATEWAY);
        assert_eq!(v["error"]["code"], "email_failed");
        assert!(!raw.contains(EMAIL));
        assert!(c.repo.reminders().is_empty());
    }

    /// Agenda com o mailer bom, vence o lembrete e roda a rotina com um mailer que falha com `erro`.
    async fn rotina_com_falha(erro: MailError) -> (Ctx, StatusCode, Value, String) {
        let c = ctx();
        let a = guardar(&c).await;
        agendar(&c, pedido(&a, EMAIL, true)).await;
        vencer_hoje(&c);
        let c2 = Ctx {
            app: router(
                ReminderState {
                    repo: c.repo.clone(),
                    mailer: Some(Arc::new(FakeMailer::failing_with(erro))),
                    cron_secret: Some(SEGREDO.into()),
                    web_url: "https://humanobill.exemplo".into(),
                },
                None,
            ),
            repo: c.repo.clone(),
            mailer: c.mailer.clone(),
        };
        let (st, v, raw) = rodar_rotina(&c2, Some(SEGREDO)).await;
        (c, st, v, raw)
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn falha_transitoria_no_envio_mantem_a_linha_para_a_proxima_rodada() {
        for erro in [
            MailError::Unavailable,
            MailError::Rejected(408),
            MailError::Rejected(429),
            MailError::Rejected(500),
            MailError::Rejected(503),
        ] {
            let (c, st, v, _) = rotina_com_falha(erro.clone()).await;
            assert_eq!(st, StatusCode::OK, "{erro:?}");
            assert_eq!(v, json!({"sent": 0, "failed": 1, "dropped": 0}), "{erro:?}");
            assert_eq!(c.repo.reminders().len(), 1, "{erro:?}");
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn recusa_permanente_apaga_a_linha_e_o_email_em_vez_de_retentar() {
        for status in [400, 403, 422] {
            let (c, st, v, raw) = rotina_com_falha(MailError::Rejected(status)).await;
            assert_eq!(st, StatusCode::OK, "{status}");
            assert_eq!(v, json!({"sent": 0, "failed": 0, "dropped": 1}), "{status}");
            assert!(
                c.repo.reminders().is_empty(),
                "e-mail precisa ser apagado ({status})"
            );
            assert!(!raw.contains("maria@"), "{raw}");
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn analise_inexistente_e_expirada() {
        let c = ctx();
        let outra = new_analysis(bill(), AnalysisSource::Demo);
        let (st, _, _) = agendar(&c, pedido(&outra, EMAIL, true)).await;
        assert_eq!(st, StatusCode::NOT_FOUND);
        let (st, _, _) = agendar(
            &c,
            json!({"analysis_id": "lixo", "email": EMAIL, "consent": true}),
        )
        .await;
        assert_eq!(st, StatusCode::NOT_FOUND);

        let mut velha = new_analysis(bill(), AnalysisSource::Demo);
        velha.expires_at = (Utc::now() - Duration::days(1)).to_rfc3339();
        crate::repo::Repo::insert(&*c.repo, &velha).await.unwrap();
        let (st, v, _) = agendar(&c, pedido(&velha, EMAIL, true)).await;
        assert_eq!(st, StatusCode::GONE);
        assert_eq!(v["error"]["code"], "expired");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn novo_pedido_na_mesma_analise_troca_o_lembrete() {
        let c = ctx();
        let a = guardar(&c).await;
        agendar(&c, pedido(&a, EMAIL, true)).await;
        let antigo = c.repo.reminders()[0].token.clone();
        agendar(&c, pedido(&a, "joao@exemplo.com", true)).await;
        let todos = c.repo.reminders();
        assert_eq!(todos.len(), 1);
        assert_eq!(todos[0].email, "joao@exemplo.com");
        let (st, _, _) = cancelar(&c, &antigo).await;
        assert_eq!(st, StatusCode::NOT_FOUND);
    }

    // ---- privacidade ----

    #[tokio::test(flavor = "multi_thread")]
    async fn respostas_nunca_trazem_o_email() {
        let c = ctx();
        let a = guardar(&c).await;
        let (_, _, ok) = agendar(&c, pedido(&a, EMAIL, true)).await;
        let (_, _, ruim) = agendar(&c, pedido(&a, "maria@", true)).await;
        let (_, _, sem_consent) = agendar(&c, pedido(&a, EMAIL, false)).await;
        let token = c.repo.reminders()[0].token.clone();
        let (_, _, canc) = cancelar(&c, &token).await;
        vencer_hoje_se_existir(&c);
        let (_, _, rotina) = rodar_rotina(&c, Some(SEGREDO)).await;
        for raw in [ok, ruim, sem_consent, canc, rotina] {
            assert!(!raw.contains("maria@"), "{raw}");
        }
    }

    fn vencer_hoje_se_existir(c: &Ctx) {
        if !c.repo.reminders().is_empty() {
            vencer_hoje(c);
        }
    }

    #[test]
    fn debug_do_lembrete_e_do_pedido_nao_mostra_o_email() {
        let r = Reminder {
            token: "tok-secreto".into(),
            analysis_id: Uuid::nil(),
            email: EMAIL.into(),
            due_at: Utc::now(),
        };
        let d = format!("{r:?}");
        assert!(!d.contains("maria") && !d.contains("tok-secreto"), "{d}");
        let p = ReminderRequest {
            analysis_id: "x".into(),
            email: EMAIL.into(),
            consent: true,
        };
        assert!(!format!("{p:?}").contains("maria"));
    }

    // ---- regras puras ----

    #[test]
    fn email_valido_e_invalido() {
        for ok in [
            "maria@exemplo.com",
            "a.b+c@sub.exemplo.com.br",
            "x_y@a-b.co",
        ] {
            assert!(valid_email(ok), "{ok}");
        }
        for ruim in [
            "",
            "maria@",
            "@exemplo.com",
            "maria",
            "maria@exemplo",
            "maria@exemplo.",
            "maria@.com",
            "ma ria@exemplo.com",
            " maria@exemplo.com",
            "maria@exemplo.com\n",
            "maria@@exemplo.com",
            "maria..x@exemplo.com",
            ".maria@exemplo.com",
            "maria@-exemplo.com",
            "maria@exemplo.c",
            "maria@exemplo.com, outro@exemplo.com",
            "<maria@exemplo.com>",
        ] {
            assert!(!valid_email(ruim), "{ruim:?}");
        }
        assert!(!valid_email(&format!("{}@exemplo.com", "a".repeat(65))));
        assert!(!valid_email(&format!("a@{}.com", "b".repeat(250))));
    }

    #[test]
    fn data_do_aviso_respeita_o_prazo_e_a_retencao() {
        let t0 = Utc.with_ymd_and_hms(2026, 11, 10, 12, 0, 0).unwrap();
        // Análise nova: expira em 30 dias; avisa 1 dia antes (nunca depois de sumir).
        assert_eq!(
            due_date(t0, t0 + Duration::days(30)),
            t0 + Duration::days(29)
        );
        // Análise com muito tempo de sobra: vale o prazo da empresa (30 dias).
        assert_eq!(
            due_date(t0, t0 + Duration::days(90)),
            t0 + Duration::days(30)
        );
        // Análise quase expirando: nunca no passado.
        assert_eq!(due_date(t0, t0 + Duration::hours(5)), t0);
    }

    #[test]
    fn data_em_formato_brasileiro_no_fuso_de_brasilia() {
        let t = Utc.with_ymd_and_hms(2026, 12, 10, 1, 0, 0).unwrap(); // 22h do dia 9 em BRT
        assert_eq!(br_date(t), "09/12/2026");
        let t = Utc.with_ymd_and_hms(2026, 12, 10, 15, 0, 0).unwrap();
        assert_eq!(br_date(t), "10/12/2026");
    }

    #[test]
    fn tokens_sao_longos_e_unicos() {
        let (a, b) = (new_token(), new_token());
        assert_eq!(a.len(), 64);
        assert_ne!(a, b);
    }
}
