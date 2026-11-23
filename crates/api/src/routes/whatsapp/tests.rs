//! Testes do bot do WhatsApp e do simulador (#58).
//!
//! Os payloads de exemplo seguem o formato da Cloud API da Meta (entry[].changes[].value).

use axum::{
    body::Body,
    http::{Request, header},
};
use http_body_util::BodyExt;
use humanobill_core::{AnalysisSource, ExtractedBill};
use humanobill_extract::{ExtractError, InputMime};
use serde_json::{Value, json};
use tower::ServiceExt;
use uuid::Uuid;

use super::*;
use crate::repo::MemRepo;
use crate::routes::{new_analysis, tests::logs_do_processo};

fn fixtures() -> PathBuf {
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures"))
}

fn bill(sample: &str) -> ExtractedBill {
    let raw = std::fs::read_to_string(fixtures().join(format!("extracted/{sample}.json"))).unwrap();
    serde_json::from_str(&raw).unwrap()
}

fn analysis(sample: &str) -> Analysis {
    new_analysis(bill(sample), AnalysisSource::Demo)
}

const WEB: &str = "https://humanobill.vercel.app";
const APP_SECRET: &str = "segredo-do-app";
const VERIFY: &str = "token-combinado";
const PHONE_ID: &str = "109876543210";
/// Número fictício do remetente (nunca pode aparecer em log).
const FROM: &str = "5511987654321";
const PDF: &[u8] = b"%PDF-1.7\n1 0 obj << /Type /Page >> endobj\n";

fn config() -> WhatsappConfig {
    WhatsappConfig {
        verify_token: crate::config::Secret(VERIFY.into()),
        app_secret: crate::config::Secret(APP_SECRET.into()),
        token: crate::config::Secret("token-da-graph".into()),
        phone_id: PHONE_ID.into(),
        graph_url: "http://graph.invalido/v1".into(),
    }
}

// ---------------------------------------------------------------------------
// Mocks
// ---------------------------------------------------------------------------

struct FakeExtractor(Result<(), ExtractError>);
#[async_trait]
impl BillExtractor for FakeExtractor {
    async fn extract(&self, _: &[u8], _: InputMime) -> Result<ExtractedBill, ExtractError> {
        match &self.0 {
            Ok(()) => Ok(bill("telecom-1")),
            Err(ExtractError::RateLimited) => Err(ExtractError::RateLimited),
            Err(_) => Err(ExtractError::Provider("falha".into())),
        }
    }
}

/// Graph de mentira: devolve a mídia combinada e guarda o que o bot enviou.
#[derive(Default)]
struct FakeGraph {
    media: Mutex<HashMap<String, Result<Vec<u8>, GraphError>>>,
    sent: Mutex<Vec<(String, String)>>,
}

impl FakeGraph {
    fn with_media(id: &str, media: Result<Vec<u8>, GraphError>) -> Arc<Self> {
        let g = Self::default();
        g.media.lock().unwrap().insert(id.into(), media);
        Arc::new(g)
    }
    fn sent(&self) -> Vec<(String, String)> {
        self.sent.lock().unwrap().clone()
    }
}

#[async_trait]
impl GraphClient for FakeGraph {
    async fn download_media(&self, id: &str, _: usize) -> Result<Vec<u8>, GraphError> {
        self.media
            .lock()
            .unwrap()
            .get(id)
            .cloned()
            .unwrap_or(Err(GraphError::Failed))
    }
    async fn send_text(&self, to: &str, body: &str) -> Result<(), GraphError> {
        self.sent.lock().unwrap().push((to.into(), body.into()));
        Ok(())
    }
}

/// Repositório em memória que conta as gravações (o `MemRepo` não expõe isso).
#[derive(Default)]
struct CountRepo {
    inner: MemRepo,
    inserts: std::sync::atomic::AtomicUsize,
}

impl CountRepo {
    fn count(&self) -> usize {
        self.inserts.load(std::sync::atomic::Ordering::SeqCst)
    }
}

#[async_trait]
impl Repo for CountRepo {
    async fn ping(&self) -> Result<(), crate::repo::RepoError> {
        self.inner.ping().await
    }
    async fn insert(&self, a: &Analysis) -> Result<(), crate::repo::RepoError> {
        self.inserts
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        self.inner.insert(a).await
    }
    async fn get(&self, id: Uuid) -> Result<Option<crate::repo::Stored>, crate::repo::RepoError> {
        self.inner.get(id).await
    }
    async fn update(&self, a: &Analysis) -> Result<(), crate::repo::RepoError> {
        self.inner.update(a).await
    }
}

struct Harness {
    app: Router,
    repo: Arc<CountRepo>,
    graph: Arc<FakeGraph>,
}

fn harness(
    enabled: bool,
    extractor: Option<Result<(), ExtractError>>,
    graph: Arc<FakeGraph>,
    limits: Option<Limits>,
) -> Harness {
    let repo = Arc::new(CountRepo::default());
    let mut state = WhatsappState::new(
        repo.clone(),
        extractor.map(|e| Arc::new(FakeExtractor(e)) as Arc<dyn BillExtractor>),
        fixtures(),
        WEB.into(),
        enabled.then(config),
    );
    state.background = false;
    if enabled {
        state.graph = Some(graph.clone());
    }
    Harness {
        app: router(state, limits),
        repo,
        graph,
    }
}

fn default_harness() -> Harness {
    harness(
        true,
        Some(Ok(())),
        FakeGraph::with_media("MEDIA1", Ok(PDF.to_vec())),
        None,
    )
}

async fn call(app: &Router, req: Request<Body>) -> (StatusCode, String) {
    let resp = app.clone().oneshot(req).await.unwrap();
    let st = resp.status();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    (st, String::from_utf8_lossy(&bytes).into_owned())
}

fn json_of(body: &str) -> Value {
    serde_json::from_str(body).unwrap_or(Value::Null)
}

// ---------------------------------------------------------------------------
// Payloads no formato da Cloud API
// ---------------------------------------------------------------------------

fn envelope(message: Value) -> Value {
    json!({
        "object": "whatsapp_business_account",
        "entry": [{
            "id": "100200300",
            "changes": [{
                "field": "messages",
                "value": {
                    "messaging_product": "whatsapp",
                    "metadata": {
                        "display_phone_number": "15550001111",
                        "phone_number_id": PHONE_ID
                    },
                    "contacts": [{ "profile": { "name": "Fulana de Tal" }, "wa_id": FROM }],
                    "messages": [message]
                }
            }]
        }]
    })
}

fn text_msg(id: &str, body: &str) -> Value {
    envelope(json!({
        "from": FROM, "id": id, "timestamp": "1760000000",
        "type": "text", "text": { "body": body }
    }))
}

fn image_msg(id: &str, media_id: &str) -> Value {
    envelope(json!({
        "from": FROM, "id": id, "timestamp": "1760000000", "type": "image",
        "image": { "mime_type": "image/jpeg", "sha256": "abc", "id": media_id, "caption": "minha conta" }
    }))
}

fn document_msg(id: &str, media_id: &str) -> Value {
    envelope(json!({
        "from": FROM, "id": id, "timestamp": "1760000000", "type": "document",
        "document": { "mime_type": "application/pdf", "sha256": "abc", "id": media_id, "filename": "fatura.pdf" }
    }))
}

fn status_update() -> Value {
    json!({
        "object": "whatsapp_business_account",
        "entry": [{ "id": "1", "changes": [{ "field": "messages", "value": {
            "messaging_product": "whatsapp",
            "metadata": { "phone_number_id": PHONE_ID },
            "statuses": [{ "id": "wamid.X", "status": "delivered", "recipient_id": FROM }]
        }}]}]
    })
}

fn sign(secret: &str, body: &[u8]) -> String {
    let mut mac = <Hmac<Sha256> as KeyInit>::new_from_slice(secret.as_bytes()).unwrap();
    mac.update(body);
    let hex: String = mac
        .finalize()
        .into_bytes()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    format!("sha256={hex}")
}

fn post_webhook(body: &Value, signature: Option<String>) -> Request<Body> {
    let raw = body.to_string();
    let mut req =
        Request::post("/whatsapp/webhook").header(header::CONTENT_TYPE, "application/json");
    if let Some(sig) = signature {
        req = req.header("X-Hub-Signature-256", sig);
    }
    req.body(Body::from(raw)).unwrap()
}

fn signed(body: &Value) -> Request<Body> {
    let sig = sign(APP_SECRET, body.to_string().as_bytes());
    post_webhook(body, Some(sig))
}

// ---------------------------------------------------------------------------
// Verificação do webhook (GET)
// ---------------------------------------------------------------------------

fn verify_req(query: &str) -> Request<Body> {
    Request::get(format!("/whatsapp/webhook?{query}"))
        .body(Body::empty())
        .unwrap()
}

#[tokio::test]
async fn verificacao_devolve_o_desafio_com_o_token_certo() {
    let h = default_harness();
    let (st, body) = call(
        &h.app,
        verify_req(&format!(
            "hub.mode=subscribe&hub.verify_token={VERIFY}&hub.challenge=1158201444"
        )),
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(body, "1158201444");
}

#[tokio::test]
async fn verificacao_recusa_token_errado_modo_errado_e_campos_ausentes() {
    let h = default_harness();
    for q in [
        "hub.mode=subscribe&hub.verify_token=errado&hub.challenge=123".to_string(),
        format!("hub.mode=unsubscribe&hub.verify_token={VERIFY}&hub.challenge=123"),
        format!("hub.verify_token={VERIFY}&hub.challenge=123"),
        "hub.mode=subscribe&hub.challenge=123".to_string(),
        String::new(),
    ] {
        let (st, _) = call(&h.app, verify_req(&q)).await;
        assert_eq!(st, StatusCode::FORBIDDEN, "{q}");
    }
}

#[tokio::test]
async fn verificacao_nao_reflete_desafio_com_caracteres_estranhos() {
    let h = default_harness();
    let (st, body) = call(
        &h.app,
        verify_req(&format!(
            "hub.mode=subscribe&hub.verify_token={VERIFY}&hub.challenge=%3Cscript%3E"
        )),
    )
    .await;
    assert_eq!(st, StatusCode::BAD_REQUEST);
    assert!(!body.contains("script"));
}

#[tokio::test]
async fn webhook_desligado_responde_404_no_get_e_no_post() {
    let h = harness(false, Some(Ok(())), Arc::new(FakeGraph::default()), None);
    let (st, _) = call(
        &h.app,
        verify_req(&format!(
            "hub.mode=subscribe&hub.verify_token={VERIFY}&hub.challenge=1"
        )),
    )
    .await;
    assert_eq!(st, StatusCode::NOT_FOUND);
    // Mesmo com assinatura certa: desligado é desligado.
    let body = text_msg("wamid.1", "oi");
    let (st, _) = call(&h.app, signed(&body)).await;
    assert_eq!(st, StatusCode::NOT_FOUND);
}

// ---------------------------------------------------------------------------
// Assinatura (POST)
// ---------------------------------------------------------------------------

#[test]
fn assinatura_confere_so_com_o_segredo_e_o_corpo_certos() {
    let corpo = br#"{"a":1}"#;
    let ok = sign("s3gredo", corpo);
    assert!(verify_signature("s3gredo", Some(&ok), corpo));
    assert!(!verify_signature("outro", Some(&ok), corpo));
    assert!(!verify_signature("s3gredo", Some(&ok), br#"{"a":2}"#));
    assert!(!verify_signature("s3gredo", None, corpo));
    for ruim in [
        "",
        "sha256=",
        "sha256=zz",
        "sha256=abc",
        "sha1=0000",
        &ok[7..], // sem o prefixo
        &format!("{ok}00"),
    ] {
        assert!(!verify_signature("s3gredo", Some(ruim), corpo), "{ruim}");
    }
    // Hex em maiúsculas também é hex válido.
    assert!(verify_signature(
        "s3gredo",
        Some(&format!("sha256={}", ok[7..].to_uppercase())),
        corpo
    ));
}

#[tokio::test]
async fn post_com_assinatura_valida_e_aceito() {
    let h = default_harness();
    let (st, _) = call(&h.app, signed(&status_update())).await;
    assert_eq!(st, StatusCode::OK);
}

#[tokio::test]
async fn post_com_assinatura_invalida_ausente_ou_malformada_e_401_e_nada_acontece() {
    let h = default_harness();
    let body = text_msg("wamid.2", "EXEMPLO");
    for sig in [
        Some(sign("segredo-errado", body.to_string().as_bytes())),
        Some("sha256=00".into()),
        Some("lixo".into()),
        None,
    ] {
        let (st, resp) = call(&h.app, post_webhook(&body, sig)).await;
        assert_eq!(st, StatusCode::UNAUTHORIZED);
        assert_eq!(json_of(&resp)["error"]["code"], "invalid_signature");
    }
    assert!(h.graph.sent().is_empty(), "nada pode ser respondido");
    assert_eq!(h.repo.count(), 0, "nada pode ser gravado");
}

#[tokio::test]
async fn post_assinado_com_json_invalido_e_400() {
    let h = default_harness();
    let raw = "isso nao e json";
    let req = Request::post("/whatsapp/webhook")
        .header("X-Hub-Signature-256", sign(APP_SECRET, raw.as_bytes()))
        .body(Body::from(raw))
        .unwrap();
    let (st, _) = call(&h.app, req).await;
    assert_eq!(st, StatusCode::BAD_REQUEST);
}

// ---------------------------------------------------------------------------
// Leitura do payload
// ---------------------------------------------------------------------------

fn parse(v: &Value) -> Vec<Inbound> {
    parse_inbound(v.to_string().as_bytes(), PHONE_ID).unwrap()
}

#[test]
fn payload_de_texto() {
    let msgs = parse(&text_msg("wamid.t", "EXEMPLO LUZ"));
    assert_eq!(msgs.len(), 1);
    assert_eq!(msgs[0].from, FROM);
    assert_eq!(msgs[0].id, "wamid.t");
    assert_eq!(msgs[0].kind, InboundKind::Text("EXEMPLO LUZ".into()));
}

#[test]
fn payload_de_imagem_e_de_documento() {
    let img = parse(&image_msg("wamid.i", "MEDIA_IMG"));
    assert_eq!(img[0].kind, InboundKind::Media("MEDIA_IMG".into()));
    let doc = parse(&document_msg("wamid.d", "MEDIA_DOC"));
    assert_eq!(doc[0].kind, InboundKind::Media("MEDIA_DOC".into()));
}

#[test]
fn payload_de_tipo_desconhecido_ou_sem_corpo_vira_nao_suportado() {
    for m in [
        json!({ "from": FROM, "id": "a", "type": "audio", "audio": { "id": "X" } }),
        json!({ "from": FROM, "id": "b", "type": "sticker", "sticker": { "id": "X" } }),
        json!({ "from": FROM, "id": "c", "type": "image" }),
        json!({ "from": FROM, "id": "d", "type": "text" }),
        json!({ "from": FROM, "id": "e" }),
    ] {
        let msgs = parse(&envelope(m));
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].kind, InboundKind::Unsupported);
    }
}

#[test]
fn payload_sem_mensagens_de_outro_numero_ou_incompleto_e_ignorado() {
    assert!(parse(&status_update()).is_empty());
    assert!(parse(&json!({})).is_empty());
    assert!(parse(&json!({ "entry": [] })).is_empty());
    assert!(parse(&json!({ "entry": [{ "changes": [{}] }] })).is_empty());
    // Mensagem sem remetente ou sem id não dá para responder nem deduplicar.
    assert!(
        parse(&envelope(
            json!({ "id": "a", "type": "text", "text": { "body": "oi" } })
        ))
        .is_empty()
    );
    assert!(
        parse(&envelope(
            json!({ "from": FROM, "type": "text", "text": { "body": "oi" } })
        ))
        .is_empty()
    );
    // Outro número da mesma conta.
    let outro = parse_inbound(text_msg("wamid.x", "oi").to_string().as_bytes(), "999").unwrap();
    assert!(outro.is_empty());
}

#[test]
fn payload_que_nao_e_json_e_erro_e_varias_mensagens_saem_em_ordem() {
    assert!(parse_inbound(b"nao e json", PHONE_ID).is_err());
    let v = json!({ "entry": [{ "changes": [{ "value": {
        "metadata": { "phone_number_id": PHONE_ID },
        "messages": [
            { "from": FROM, "id": "1", "type": "text", "text": { "body": "um" } },
            { "from": FROM, "id": "2", "type": "text", "text": { "body": "dois" } }
        ]
    }}]}]});
    let ids: Vec<_> = parse(&v).into_iter().map(|m| m.id).collect();
    assert_eq!(ids, ["1", "2"]);
}

#[test]
fn debug_dos_tipos_que_carregam_numero_e_texto_nao_vaza() {
    let m = Inbound {
        from: FROM.into(),
        id: "wamid.secreto".into(),
        kind: InboundKind::Text("meu cpf é 123".into()),
    };
    let dbg = format!("{m:?} {:?}", m.kind);
    for proibido in [FROM, "wamid.secreto", "cpf", "123"] {
        assert!(!dbg.contains(proibido), "{dbg}");
    }
}

// ---------------------------------------------------------------------------
// Formatação da resposta
// ---------------------------------------------------------------------------

#[test]
fn dinheiro_e_mes_em_portugues() {
    assert_eq!(brl(0), "R$ 0,00");
    assert_eq!(brl(5), "R$ 0,05");
    assert_eq!(brl(1290), "R$ 12,90");
    assert_eq!(brl(123_456), "R$ 1.234,56");
    assert_eq!(brl(-100), "-R$ 1,00");
    assert_eq!(month_label("2026-10"), "outubro de 2026");
    assert_eq!(month_label("2026-03"), "março de 2026");
    assert_eq!(month_label("2026-13"), "2026-13");
    assert_eq!(month_label("outubro"), "outubro");
}

#[test]
fn resposta_com_achados_traz_empresa_total_contagem_valor_e_link() {
    let a = analysis("telecom-1");
    let texto = analysis_reply(&a, WEB);
    assert!(
        texto.starts_with("Vivaz Telecom · outubro de 2026\n"),
        "{texto}"
    );
    assert!(texto.contains(&format!("Total da fatura: {}", brl(a.bill.total_cents))));
    let problemas = a
        .findings
        .iter()
        .filter(|f| f.severity != Severity::Ok)
        .count();
    assert!(problemas > 0);
    assert!(texto.contains(&format!("Achei {problemas}")), "{texto}");
    if a.recoverable_cents > 0 {
        assert!(texto.contains(&format!("{} parece indevido", brl(a.recoverable_cents))));
    }
    assert!(texto.contains(&format!("{WEB}/r/{}", a.id)));
    assert!(texto.contains("Conteúdo informativo, não é orientação jurídica."));
}

#[test]
fn resposta_lista_no_maximo_3_achados_uma_linha_cada_e_resume_o_resto() {
    // Monta uma análise com 5 achados a partir de uma fatura real de exemplo.
    let mut a = analysis("telecom-1");
    let modelo = a
        .findings
        .iter()
        .find(|f| f.severity != Severity::Ok)
        .cloned()
        .expect("telecom-1 tem achado");
    a.findings = (0..5)
        .map(|i| {
            let mut f = modelo.clone();
            f.id = format!("X-{i}");
            f.title = format!("Achado número {i}");
            f
        })
        .collect();
    let texto = analysis_reply(&a, WEB);
    let linhas: Vec<_> = texto.lines().filter(|l| l.starts_with("• ")).collect();
    assert_eq!(linhas.len(), MAX_FINDINGS_IN_REPLY, "{texto}");
    assert!(texto.contains("Achei 5 pontos para conferir."));
    assert!(texto.contains("+ 2 outros pontos no link."), "{texto}");
    assert!(texto.contains("Achado número 0") && !texto.contains("Achado número 3"));
}

#[test]
fn resposta_sem_achados_diz_que_nao_achou_nada_e_nao_cita_valor_indevido() {
    let a = analysis("telecom-3");
    let reais: Vec<_> = a
        .findings
        .iter()
        .filter(|f| f.severity != Severity::Ok)
        .collect();
    assert!(reais.is_empty(), "telecom-3 é a conta sem problemas");
    let texto = analysis_reply(&a, WEB);
    assert!(texto.contains("Não encontrei nada de errado"), "{texto}");
    assert!(!texto.contains("indevido"));
    assert!(!texto.contains("• "));
    assert!(texto.contains(&format!("{WEB}/r/{}", a.id)));
}

#[test]
fn resposta_so_com_suspeitos_nao_afirma_valor_indevido() {
    let mut a = analysis("telecom-1");
    a.findings.retain(|f| f.severity == Severity::Suspicious);
    a.recoverable_cents = 0;
    assert!(!a.findings.is_empty(), "telecom-1 tem achado suspeito");
    let texto = analysis_reply(&a, WEB);
    assert!(
        texto.contains("Nada está confirmado como indevido"),
        "{texto}"
    );
    assert!(texto.contains("Suspeito: "));
    assert!(!texto.contains("Indevido: "));
}

#[test]
fn resposta_de_um_achado_usa_o_singular() {
    let mut a = analysis("telecom-1");
    a.findings.truncate(1);
    a.findings[0].severity = Severity::Suspicious;
    assert!(analysis_reply(&a, WEB).contains("Achei 1 ponto para conferir."));
}

#[test]
fn menu_diz_como_usar_e_avisa_da_leitura_por_ia() {
    let m = menu_text(WEB);
    assert!(m.contains("EXEMPLO"));
    assert!(m.contains("foto") && m.contains("PDF"));
    assert!(m.contains("IA do Google"));
    assert!(m.contains(&format!("{WEB}/privacidade")));
}

#[test]
fn cada_erro_tem_mensagem_propria_sem_termo_tecnico() {
    let todos = [
        BotError::Unreadable,
        BotError::TooLarge,
        BotError::ReadFailed,
        BotError::RateLimited,
        BotError::Unavailable,
        BotError::DownloadFailed,
        BotError::Unsupported,
        BotError::Internal,
    ];
    let textos: HashSet<_> = todos.iter().map(|e| error_text(*e)).collect();
    assert_eq!(textos.len(), todos.len());
    for t in textos {
        assert!(!t.is_empty());
        for jargao in ["HTTP", "erro 4", "erro 5", "exception", "null", "API"] {
            assert!(!t.contains(jargao), "{t}");
        }
    }
    assert!(error_text(BotError::TooLarge).contains("10 MB"));
}

#[test]
fn erro_da_api_vira_o_motivo_certo_para_o_usuario() {
    let e = |code| ApiError::new(StatusCode::BAD_REQUEST, code, "x");
    assert_eq!(BotError::from(&e("too_large")), BotError::TooLarge);
    assert_eq!(BotError::from(&e("invalid_file")), BotError::Unreadable);
    assert_eq!(
        BotError::from(&e("extraction_failed")),
        BotError::ReadFailed
    );
    assert_eq!(BotError::from(&e("rate_limited")), BotError::RateLimited);
    assert_eq!(
        BotError::from(&e("extraction_unavailable")),
        BotError::Unavailable
    );
    assert_eq!(BotError::from(&e("internal")), BotError::Internal);
}

// ---------------------------------------------------------------------------
// Entendimento do texto
// ---------------------------------------------------------------------------

#[test]
fn intencao_do_texto() {
    use Intent::*;
    for (texto, esperado) in [
        ("EXEMPLO", Sample("telecom-1")),
        ("  exemplo  ", Sample("telecom-1")),
        ("Exemplo!", Sample("telecom-1")),
        ("exemplo luz", Sample("energia-1")),
        ("EXEMPLO ENERGIA 2", Sample("energia-2")),
        ("exemplo celular 3", Sample("telecom-3")),
        ("exemplo 2", Sample("telecom-2")),
        ("exemplo 9", Sample("telecom-1")),
        ("telecom-2", Sample("telecom-2")),
        ("energia-3", Sample("energia-3")),
        ("teste", Sample("telecom-1")),
        ("menu", Menu),
        ("Olá", Menu),
        ("oi", Menu),
        ("Bom dia", Menu),
        ("ajuda", Menu),
        ("", Unknown),
        ("   ", Unknown),
        ("quanto custa?", Unknown),
        ("../../etc/passwd", Unknown),
    ] {
        assert_eq!(parse_intent(texto), esperado, "{texto:?}");
    }
}

#[test]
fn todo_exemplo_que_o_bot_conhece_existe_nas_fixtures() {
    for s in SAMPLES {
        assert!(
            fixtures().join(format!("extracted/{s}.json")).exists(),
            "{s}"
        );
    }
}

// ---------------------------------------------------------------------------
// Simulador
// ---------------------------------------------------------------------------

fn sim(body: Value) -> Request<Body> {
    Request::post("/whatsapp/simulador")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

#[tokio::test]
async fn simulador_com_exemplo_cria_a_analise_sem_ia_e_devolve_o_resultado() {
    // Sem extrator e com o webhook desligado: o simulador continua funcionando.
    let h = harness(false, None, Arc::new(FakeGraph::default()), None);
    let (st, body) = call(&h.app, sim(json!({ "exemplo": "telecom-1" }))).await;
    assert_eq!(st, StatusCode::OK, "{body}");
    let v = json_of(&body);
    let msg = &v["mensagens"][0];
    let id = msg["resultado_id"]
        .as_str()
        .expect("traz o id do resultado");
    assert!(Uuid::parse_str(id).is_ok());
    let texto = msg["texto"].as_str().unwrap();
    assert!(texto.contains("Vivaz Telecom"));
    assert!(texto.contains(&format!("/r/{id}")));
    assert_eq!(h.repo.count(), 1, "a análise foi gravada");
}

#[tokio::test]
async fn simulador_com_texto_segue_a_mesma_conversa_do_bot() {
    let h = harness(false, None, Arc::new(FakeGraph::default()), None);
    let (_, body) = call(&h.app, sim(json!({ "texto": "EXEMPLO LUZ" }))).await;
    let v = json_of(&body);
    assert!(
        v["mensagens"][0]["texto"]
            .as_str()
            .unwrap()
            .contains("Luzia Energia")
    );
    assert!(v["mensagens"][0]["resultado_id"].is_string());

    let (_, body) = call(&h.app, sim(json!({ "texto": "menu" }))).await;
    let v = json_of(&body);
    assert_eq!(v["mensagens"][0]["texto"], menu_text(WEB));
    assert!(v["mensagens"][0]["resultado_id"].is_null());

    let (_, body) = call(&h.app, sim(json!({ "texto": "blá blá" }))).await;
    assert_eq!(json_of(&body)["mensagens"][0]["texto"], unknown_text());
}

#[tokio::test]
async fn simulador_recusa_corpo_invalido_exemplo_inexistente_e_texto_longo() {
    let h = harness(false, None, Arc::new(FakeGraph::default()), None);
    for corpo in [
        json!({}),
        json!({ "texto": "oi", "exemplo": "telecom-1" }),
        json!({ "texto": "   " }),
        json!({ "texto": "a".repeat(MAX_SIM_TEXT_CHARS + 1) }),
        json!({ "arquivo": "x" }),
        json!({ "texto": 5 }),
    ] {
        let (st, body) = call(&h.app, sim(corpo.clone())).await;
        assert_eq!(st, StatusCode::BAD_REQUEST, "{corpo}");
        assert_eq!(json_of(&body)["error"]["code"], "invalid_body");
    }
    for exemplo in ["nao-existe", "..%2Fsegredo", "TELECOM-1", ""] {
        let (st, _) = call(&h.app, sim(json!({ "exemplo": exemplo }))).await;
        assert_eq!(st, StatusCode::NOT_FOUND, "{exemplo}");
    }
    assert_eq!(h.repo.count(), 0);
}

#[tokio::test]
async fn simulador_nao_aceita_upload_de_arquivo() {
    let h = harness(false, Some(Ok(())), Arc::new(FakeGraph::default()), None);
    let req = Request::post("/whatsapp/simulador")
        .header(header::CONTENT_TYPE, "multipart/form-data; boundary=X")
        .body(Body::from(
            "--X\r\nContent-Disposition: form-data; name=\"file\"; filename=\"f.pdf\"\r\n\r\n%PDF-1.7\r\n--X--\r\n",
        ))
        .unwrap();
    let (st, _) = call(&h.app, req).await;
    assert_eq!(st, StatusCode::BAD_REQUEST);
    assert_eq!(h.repo.count(), 0);
}

#[tokio::test]
async fn simulador_tem_o_mesmo_rate_limit_das_rotas_de_escrita() {
    let limites = Limits {
        write_burst: 2,
        write_refill: Duration::from_secs(3600),
        ..Limits::PRODUCTION
    };
    let h = harness(false, None, Arc::new(FakeGraph::default()), Some(limites));
    let pedido = || {
        let mut r = sim(json!({ "texto": "menu" }));
        r.headers_mut()
            .insert("x-forwarded-for", "203.0.113.7".parse().unwrap());
        r
    };
    assert_eq!(call(&h.app, pedido()).await.0, StatusCode::OK);
    assert_eq!(call(&h.app, pedido()).await.0, StatusCode::OK);
    let (st, body) = call(&h.app, pedido()).await;
    assert_eq!(st, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(json_of(&body)["error"]["code"], "rate_limited");
}

// ---------------------------------------------------------------------------
// Fluxo completo do webhook (com a Graph de mentira)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn foto_enviada_vira_analise_e_o_bot_responde_com_aviso_e_resultado() {
    let h = default_harness();
    let (st, _) = call(&h.app, signed(&image_msg("wamid.f1", "MEDIA1"))).await;
    assert_eq!(st, StatusCode::OK);
    let sent = h.graph.sent();
    assert_eq!(sent.len(), 2, "aviso de recebimento + resultado: {sent:?}");
    assert!(sent.iter().all(|(to, _)| to == FROM));
    assert_eq!(sent[0].1, ACK_TEXT);
    assert!(sent[1].1.contains("Vivaz Telecom"));
    assert!(sent[1].1.contains(&format!("{WEB}/r/")));
    assert_eq!(h.repo.count(), 1);
}

#[tokio::test]
async fn pdf_como_documento_usa_o_mesmo_caminho() {
    let h = default_harness();
    call(&h.app, signed(&document_msg("wamid.d1", "MEDIA1"))).await;
    assert_eq!(h.graph.sent().len(), 2);
    assert_eq!(h.repo.count(), 1);
}

#[tokio::test]
async fn texto_exemplo_responde_sem_gastar_ia() {
    // Sem extrator: só a demo.
    let h = harness(
        true,
        None,
        FakeGraph::with_media("M", Ok(PDF.to_vec())),
        None,
    );
    call(&h.app, signed(&text_msg("wamid.e1", "EXEMPLO"))).await;
    let sent = h.graph.sent();
    assert_eq!(sent.len(), 1);
    assert!(sent[0].1.contains("Vivaz Telecom"));
    assert_eq!(h.repo.count(), 1);
}

#[tokio::test]
async fn texto_qualquer_recebe_o_menu_ou_a_ajuda() {
    let h = default_harness();
    call(&h.app, signed(&text_msg("wamid.m1", "oi"))).await;
    call(
        &h.app,
        signed(&text_msg("wamid.m2", "qual o sentido da vida")),
    )
    .await;
    let sent = h.graph.sent();
    assert_eq!(sent[0].1, menu_text(WEB));
    assert_eq!(sent[1].1, unknown_text());
    assert_eq!(h.repo.count(), 0);
}

#[tokio::test]
async fn tipos_nao_suportados_recebem_orientacao() {
    let h = default_harness();
    let audio = envelope(
        json!({ "from": FROM, "id": "wamid.a1", "type": "audio", "audio": { "id": "A" } }),
    );
    call(&h.app, signed(&audio)).await;
    assert_eq!(h.graph.sent()[0].1, error_text(BotError::Unsupported));
}

#[tokio::test]
async fn erros_de_arquivo_viram_mensagens_amigaveis() {
    // Arquivo que não é foto nem PDF.
    let g = FakeGraph::with_media("DOCX", Ok(b"PK\x03\x04docx-falso".to_vec()));
    let h = harness(true, Some(Ok(())), g, None);
    call(&h.app, signed(&document_msg("wamid.x1", "DOCX"))).await;
    assert_eq!(
        h.graph.sent().last().unwrap().1,
        error_text(BotError::Unreadable)
    );

    // Arquivo grande demais, avisado pela Graph.
    let g = FakeGraph::with_media("GRANDE", Err(GraphError::TooLarge));
    let h = harness(true, Some(Ok(())), g, None);
    call(&h.app, signed(&document_msg("wamid.x2", "GRANDE"))).await;
    assert_eq!(
        h.graph.sent().last().unwrap().1,
        error_text(BotError::TooLarge)
    );

    // Arquivo grande demais, só percebido na validação.
    let mut grande = PDF.to_vec();
    grande.resize(MAX_BYTES + 1, b' ');
    let g = FakeGraph::with_media("GRANDE2", Ok(grande));
    let h = harness(true, Some(Ok(())), g, None);
    call(&h.app, signed(&document_msg("wamid.x3", "GRANDE2"))).await;
    assert_eq!(
        h.graph.sent().last().unwrap().1,
        error_text(BotError::TooLarge)
    );

    // Falha ao baixar.
    let h = harness(true, Some(Ok(())), Arc::new(FakeGraph::default()), None);
    call(&h.app, signed(&image_msg("wamid.x4", "SUMIU"))).await;
    assert_eq!(
        h.graph.sent().last().unwrap().1,
        error_text(BotError::DownloadFailed)
    );

    // A IA não conseguiu ler.
    let g = FakeGraph::with_media("M", Ok(PDF.to_vec()));
    let h = harness(true, Some(Err(ExtractError::Provider("x".into()))), g, None);
    call(&h.app, signed(&image_msg("wamid.x5", "M"))).await;
    assert_eq!(
        h.graph.sent().last().unwrap().1,
        error_text(BotError::ReadFailed)
    );

    // Quota da IA esgotada.
    let g = FakeGraph::with_media("M", Ok(PDF.to_vec()));
    let h = harness(true, Some(Err(ExtractError::RateLimited)), g, None);
    call(&h.app, signed(&image_msg("wamid.x6", "M"))).await;
    assert_eq!(
        h.graph.sent().last().unwrap().1,
        error_text(BotError::RateLimited)
    );

    // Sem chave da IA: só o exemplo funciona.
    let g = FakeGraph::with_media("M", Ok(PDF.to_vec()));
    let h = harness(true, None, g, None);
    call(&h.app, signed(&image_msg("wamid.x7", "M"))).await;
    assert_eq!(
        h.graph.sent().last().unwrap().1,
        error_text(BotError::Unavailable)
    );
    assert_eq!(h.repo.count(), 0);
}

#[tokio::test]
async fn limite_de_tentativas_por_remetente_vale_para_fotos_e_exemplos() {
    let h = default_harness();
    for i in 0..UPLOADS_PER_SENDER {
        call(&h.app, signed(&image_msg(&format!("wamid.l{i}"), "MEDIA1"))).await;
    }
    assert_eq!(h.repo.count(), UPLOADS_PER_SENDER);
    let antes = h.graph.sent().len();
    call(&h.app, signed(&image_msg("wamid.l-extra", "MEDIA1"))).await;
    let sent = h.graph.sent();
    assert_eq!(
        sent.len(),
        antes + 1,
        "só a mensagem de limite, sem aviso de leitura"
    );
    assert_eq!(sent.last().unwrap().1, error_text(BotError::RateLimited));
    assert_eq!(
        h.repo.count(),
        UPLOADS_PER_SENDER,
        "a IA não foi chamada de novo"
    );

    let h = default_harness();
    for i in 0..SAMPLES_PER_SENDER {
        call(&h.app, signed(&text_msg(&format!("wamid.s{i}"), "exemplo"))).await;
    }
    call(&h.app, signed(&text_msg("wamid.s-extra", "exemplo"))).await;
    assert_eq!(
        h.graph.sent().last().unwrap().1,
        error_text(BotError::RateLimited)
    );
    assert_eq!(h.repo.count(), SAMPLES_PER_SENDER);
}

#[test]
fn janela_do_limite_libera_de_novo_depois_do_tempo() {
    let t = Throttle::new(2, Duration::from_secs(60));
    let k = hash_key("a");
    let t0 = Instant::now();
    assert!(t.allow(k, t0));
    assert!(t.allow(k, t0));
    assert!(!t.allow(k, t0 + Duration::from_secs(30)));
    assert!(t.allow(k, t0 + Duration::from_secs(61)));
    // Outro remetente não é afetado.
    assert!(t.allow(hash_key("b"), t0));
}

#[tokio::test]
async fn entrega_repetida_da_mesma_mensagem_e_processada_uma_vez() {
    let h = default_harness();
    let corpo = image_msg("wamid.dup", "MEDIA1");
    call(&h.app, signed(&corpo)).await;
    let (st, _) = call(&h.app, signed(&corpo)).await;
    assert_eq!(
        st,
        StatusCode::OK,
        "a Meta precisa receber 200 mesmo na repetição"
    );
    assert_eq!(h.graph.sent().len(), 2);
    assert_eq!(h.repo.count(), 1);
}

#[test]
fn dedup_esquece_as_mais_antigas_quando_enche() {
    let d = Dedup::default();
    assert!(d.first_time("a"));
    assert!(!d.first_time("a"));
    for i in 0..DEDUP_CAPACITY {
        assert!(d.first_time(&format!("m{i}")));
    }
    assert!(d.first_time("a"), "a mais antiga saiu da lembrança");
}

// ---------------------------------------------------------------------------
// Cliente HTTP da Graph API, contra uma Graph de mentira em localhost
// ---------------------------------------------------------------------------

#[tokio::test]
async fn cliente_http_baixa_a_midia_e_envia_texto_no_formato_da_graph() {
    use axum::{extract::Path as AxPath, routing::get as ax_get};

    #[derive(Clone, Default)]
    struct Visto {
        enviados: Arc<Mutex<Vec<(String, Value)>>>,
        tokens: Arc<Mutex<Vec<String>>>,
    }
    let visto = Visto::default();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());

    let meta_base = base.clone();
    let app = Router::new()
        .route(
            "/v1/{id}",
            ax_get({
                let visto = visto.clone();
                move |AxPath(id): AxPath<String>, h: HeaderMap| {
                    let visto = visto.clone();
                    let meta_base = meta_base.clone();
                    async move {
                        visto.tokens.lock().unwrap().push(
                            h.get("authorization").unwrap().to_str().unwrap().to_string(),
                        );
                        match id.as_str() {
                            "OK" => Json(json!({ "url": format!("{meta_base}/arquivo"), "file_size": 4 })).into_response(),
                            "GRANDE" => Json(json!({ "url": format!("{meta_base}/arquivo"), "file_size": 999_999_999u64 })).into_response(),
                            "SEM_URL" => Json(json!({})).into_response(),
                            _ => StatusCode::NOT_FOUND.into_response(),
                        }
                    }
                }
            }),
        )
        .route("/arquivo", ax_get(|| async { "ABCD" }))
        .route(
            "/v1/{phone}/messages",
            post({
                let visto = visto.clone();
                move |AxPath(phone): AxPath<String>, Json(body): Json<Value>| {
                    let visto = visto.clone();
                    async move {
                        visto.enviados.lock().unwrap().push((phone, body));
                        Json(json!({ "messages": [{ "id": "wamid.Z" }] }))
                    }
                }
            }),
        );
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

    let cfg = Arc::new(WhatsappConfig {
        graph_url: format!("{base}/v1"),
        ..config()
    });
    let client = HttpGraphClient::new(cfg);

    assert_eq!(client.download_media("OK", 1024).await.unwrap(), b"ABCD");
    assert_eq!(
        client.download_media("OK", 3).await,
        Err(GraphError::TooLarge)
    );
    assert_eq!(
        client.download_media("GRANDE", 1024).await,
        Err(GraphError::TooLarge)
    );
    assert_eq!(
        client.download_media("SEM_URL", 1024).await,
        Err(GraphError::Failed)
    );
    assert_eq!(
        client.download_media("NAO_EXISTE", 1024).await,
        Err(GraphError::Failed)
    );
    // Id com caracteres de caminho nunca chega à rede.
    assert_eq!(
        client.download_media("../x", 1024).await,
        Err(GraphError::Failed)
    );
    assert_eq!(
        client.download_media("", 1024).await,
        Err(GraphError::Failed)
    );
    assert!(
        visto
            .tokens
            .lock()
            .unwrap()
            .iter()
            .all(|t| t == "Bearer token-da-graph")
    );

    client.send_text(FROM, "Olá!").await.unwrap();
    let enviados = visto.enviados.lock().unwrap().clone();
    assert_eq!(enviados.len(), 1);
    assert_eq!(enviados[0].0, PHONE_ID);
    assert_eq!(enviados[0].1["messaging_product"], "whatsapp");
    assert_eq!(enviados[0].1["to"], FROM);
    assert_eq!(enviados[0].1["type"], "text");
    assert_eq!(enviados[0].1["text"]["body"], "Olá!");
}

#[test]
fn midia_so_vem_por_https_salvo_na_graph_de_teste_em_http() {
    let prod = "https://graph.facebook.com/v23.0";
    assert!(media_url_allowed("https://lookaside.fbsbx.com/x", prod));
    assert!(!media_url_allowed("http://lookaside.fbsbx.com/x", prod));
    assert!(!media_url_allowed("file:///etc/passwd", prod));
    assert!(!media_url_allowed("ftp://x/y", prod));
    assert!(media_url_allowed(
        "http://127.0.0.1:9/x",
        "http://127.0.0.1:9/v1"
    ));
}

// ---------------------------------------------------------------------------
// Logs (RNF13)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn logs_do_webhook_nao_tem_telefone_texto_nem_conteudo_da_fatura() {
    let buf = logs_do_processo();
    let h = default_harness();

    call(&h.app, signed(&image_msg("wamid.LOG-IMG", "MEDIA1"))).await;
    call(&h.app, signed(&document_msg("wamid.LOG-DOC", "MEDIA1"))).await;
    call(
        &h.app,
        signed(&text_msg("wamid.LOG-TXT", "EXEMPLO segredo-do-texto")),
    )
    .await;
    call(
        &h.app,
        signed(&text_msg("wamid.LOG-MENU", "meu cpf é 123.456.789-00")),
    )
    .await;
    // Assinatura inválida e erros de arquivo também registram log: nenhum pode vazar.
    call(
        &h.app,
        post_webhook(
            &text_msg("wamid.LOG-BAD", "texto-do-intruso"),
            Some("sha256=00".into()),
        ),
    )
    .await;
    let g = FakeGraph::with_media("RUIM", Ok(b"PK\x03\x04docx-falso".to_vec()));
    let h2 = harness(true, Some(Ok(())), g, None);
    call(&h2.app, signed(&document_msg("wamid.LOG-ERR", "RUIM"))).await;
    call(&h2.app, signed(&image_msg("wamid.LOG-DL", "NAO-EXISTE"))).await;
    // Simulador.
    let sim_h = harness(false, None, Arc::new(FakeGraph::default()), None);
    call(&sim_h.app, sim(json!({ "texto": "exemplo luz" }))).await;

    let logs = String::from_utf8_lossy(&buf.0.lock().unwrap()).into_owned();
    assert!(
        logs.contains("whatsapp: análise criada"),
        "o fluxo precisa ter registrado o id da análise"
    );
    assert!(logs.contains("whatsapp: assinatura inválida"));
    for proibido in [
        FROM,
        "5511987654321",
        "wamid.",
        "LOG-IMG",
        "MEDIA1",
        "NAO-EXISTE",
        "segredo-do-texto",
        "segredo-do-app",
        "token-da-graph",
        "token-combinado",
        "cpf",
        "123.456.789-00",
        "texto-do-intruso",
        "Fulana",
        "minha conta",
        "fatura.pdf",
        "Clube Vivaz",
        "CLUBE VZ",
        "Seguro Tela",
        "PLANO VZ",
        "Vivaz Telecom",
        "Luzia Energia",
        "12,90",
        "Olá!",
    ] {
        assert!(
            !logs.contains(proibido),
            "conteúdo vazou nos logs: {proibido}"
        );
    }
}
