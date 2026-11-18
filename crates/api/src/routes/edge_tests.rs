//! Testes de borda das rotas HTTP: validação de upload, JSON malformado, expiração,
//! banco indisponível, falhas do extrator (Gemini sobrecarregado), rate limit e overflow.
//!
//! Só dados sintéticos (fixtures com marcas fictícias) e nenhuma chamada real de rede:
//! o extrator é sempre um mock e o banco é o `MemRepo` (com falhas injetáveis).

use std::{
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::Duration as StdDuration,
};

use async_trait::async_trait;
use axum::{
    body::Body,
    http::{HeaderMap, Request, StatusCode, header},
};
use chrono::{DateTime, FixedOffset, Utc};
use http_body_util::BodyExt;
use humanobill_core::{Analysis, AnalysisSource, ExtractedBill, ValidationIssue};
use humanobill_extract::{BillExtractor, ExtractError, InputMime};
use serde_json::{Value, json};
use tower::ServiceExt;
use uuid::Uuid;

use super::*;
use crate::{
    limits::Limits,
    repo::{MemRepo, Repo, RepoError, Stored},
    upload::MAX_BYTES,
};

// ───────────────────────────── infraestrutura de teste ─────────────────────────────

fn fixtures() -> PathBuf {
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures"))
}

fn carregar_bill(nome: &str) -> ExtractedBill {
    let raw = std::fs::read_to_string(fixtures().join(format!("extracted/{nome}.json"))).unwrap();
    serde_json::from_str(&raw).unwrap()
}

fn erro_db() -> RepoError {
    RepoError::Db(sqlx::Error::PoolTimedOut)
}

/// Repositório que delega ao `MemRepo`, conta chamadas e permite injetar falhas.
#[derive(Default)]
struct SpyRepo {
    inner: MemRepo,
    falha_ping: AtomicBool,
    falha_insert: AtomicBool,
    falha_get: AtomicBool,
    falha_update: AtomicBool,
    inserts: AtomicUsize,
    updates: AtomicUsize,
}

impl SpyRepo {
    fn inserts(&self) -> usize {
        self.inserts.load(Ordering::SeqCst)
    }
    fn updates(&self) -> usize {
        self.updates.load(Ordering::SeqCst)
    }
}

#[async_trait]
impl Repo for SpyRepo {
    async fn ping(&self) -> Result<(), RepoError> {
        if self.falha_ping.load(Ordering::SeqCst) {
            return Err(erro_db());
        }
        self.inner.ping().await
    }
    async fn insert(&self, a: &Analysis) -> Result<(), RepoError> {
        if self.falha_insert.load(Ordering::SeqCst) {
            return Err(erro_db());
        }
        self.inserts.fetch_add(1, Ordering::SeqCst);
        self.inner.insert(a).await
    }
    async fn get(&self, id: Uuid) -> Result<Option<Stored>, RepoError> {
        if self.falha_get.load(Ordering::SeqCst) {
            return Err(erro_db());
        }
        self.inner.get(id).await
    }
    async fn update(&self, a: &Analysis) -> Result<(), RepoError> {
        if self.falha_update.load(Ordering::SeqCst) {
            return Err(erro_db());
        }
        self.updates.fetch_add(1, Ordering::SeqCst);
        self.inner.update(a).await
    }
}

type Saida = Box<dyn Fn(usize) -> Result<ExtractedBill, ExtractError> + Send + Sync>;

/// Extrator falso: o comportamento depende do nº da chamada (0, 1, 2...).
struct MockExtractor {
    saida: Saida,
    atraso: StdDuration,
    chamadas: AtomicUsize,
    mimes: Mutex<Vec<InputMime>>,
}

impl MockExtractor {
    fn chamadas(&self) -> usize {
        self.chamadas.load(Ordering::SeqCst)
    }
}

#[async_trait]
impl BillExtractor for MockExtractor {
    async fn extract(&self, _: &[u8], mime: InputMime) -> Result<ExtractedBill, ExtractError> {
        let n = self.chamadas.fetch_add(1, Ordering::SeqCst);
        self.mimes.lock().unwrap().push(mime);
        if !self.atraso.is_zero() {
            tokio::time::sleep(self.atraso).await;
        }
        (self.saida)(n)
    }
}

struct Amb {
    app: Router,
    repo: Arc<SpyRepo>,
    ext: Option<Arc<MockExtractor>>,
}

fn montar(saida: Option<Saida>, atraso_ms: u64, dir: PathBuf, limits: Option<Limits>) -> Amb {
    let repo = Arc::new(SpyRepo::default());
    let ext = saida.map(|saida| {
        Arc::new(MockExtractor {
            saida,
            atraso: StdDuration::from_millis(atraso_ms),
            chamadas: AtomicUsize::new(0),
            mimes: Mutex::new(Vec::new()),
        })
    });
    let app = router(AppState {
        repo: repo.clone(),
        extractor: ext.clone().map(|e| e as Arc<dyn BillExtractor>),
        fixtures_dir: dir,
        limits,
    });
    Amb { app, repo, ext }
}

/// Sem extrator (só a demo funciona), sem rate limit.
fn amb() -> Amb {
    montar(None, 0, fixtures(), None)
}

/// Extrator que sempre devolve a fatura telecom-1.
fn amb_ia() -> Amb {
    amb_ia_com(Box::new(|_| Ok(carregar_bill("telecom-1"))))
}

fn amb_ia_com(saida: Saida) -> Amb {
    montar(Some(saida), 0, fixtures(), None)
}

fn amb_com_limites(l: Limits) -> Amb {
    montar(
        Some(Box::new(|_| Ok(carregar_bill("telecom-1")))),
        0,
        fixtures(),
        Some(l),
    )
}

async fn send_full(app: &Router, req: Request<Body>) -> (StatusCode, HeaderMap, Vec<u8>) {
    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let headers = resp.headers().clone();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    (status, headers, bytes.to_vec())
}

async fn send(app: &Router, req: Request<Body>) -> (StatusCode, Value) {
    let (st, _, bytes) = send_full(app, req).await;
    (st, serde_json::from_slice(&bytes).unwrap_or(Value::Null))
}

/// Envia a requisição em outra task: se o handler der panic, devolve `None` em vez de
/// derrubar o teste.
async fn send_sem_panico(app: &Router, req: Request<Body>) -> Option<(StatusCode, Value)> {
    let app = app.clone();
    tokio::spawn(async move {
        let resp = app.oneshot(req).await.unwrap();
        let st = resp.status();
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        (st, serde_json::from_slice(&bytes).unwrap_or(Value::Null))
    })
    .await
    .ok()
}

fn get(uri: &str) -> Request<Body> {
    Request::get(uri).body(Body::empty()).unwrap()
}

fn post_vazio(uri: &str) -> Request<Body> {
    Request::post(uri).body(Body::empty()).unwrap()
}

fn json_req(method: &str, uri: &str, ct: Option<&str>, body: impl Into<Body>) -> Request<Body> {
    let mut b = Request::builder().method(method).uri(uri);
    if let Some(ct) = ct {
        b = b.header(header::CONTENT_TYPE, ct);
    }
    b.body(body.into()).unwrap()
}

fn patch(id: &str, body: &str) -> Request<Body> {
    json_req(
        "PATCH",
        &format!("/analyses/{id}"),
        Some("application/json"),
        body.to_string(),
    )
}

fn patch_json(id: &str, v: &Value) -> Request<Body> {
    patch(id, &v.to_string())
}

fn dispute(id: &str, body: &str) -> Request<Body> {
    json_req(
        "POST",
        &format!("/analyses/{id}/dispute"),
        Some("application/json"),
        body.to_string(),
    )
}

async fn demo(a: &Amb, sample: &str) -> Value {
    let (st, v) = send(&a.app, post_vazio(&format!("/analyses/demo/{sample}"))).await;
    assert_eq!(st, StatusCode::CREATED, "demo {sample}: {v}");
    v
}

fn id_de(v: &Value) -> String {
    v["id"].as_str().unwrap().to_string()
}

fn achados(v: &Value) -> &Vec<Value> {
    v["findings"].as_array().unwrap()
}

/// Grava direto no repositório uma análise com a validade pedida.
async fn semear(a: &Amb, fixture: &str, expires_at: &str) -> String {
    let mut an = new_analysis(carregar_bill(fixture), AnalysisSource::Demo);
    an.expires_at = expires_at.to_string();
    a.repo.insert(&an).await.unwrap();
    an.id
}

/// Diretório temporário com `extracted/` próprio (apagado no Drop).
struct DirTemp(PathBuf);

impl DirTemp {
    fn nova() -> Self {
        let p = std::env::temp_dir().join(format!("humanobill-api-edge-{}", Uuid::new_v4()));
        std::fs::create_dir_all(p.join("extracted")).unwrap();
        Self(p)
    }
    fn escrever(&self, nome: &str, conteudo: &str) {
        std::fs::write(self.0.join("extracted").join(nome), conteudo).unwrap();
    }
}

impl Drop for DirTemp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

// ── multipart ──

struct Parte {
    nome: String,
    arquivo: Option<String>,
    tipo: Option<String>,
    conteudo: Vec<u8>,
}

fn consent(v: &str) -> Parte {
    Parte {
        nome: "consent".into(),
        arquivo: None,
        tipo: None,
        conteudo: v.as_bytes().to_vec(),
    }
}

fn arquivo(bytes: &[u8]) -> Parte {
    Parte {
        nome: "file".into(),
        arquivo: Some("fatura.pdf".into()),
        tipo: Some("application/pdf".into()),
        conteudo: bytes.to_vec(),
    }
}

fn parte(nome: &str, v: &str) -> Parte {
    Parte {
        nome: nome.into(),
        arquivo: None,
        tipo: None,
        conteudo: v.as_bytes().to_vec(),
    }
}

fn multipart(partes: Vec<Parte>) -> Request<Body> {
    let b = "XBORDAX";
    let mut body = Vec::new();
    for p in partes {
        let mut cab = format!(
            "--{b}\r\nContent-Disposition: form-data; name=\"{}\"",
            p.nome
        );
        if let Some(f) = &p.arquivo {
            cab += &format!("; filename=\"{f}\"");
        }
        if let Some(t) = &p.tipo {
            cab += &format!("\r\nContent-Type: {t}");
        }
        cab += "\r\n\r\n";
        body.extend(cab.as_bytes());
        body.extend(&p.conteudo);
        body.extend(b"\r\n");
    }
    body.extend(format!("--{b}--\r\n").as_bytes());
    json_req(
        "POST",
        "/analyses",
        Some(&format!("multipart/form-data; boundary={b}")),
        body,
    )
}

const PDF: &[u8] = b"%PDF-1.7\n1 0 obj << /Type /Page >> endobj\n";
const PNG: [u8; 12] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0, 0];

fn pdf_com_tamanho(n: usize) -> Vec<u8> {
    let mut v = PDF.to_vec();
    v.resize(n, b' ');
    v
}

// ───────────────────────────── banco indisponível ─────────────────────────────

#[tokio::test]
async fn health_com_banco_fora_responde_503_degradado() {
    let a = amb();
    a.repo.falha_ping.store(true, Ordering::SeqCst);
    let (st, v) = send(&a.app, get("/health")).await;
    assert_eq!(st, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(v["status"], "degraded");
    assert_eq!(v["db"], "down");
    assert!(!v["version"].as_str().unwrap().is_empty());
}

#[tokio::test]
async fn health_volta_ao_normal_quando_o_banco_volta() {
    let a = amb();
    a.repo.falha_ping.store(true, Ordering::SeqCst);
    assert_eq!(
        send(&a.app, get("/health")).await.0,
        StatusCode::SERVICE_UNAVAILABLE
    );
    a.repo.falha_ping.store(false, Ordering::SeqCst);
    let (st, v) = send(&a.app, get("/health")).await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(v["db"], "ok");
}

#[tokio::test]
async fn demo_com_banco_fora_e_500_sem_vazar_detalhe_do_driver() {
    let a = amb();
    a.repo.falha_insert.store(true, Ordering::SeqCst);
    let (st, _, bytes) = send_full(&a.app, post_vazio("/analyses/demo/telecom-1")).await;
    assert_eq!(st, StatusCode::INTERNAL_SERVER_ERROR);
    let txt = String::from_utf8(bytes).unwrap();
    let v: Value = serde_json::from_str(&txt).unwrap();
    assert_eq!(v["error"]["code"], "internal");
    for proibido in ["pool", "timed out", "sqlx", "banco:"] {
        assert!(!txt.to_lowercase().contains(proibido), "vazou: {proibido}");
    }
}

#[tokio::test]
async fn get_com_banco_fora_e_500() {
    let a = amb();
    let id = id_de(&demo(&a, "telecom-1").await);
    a.repo.falha_get.store(true, Ordering::SeqCst);
    let (st, v) = send(&a.app, get(&format!("/analyses/{id}"))).await;
    assert_eq!(st, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(v["error"]["code"], "internal");
}

#[tokio::test]
async fn patch_com_banco_fora_no_update_e_500_e_nao_persiste() {
    let a = amb();
    let v0 = demo(&a, "telecom-1").await;
    let id = id_de(&v0);
    a.repo.falha_update.store(true, Ordering::SeqCst);
    let (st, v) = send(&a.app, patch(&id, r#"{"paid":true}"#)).await;
    assert_eq!(st, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(v["error"]["code"], "internal");
    assert_eq!(a.repo.updates(), 0);
    // O estado guardado continua o original.
    let (_, v2) = send(&a.app, get(&format!("/analyses/{id}"))).await;
    assert_eq!(v2["paid"], Value::Null);
}

#[tokio::test]
async fn patch_e_dispute_com_banco_fora_na_leitura_sao_500() {
    let a = amb();
    let id = id_de(&demo(&a, "telecom-2").await);
    a.repo.falha_get.store(true, Ordering::SeqCst);
    let (st, _) = send(&a.app, patch(&id, "{}")).await;
    assert_eq!(st, StatusCode::INTERNAL_SERVER_ERROR);
    let (st, _) = send(&a.app, dispute(&id, r#"{"finding_ids":["TEL-03:i2"]}"#)).await;
    assert_eq!(st, StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn upload_com_banco_fora_depois_de_extrair_e_500() {
    let a = amb_ia();
    a.repo.falha_insert.store(true, Ordering::SeqCst);
    let (st, v) = send(&a.app, multipart(vec![consent("true"), arquivo(PDF)])).await;
    assert_eq!(st, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(v["error"]["code"], "internal");
    assert_eq!(a.ext.as_ref().unwrap().chamadas(), 1);
    assert_eq!(a.repo.inserts(), 0);
}

#[tokio::test]
async fn data_de_expiracao_corrompida_no_banco_vira_500_e_nao_410() {
    let a = amb();
    let id = semear(&a, "telecom-3", "ontem de manhã").await;
    for req in [
        get(&format!("/analyses/{id}")),
        patch(&id, "{}"),
        dispute(&id, r#"{"finding_ids":["x"]}"#),
    ] {
        let (st, v) = send(&a.app, req).await;
        assert_eq!(st, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(v["error"]["code"], "internal");
    }
}

// ───────────────────────────── expiração ─────────────────────────────

#[tokio::test]
async fn expirou_agora_ou_no_passado_remoto_e_410_nas_tres_rotas() {
    let a = amb();
    let agora = Utc::now().to_rfc3339();
    let epoca = "1970-01-01T00:00:00+00:00";
    for exp in [agora.as_str(), epoca] {
        let id = semear(&a, "telecom-2", exp).await;
        let (st, v) = send(&a.app, get(&format!("/analyses/{id}"))).await;
        assert_eq!(st, StatusCode::GONE, "GET {exp}");
        assert_eq!(v["error"]["code"], "expired");
        let (st, v) = send(&a.app, patch(&id, r#"{"paid":true}"#)).await;
        assert_eq!(st, StatusCode::GONE, "PATCH {exp}");
        assert_eq!(v["error"]["code"], "expired");
        let (st, v) = send(&a.app, dispute(&id, r#"{"finding_ids":["TEL-03:i2"]}"#)).await;
        assert_eq!(st, StatusCode::GONE, "dispute {exp}");
        assert_eq!(v["error"]["code"], "expired");
    }
    // PATCH em resultado expirado não grava nada.
    assert_eq!(a.repo.updates(), 0);
}

#[tokio::test]
async fn ainda_valido_por_uma_hora_ou_ate_o_ano_9999_responde_200() {
    let a = amb();
    let em_1h = (Utc::now() + Duration::hours(1)).to_rfc3339();
    for exp in [em_1h.as_str(), "9999-12-31T23:59:59+00:00"] {
        let id = semear(&a, "telecom-3", exp).await;
        let (st, _) = send(&a.app, get(&format!("/analyses/{id}"))).await;
        assert_eq!(st, StatusCode::OK, "GET {exp}");
        let (st, _) = send(&a.app, patch(&id, "{}")).await;
        assert_eq!(st, StatusCode::OK, "PATCH {exp}");
    }
}

#[tokio::test]
async fn expiracao_compara_o_instante_e_nao_o_texto_do_fuso() {
    let a = amb();
    // +1h no futuro, escrito num fuso extremo (+14:00): o texto "parece" bem adiante/atrás.
    let mais14 = FixedOffset::east_opt(14 * 3600).unwrap();
    let menos12 = FixedOffset::west_opt(12 * 3600).unwrap();
    let futuro: DateTime<FixedOffset> = (Utc::now() + Duration::hours(1)).with_timezone(&mais14);
    let passado: DateTime<FixedOffset> = (Utc::now() - Duration::hours(1)).with_timezone(&menos12);
    let id_ok = semear(&a, "telecom-3", &futuro.to_rfc3339()).await;
    let id_exp = semear(&a, "telecom-3", &passado.to_rfc3339()).await;
    assert_eq!(
        send(&a.app, get(&format!("/analyses/{id_ok}"))).await.0,
        StatusCode::OK
    );
    assert_eq!(
        send(&a.app, get(&format!("/analyses/{id_exp}"))).await.0,
        StatusCode::GONE
    );
}

#[tokio::test]
async fn corpo_invalido_tem_prioridade_sobre_resultado_expirado_ou_inexistente() {
    let a = amb();
    let expirado = semear(&a, "telecom-3", "1970-01-01T00:00:00+00:00").await;
    let inexistente = Uuid::new_v4().to_string();
    for id in [expirado.as_str(), inexistente.as_str(), "nao-e-uuid"] {
        let (st, v) = send(&a.app, patch(id, "{isso nao e json")).await;
        assert_eq!(st, StatusCode::BAD_REQUEST, "{id}");
        assert_eq!(v["error"]["code"], "invalid_body");
    }
}

#[tokio::test]
async fn new_analysis_nasce_com_30_dias_exatos_e_estado_inicial_limpo() {
    let antes = Utc::now();
    let a1 = new_analysis(carregar_bill("telecom-1"), AnalysisSource::Upload);
    let a2 = new_analysis(carregar_bill("telecom-1"), AnalysisSource::Upload);
    let depois = Utc::now();
    let criado = DateTime::parse_from_rfc3339(&a1.created_at).unwrap();
    let expira = DateTime::parse_from_rfc3339(&a1.expires_at).unwrap();
    assert!(criado >= antes && criado <= depois);
    assert_eq!(expira - criado, Duration::days(30));
    assert_ne!(a1.id, a2.id);
    let u = Uuid::parse_str(&a1.id).unwrap();
    assert_eq!(u.get_version_num(), 4);
    assert!(a1.answers.is_empty());
    assert_eq!(a1.paid, None);
    assert!(!a1.rules_version.is_empty());
}

// ───────────────────────────── ids, métodos e rotas ─────────────────────────────

#[tokio::test]
async fn uuid_em_maiusculas_e_sem_hifen_reabre_a_mesma_analise() {
    let a = amb();
    let id = id_de(&demo(&a, "telecom-1").await);
    let maiusculo = id.to_uppercase();
    let simples = id.replace('-', "");
    for forma in [maiusculo, simples] {
        let (st, v) = send(&a.app, get(&format!("/analyses/{forma}"))).await;
        assert_eq!(st, StatusCode::OK, "{forma}");
        assert_eq!(v["id"], id.as_str());
    }
}

#[tokio::test]
async fn ids_estranhos_dao_404_em_json_sem_estourar() {
    let a = amb();
    let longo = "a".repeat(5000);
    let ids = [
        "0",
        "%20",
        "00000000-0000-0000-0000-000000000000",
        "an%C3%A1lise",
        "1'%20OR%20'1'='1",
        "..%2F..%2Fetc%2Fpasswd",
        "%00",
        longo.as_str(),
    ];
    for id in ids {
        let (st, v) = send(&a.app, get(&format!("/analyses/{id}"))).await;
        assert_eq!(st, StatusCode::NOT_FOUND, "{id}");
        assert_eq!(v["error"]["code"], "not_found", "{id}");
        let (st, _) = send(&a.app, patch(id, "{}")).await;
        assert_eq!(st, StatusCode::NOT_FOUND, "PATCH {id}");
        let (st, _) = send(&a.app, dispute(id, r#"{"finding_ids":["x"]}"#)).await;
        assert_eq!(st, StatusCode::NOT_FOUND, "dispute {id}");
    }
}

#[tokio::test]
async fn metodos_nao_permitidos_dao_405() {
    let a = amb();
    let id = id_de(&demo(&a, "telecom-1").await);
    let casos = [
        ("DELETE", format!("/analyses/{id}")),
        ("PUT", format!("/analyses/{id}")),
        ("GET", "/analyses".to_string()),
        ("GET", "/analyses/demo/telecom-1".to_string()),
        ("GET", format!("/analyses/{id}/dispute")),
        ("POST", "/health".to_string()),
    ];
    for (m, uri) in casos {
        let (st, _) = send(&a.app, json_req(m, &uri, None, Body::empty())).await;
        assert_eq!(st, StatusCode::METHOD_NOT_ALLOWED, "{m} {uri}");
    }
}

#[tokio::test]
async fn rotas_inexistentes_dao_404() {
    let a = amb();
    for uri in [
        "/",
        "/nope",
        "/analyses/",
        "/analyses/x/y/z",
        "/health/extra",
    ] {
        let (st, _) = send(&a.app, get(uri)).await;
        assert_eq!(st, StatusCode::NOT_FOUND, "{uri}");
    }
}

// ───────────────────────────── demo ─────────────────────────────

#[tokio::test]
async fn demo_com_nome_fora_de_a_z_0_9_hifen_e_404_sem_gravar_nada() {
    let a = amb();
    let longo = "a".repeat(5000);
    let nomes = [
        "telecom_1",
        "telecom-1.json",
        "TELECOM-1",
        "telecom%201",
        "%C3%A9",
        "a%00b",
        "..",
        "%2e%2e",
        "%20",
        "telecom-1%2F",
        "---",
        "inexistente",
        longo.as_str(),
    ];
    for s in nomes {
        let (st, v) = send(&a.app, post_vazio(&format!("/analyses/demo/{s}"))).await;
        assert_eq!(st, StatusCode::NOT_FOUND, "{s}");
        assert_eq!(v["error"]["code"], "not_found", "{s}");
    }
    assert_eq!(a.repo.inserts(), 0);
}

#[tokio::test]
async fn todas_as_fixtures_extraidas_viram_demo_valida_sem_ia() {
    let a = amb();
    for s in [
        "telecom-1",
        "telecom-2",
        "telecom-3",
        "telecom-injecao",
        "energia-1",
        "energia-2",
        "energia-3",
    ] {
        let v = demo(&a, s).await;
        assert_eq!(v["source"], "demo", "{s}");
        assert_eq!(v["answers"], json!({}), "{s}");
        assert_eq!(v["paid"], Value::Null, "{s}");
        assert!(v["recoverable_cents"].as_i64().unwrap() >= 0, "{s}");
    }
    assert_eq!(a.repo.inserts(), 7);
    assert!(a.ext.is_none());
}

#[tokio::test]
async fn demo_repetida_gera_ids_distintos() {
    let a = amb();
    let v1 = demo(&a, "telecom-3").await;
    let v2 = demo(&a, "telecom-3").await;
    assert_ne!(v1["id"], v2["id"]);
    assert_eq!(a.repo.inserts(), 2);
}

#[tokio::test]
async fn demo_com_pasta_de_fixtures_inexistente_e_404() {
    let a = montar(
        None,
        0,
        PathBuf::from("/caminho/que/nao/existe-humanobill"),
        None,
    );
    let (st, v) = send(&a.app, post_vazio("/analyses/demo/telecom-1")).await;
    assert_eq!(st, StatusCode::NOT_FOUND);
    assert_eq!(v["error"]["code"], "not_found");
}

#[tokio::test]
async fn fixture_quebrada_vira_500_e_nao_grava() {
    let tmp = DirTemp::nova();
    let valida = std::fs::read_to_string(fixtures().join("extracted/telecom-3.json")).unwrap();
    tmp.escrever("vazia.json", "");
    tmp.escrever("malformada.json", "{\"kind\": \"telecom\", ");
    tmp.escrever("sem-campos.json", "{}");
    tmp.escrever("nulo.json", "null");
    tmp.escrever("tipo-errado.json", r#"{"kind": 7}"#);
    // JSON bom mas com soma dos itens diferente do total: "inconsistente".
    tmp.escrever(
        "soma-errada.json",
        &valida.replace("\"total_cents\": 13990", "\"total_cents\": 99999"),
    );
    tmp.escrever("sem-itens.json", &{
        let mut v: Value = serde_json::from_str(&valida).unwrap();
        v["items"] = json!([]);
        v.to_string()
    });
    let a = montar(None, 0, tmp.0.clone(), None);
    for s in [
        "vazia",
        "malformada",
        "sem-campos",
        "nulo",
        "tipo-errado",
        "soma-errada",
        "sem-itens",
    ] {
        let (st, v) = send(&a.app, post_vazio(&format!("/analyses/demo/{s}"))).await;
        assert_eq!(st, StatusCode::INTERNAL_SERVER_ERROR, "{s}");
        assert_eq!(v["error"]["code"], "internal", "{s}");
    }
    assert_eq!(a.repo.inserts(), 0);
}

#[tokio::test]
async fn fixture_valida_na_pasta_temporaria_funciona() {
    // Garante que o teste acima falha pelos motivos certos (a pasta e o fluxo funcionam).
    let tmp = DirTemp::nova();
    tmp.escrever(
        "boa.json",
        &std::fs::read_to_string(fixtures().join("extracted/telecom-3.json")).unwrap(),
    );
    let a = montar(None, 0, tmp.0.clone(), None);
    let (st, _) = send(&a.app, post_vazio("/analyses/demo/boa")).await;
    assert_eq!(st, StatusCode::CREATED);
}

// ───────────────────────────── upload: consentimento ─────────────────────────────

#[tokio::test]
async fn consentimento_so_vale_com_o_texto_exato_true() {
    let a = amb_ia();
    let invalidos = [
        "True",
        "TRUE",
        "1",
        "yes",
        "sim",
        "false",
        "",
        " true",
        "true ",
        "true\n",
        "tru\u{200b}e",
        "ｔｒｕｅ",
    ];
    for c in invalidos {
        let (st, v) = send(&a.app, multipart(vec![consent(c), arquivo(PDF)])).await;
        assert_eq!(st, StatusCode::BAD_REQUEST, "consent {c:?}");
        assert_eq!(v["error"]["code"], "consent_required", "consent {c:?}");
    }
    // Sem consentimento a IA nunca é chamada e nada é gravado.
    assert_eq!(a.ext.as_ref().unwrap().chamadas(), 0);
    assert_eq!(a.repo.inserts(), 0);
}

#[tokio::test]
async fn consentimento_repetido_vale_o_ultimo_valor() {
    let a = amb_ia();
    let (st, _) = send(
        &a.app,
        multipart(vec![consent("false"), consent("true"), arquivo(PDF)]),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED);
    let (st, v) = send(
        &a.app,
        multipart(vec![consent("true"), consent("false"), arquivo(PDF)]),
    )
    .await;
    assert_eq!(st, StatusCode::BAD_REQUEST);
    assert_eq!(v["error"]["code"], "consent_required");
}

#[tokio::test]
async fn ordem_dos_campos_nao_importa_e_campos_extras_sao_ignorados() {
    let a = amb_ia();
    let (st, _) = send(
        &a.app,
        multipart(vec![
            arquivo(PDF),
            parte("observacao", "qualquer coisa ✓"),
            consent("true"),
        ]),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED);
}

#[tokio::test]
async fn nome_do_campo_diferencia_maiusculas() {
    let a = amb_ia();
    let mut f = arquivo(PDF);
    f.nome = "File".into();
    let (st, v) = send(&a.app, multipart(vec![consent("true"), f])).await;
    assert_eq!(st, StatusCode::BAD_REQUEST);
    assert_eq!(v["error"]["code"], "invalid_file");
    let mut c = consent("true");
    c.nome = "Consent".into();
    let (st, v) = send(&a.app, multipart(vec![c, arquivo(PDF)])).await;
    assert_eq!(st, StatusCode::BAD_REQUEST);
    assert_eq!(v["error"]["code"], "consent_required");
}

#[tokio::test]
async fn upload_sem_campos_ou_sem_arquivo() {
    let a = amb_ia();
    // Nada enviado: o consentimento é checado primeiro.
    let (st, v) = send(&a.app, multipart(vec![])).await;
    assert_eq!(st, StatusCode::BAD_REQUEST);
    assert_eq!(v["error"]["code"], "consent_required");
    // Consentiu mas não mandou arquivo.
    let (st, v) = send(&a.app, multipart(vec![consent("true")])).await;
    assert_eq!(st, StatusCode::BAD_REQUEST);
    assert_eq!(v["error"]["code"], "invalid_file");
    assert_eq!(a.ext.as_ref().unwrap().chamadas(), 0);
}

#[tokio::test]
async fn upload_sem_consentimento_tem_prioridade_sobre_arquivo_invalido() {
    let a = amb_ia();
    let (st, v) = send(
        &a.app,
        multipart(vec![consent("false"), arquivo(b"PK\x03\x04zip")]),
    )
    .await;
    assert_eq!(st, StatusCode::BAD_REQUEST);
    assert_eq!(v["error"]["code"], "consent_required");
}

// ───────────────────────────── upload: arquivo ─────────────────────────────

#[tokio::test]
async fn arquivo_vazio_e_400_invalid_file() {
    let a = amb_ia();
    let (st, v) = send(&a.app, multipart(vec![consent("true"), arquivo(b"")])).await;
    assert_eq!(st, StatusCode::BAD_REQUEST);
    assert_eq!(v["error"]["code"], "invalid_file");
}

#[tokio::test]
async fn arquivos_de_tipo_nao_suportado_sao_415_e_nao_chamam_a_ia() {
    let a = amb_ia();
    let casos: Vec<(&str, Vec<u8>)> = vec![
        ("gif", b"GIF89a\x01\x00\x01\x00".to_vec()),
        ("texto", "olá, mundo".as_bytes().to_vec()),
        ("html", b"<html><body>oi</body></html>".to_vec()),
        ("1 byte", vec![0x25]),
        ("zeros", vec![0u8; 64]),
        ("exe", b"MZ\x90\x00\x03\x00\x00\x00".to_vec()),
    ];
    for (nome, bytes) in casos {
        let (st, v) = send(&a.app, multipart(vec![consent("true"), arquivo(&bytes)])).await;
        assert_eq!(st, StatusCode::UNSUPPORTED_MEDIA_TYPE, "{nome}");
        assert_eq!(v["error"]["code"], "invalid_file", "{nome}");
    }
    assert_eq!(a.ext.as_ref().unwrap().chamadas(), 0);
}

#[tokio::test]
async fn tipo_vem_dos_magic_bytes_e_nao_do_content_type_nem_do_nome() {
    let a = amb_ia();
    let jpeg = [
        0xFF, 0xD8, 0xFF, 0xE0, 0, 0x10, b'J', b'F', b'I', b'F', 0, 1,
    ];
    let webp = b"RIFF\x24\x00\x00\x00WEBPVP8 ".to_vec();
    let mut png = arquivo(&PNG);
    png.arquivo = Some("fatura.pdf".into());
    png.tipo = Some("application/pdf".into()); // mente: é PNG
    let mut pdf = arquivo(PDF);
    pdf.arquivo = Some("virus.exe".into());
    pdf.tipo = Some("text/plain".into()); // mente: é PDF
    let mut jpg = arquivo(&jpeg);
    jpg.tipo = None;
    let mut wp = arquivo(&webp);
    wp.arquivo = None;
    for p in [png, pdf, jpg, wp] {
        let (st, _) = send(&a.app, multipart(vec![consent("true"), p])).await;
        assert_eq!(st, StatusCode::CREATED);
    }
    let mimes = a.ext.as_ref().unwrap().mimes.lock().unwrap().clone();
    assert_eq!(
        mimes,
        vec![
            InputMime::Png,
            InputMime::Pdf,
            InputMime::Jpeg,
            InputMime::Webp
        ]
    );
}

#[tokio::test]
async fn arquivo_no_limite_exato_de_10mb_passa_e_um_byte_a_mais_e_413() {
    let a = amb_ia();
    let (st, _) = send(
        &a.app,
        multipart(vec![consent("true"), arquivo(&pdf_com_tamanho(MAX_BYTES))]),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "exatamente MAX_BYTES");
    let (st, v) = send(
        &a.app,
        multipart(vec![
            consent("true"),
            arquivo(&pdf_com_tamanho(MAX_BYTES + 1)),
        ]),
    )
    .await;
    assert_eq!(st, StatusCode::PAYLOAD_TOO_LARGE, "MAX_BYTES + 1");
    assert_eq!(v["error"]["code"], "too_large");
    assert_eq!(a.ext.as_ref().unwrap().chamadas(), 1);
}

#[tokio::test]
async fn corpo_acima_do_limite_do_servidor_e_rejeitado_sem_chamar_a_ia() {
    let a = amb_ia();
    // MAX_BYTES + 64 KiB é o teto do corpo; passar disso corta a leitura no meio.
    let (st, v) = send(
        &a.app,
        multipart(vec![
            consent("true"),
            arquivo(&pdf_com_tamanho(MAX_BYTES + 200 * 1024)),
        ]),
    )
    .await;
    assert!(st.is_client_error(), "status {st}");
    assert!(v["error"]["code"].is_string(), "erro deveria ser JSON: {v}");
    assert_eq!(a.ext.as_ref().unwrap().chamadas(), 0);
    assert_eq!(a.repo.inserts(), 0);
}

#[tokio::test]
async fn corpo_acima_do_limite_do_servidor_deveria_ser_413_too_large() {
    let a = amb_ia();
    let (st, v) = send(
        &a.app,
        multipart(vec![
            consent("true"),
            arquivo(&pdf_com_tamanho(MAX_BYTES + 200 * 1024)),
        ]),
    )
    .await;
    assert_eq!(st, StatusCode::PAYLOAD_TOO_LARGE);
    assert_eq!(v["error"]["code"], "too_large");
}

#[tokio::test]
async fn pdf_com_6_paginas_e_400_pela_rota() {
    let a = amb_ia();
    let mut pdf = b"%PDF-1.7\n".to_vec();
    for _ in 0..6 {
        pdf.extend_from_slice(b"1 0 obj << /Type /Page >> endobj\n");
    }
    let (st, v) = send(&a.app, multipart(vec![consent("true"), arquivo(&pdf)])).await;
    assert_eq!(st, StatusCode::BAD_REQUEST);
    assert_eq!(v["error"]["code"], "invalid_file");
    assert_eq!(a.ext.as_ref().unwrap().chamadas(), 0);
}

/// Multipart de consentimento + PDF cortado 30 bytes antes do fim (sem boundary final).
async fn multipart_truncado() -> Request<Body> {
    let (_, body) = multipart(vec![consent("true"), arquivo(PDF)]).into_parts();
    let bytes = body.collect().await.unwrap().to_bytes();
    json_req(
        "POST",
        "/analyses",
        Some("multipart/form-data; boundary=XBORDAX"),
        bytes[..bytes.len() - 30].to_vec(),
    )
}

#[tokio::test]
async fn multipart_truncado_e_erro_de_cliente_em_json_e_nao_chama_a_ia() {
    let a = amb_ia();
    let (st, v) = send(&a.app, multipart_truncado().await).await;
    assert!(st.is_client_error(), "status {st}");
    assert!(v["error"]["code"].is_string(), "erro deveria ser JSON: {v}");
    assert_eq!(a.ext.as_ref().unwrap().chamadas(), 0);
    assert_eq!(a.repo.inserts(), 0);
}

#[tokio::test]
async fn multipart_truncado_deveria_ser_400_invalid_file() {
    let a = amb_ia();
    let (st, v) = send(&a.app, multipart_truncado().await).await;
    assert_eq!(st, StatusCode::BAD_REQUEST);
    assert_eq!(v["error"]["code"], "invalid_file");
}

#[tokio::test]
async fn upload_com_content_type_que_nao_e_multipart_responde_erro_4xx() {
    let a = amb_ia();
    let casos = [
        Some("application/json"),
        Some("text/plain"),
        Some("multipart/form-data"), // sem boundary
        None,
    ];
    for ct in casos {
        let (st, _, _) = send_full(
            &a.app,
            json_req("POST", "/analyses", ct, r#"{"consent":true}"#.to_string()),
        )
        .await;
        assert!(st.is_client_error(), "{ct:?} -> {st}");
    }
    assert_eq!(a.ext.as_ref().unwrap().chamadas(), 0);
}

// docs/04: todo erro da API segue o formato {"error": {"code", "message"}}.
#[tokio::test]
async fn upload_sem_multipart_tambem_responde_no_formato_json_de_erro() {
    let a = amb_ia();
    let (st, headers, bytes) = send_full(
        &a.app,
        json_req(
            "POST",
            "/analyses",
            Some("application/json"),
            "{}".to_string(),
        ),
    )
    .await;
    assert!(st.is_client_error());
    let ct = headers
        .get(header::CONTENT_TYPE)
        .map(|v| v.to_str().unwrap().to_string())
        .unwrap_or_default();
    assert!(ct.starts_with("application/json"), "content-type: {ct}");
    let v: Value = serde_json::from_slice(&bytes).expect("corpo deveria ser JSON");
    assert!(v["error"]["code"].is_string());
}

// ───────────────────────────── upload: extrator (Gemini) ─────────────────────────────

#[tokio::test]
async fn erros_do_extrator_viram_status_e_codigos_estaveis_sem_vazar_detalhe() {
    type Caso = (&'static str, fn() -> ExtractError, StatusCode, &'static str);
    let casos: Vec<Caso> = vec![
        (
            "indisponível",
            || ExtractError::Unavailable("chave=SEGREDO-123".into()),
            StatusCode::SERVICE_UNAVAILABLE,
            "extraction_unavailable",
        ),
        (
            "quota",
            || ExtractError::RateLimited,
            StatusCode::TOO_MANY_REQUESTS,
            "rate_limited",
        ),
        (
            "provedor (503 do Google)",
            || ExtractError::Provider("HTTP 503 model overloaded; key=SEGREDO-123".into()),
            StatusCode::UNPROCESSABLE_ENTITY,
            "extraction_failed",
        ),
        (
            "resposta inválida sem detalhes",
            || ExtractError::Invalid { issues: vec![] },
            StatusCode::UNPROCESSABLE_ENTITY,
            "extraction_failed",
        ),
        (
            "resposta inválida com detalhes",
            || ExtractError::Invalid {
                issues: vec![ValidationIssue::NoItems, ValidationIssue::NegativeTotal],
            },
            StatusCode::UNPROCESSABLE_ENTITY,
            "extraction_failed",
        ),
    ];
    for (nome, f, status, code) in casos {
        let a = amb_ia_com(Box::new(move |_| Err(f())));
        let (st, _, bytes) =
            send_full(&a.app, multipart(vec![consent("true"), arquivo(PDF)])).await;
        assert_eq!(st, status, "{nome}");
        let txt = String::from_utf8(bytes).unwrap();
        let v: Value = serde_json::from_str(&txt).unwrap();
        assert_eq!(v["error"]["code"], code, "{nome}");
        assert!(
            !txt.contains("SEGREDO"),
            "{nome}: vazou detalhe do provedor"
        );
        assert!(
            !txt.contains("overloaded"),
            "{nome}: vazou detalhe do provedor"
        );
        // A API não repete a chamada sozinha (o retry é do extrator) e não grava nada.
        assert_eq!(a.ext.as_ref().unwrap().chamadas(), 1, "{nome}");
        assert_eq!(a.repo.inserts(), 0, "{nome}");
    }
}

#[tokio::test]
async fn api_se_recupera_depois_de_uma_falha_do_extrator() {
    // 1ª chamada: Google sobrecarregado; 2ª e 3ª: ok.
    let a = amb_ia_com(Box::new(|n| {
        if n == 0 {
            Err(ExtractError::Provider("503".into()))
        } else {
            Ok(carregar_bill("telecom-1"))
        }
    }));
    let (st, _) = send(&a.app, multipart(vec![consent("true"), arquivo(PDF)])).await;
    assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY);
    for _ in 0..2 {
        let (st, v) = send(&a.app, multipart(vec![consent("true"), arquivo(PDF)])).await;
        assert_eq!(st, StatusCode::CREATED);
        assert_eq!(v["source"], "upload");
    }
    assert_eq!(a.repo.inserts(), 2);
}

#[tokio::test]
async fn demo_continua_funcionando_com_a_ia_fora_do_ar() {
    let a = amb_ia_com(Box::new(|_| Err(ExtractError::Unavailable("fora".into()))));
    let (st, _) = send(&a.app, multipart(vec![consent("true"), arquivo(PDF)])).await;
    assert_eq!(st, StatusCode::SERVICE_UNAVAILABLE);
    demo(&a, "telecom-1").await;
    demo(&a, "energia-1").await;
}

#[tokio::test]
async fn upload_de_fatura_de_energia_cria_analise_de_energia() {
    let a = amb_ia_com(Box::new(|_| Ok(carregar_bill("energia-1"))));
    let (st, v) = send(&a.app, multipart(vec![consent("true"), arquivo(PDF)])).await;
    assert_eq!(st, StatusCode::CREATED);
    assert_eq!(v["bill"]["kind"], "energy");
    assert_eq!(v["source"], "upload");
}

#[tokio::test]
async fn resposta_do_upload_nao_carrega_o_conteudo_do_arquivo_enviado() {
    let a = amb_ia();
    let marcador = "MARCADOR-UNICO-DO-ARQUIVO-777";
    let mut pdf = PDF.to_vec();
    pdf.extend_from_slice(marcador.as_bytes());
    let (st, _, bytes) = send_full(&a.app, multipart(vec![consent("true"), arquivo(&pdf)])).await;
    assert_eq!(st, StatusCode::CREATED);
    assert!(!String::from_utf8_lossy(&bytes).contains(marcador));
}

#[tokio::test]
async fn vinte_uploads_simultaneos_com_ia_lenta_geram_vinte_analises_distintas() {
    let a = montar(
        Some(Box::new(|_| Ok(carregar_bill("telecom-1")))),
        30,
        fixtures(),
        None,
    );
    let mut tarefas = Vec::new();
    for _ in 0..20 {
        let app = a.app.clone();
        tarefas.push(tokio::spawn(async move {
            send(&app, multipart(vec![consent("true"), arquivo(PDF)])).await
        }));
    }
    let mut ids = std::collections::BTreeSet::new();
    for t in tarefas {
        let (st, v) = t.await.unwrap();
        assert_eq!(st, StatusCode::CREATED);
        ids.insert(id_de(&v));
    }
    assert_eq!(ids.len(), 20);
    assert_eq!(a.repo.inserts(), 20);
    assert_eq!(a.ext.as_ref().unwrap().chamadas(), 20);
}

// ───────────────────────────── PATCH: corpo e respostas ─────────────────────────────

#[tokio::test]
async fn patch_com_corpo_malformado_ou_de_tipo_errado_e_400_invalid_body() {
    let a = amb();
    let id = id_de(&demo(&a, "telecom-1").await);
    let corpos = [
        "",
        "null",
        "42",
        "\"texto\"",
        "{",
        "{\"paid\":}",
        r#"{"paid":"true"}"#,
        r#"{"paid":1}"#,
        r#"{"paid":[true]}"#,
        r#"{"answers":[]}"#,
        r#"{"answers":"x"}"#,
        r#"{"answers":{"TEL-01:i2":{"type":"talvez"}}}"#,
        r#"{"answers":{"TEL-01:i2":{"type":"amount"}}}"#,
        r#"{"answers":{"TEL-01:i2":{"type":"amount","value":"12"}}}"#,
        r#"{"answers":{"TEL-01:i2":{"type":"amount","value":1.5}}}"#,
        r#"{"answers":{"TEL-01:i2":{"type":"amount","value":1e30}}}"#,
        r#"{"answers":{"TEL-01:i2":{"type":"date","value":"31/02/2026"}}}"#,
        r#"{"answers":{"TEL-01:i2":{"type":"date","value":"2026-02-30"}}}"#,
        r#"{"answers":{"TEL-01:i2":"yes"}}"#,
        r#"{"bill":null,"extra":1}"#,
        r#"{"bill":{}}"#,
        r#"{"bill":"x"}"#,
    ];
    for c in corpos {
        let (st, v) = send(&a.app, patch(&id, c)).await;
        assert_eq!(st, StatusCode::BAD_REQUEST, "corpo: {c}");
        assert_eq!(v["error"]["code"], "invalid_body", "corpo: {c}");
    }
    assert_eq!(a.repo.updates(), 0);
}

#[tokio::test]
async fn patch_com_bytes_que_nao_sao_utf8_ou_sem_content_type_e_400() {
    let a = amb();
    let id = id_de(&demo(&a, "telecom-1").await);
    let uri = format!("/analyses/{id}");
    let casos = [
        json_req(
            "PATCH",
            &uri,
            Some("application/json"),
            vec![0xFF, 0xFE, b'{', b'}'],
        ),
        json_req("PATCH", &uri, None, "{}".to_string()),
        json_req("PATCH", &uri, Some("text/plain"), "{}".to_string()),
        json_req("PATCH", &uri, Some("application/xml"), "<a/>".to_string()),
    ];
    for req in casos {
        let (st, v) = send(&a.app, req).await;
        assert_eq!(st, StatusCode::BAD_REQUEST);
        assert_eq!(v["error"]["code"], "invalid_body");
    }
}

#[tokio::test]
async fn patch_com_corpo_gigante_e_rejeitado_como_erro_de_cliente() {
    let a = amb();
    let id = id_de(&demo(&a, "telecom-1").await);
    // Acima do limite padrão do Axum (2 MiB).
    let grande = format!(
        r#"{{"answers":{{"{}":{{"type":"yes"}}}}}}"#,
        "x".repeat(3_000_000)
    );
    let (st, v) = send(&a.app, patch(&id, &grande)).await;
    assert!(st.is_client_error(), "status {st}");
    assert_eq!(v["error"]["code"], "invalid_body");
    assert_eq!(a.repo.updates(), 0);
}

#[tokio::test]
async fn patch_vazio_e_idempotente_e_nao_muda_o_resultado() {
    let a = amb();
    let v0 = demo(&a, "telecom-2").await;
    let id = id_de(&v0);
    for _ in 0..3 {
        let (st, v) = send(&a.app, patch(&id, "{}")).await;
        assert_eq!(st, StatusCode::OK);
        assert_eq!(v["findings"], v0["findings"]);
        assert_eq!(v["recoverable_cents"], v0["recoverable_cents"]);
        assert_eq!(v["bill"], v0["bill"]);
    }
}

#[tokio::test]
async fn answers_null_mantem_e_answers_vazio_limpa_as_respostas() {
    let a = amb();
    let id = id_de(&demo(&a, "telecom-1").await);
    let (_, v) = send(
        &a.app,
        patch(&id, r#"{"answers":{"TEL-01:i2":{"type":"no"}}}"#),
    )
    .await;
    assert_eq!(v["recoverable_cents"], 1290);
    let (st, v) = send(&a.app, patch(&id, r#"{"answers":null}"#)).await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(
        v["recoverable_cents"], 1290,
        "null não deve apagar respostas"
    );
    let (st, v) = send(&a.app, patch(&id, r#"{"answers":{}}"#)).await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(v["recoverable_cents"], 0, "{{}} deve limpar as respostas");
    assert_eq!(v["answers"], json!({}));
}

#[tokio::test]
async fn paid_ausente_mantem_e_null_volta_para_nao_informado() {
    let a = amb();
    let id = id_de(&demo(&a, "telecom-2").await);
    let (_, v) = send(&a.app, patch(&id, r#"{"paid":true}"#)).await;
    assert_eq!(v["paid"], true);
    let (_, v) = send(&a.app, patch(&id, "{}")).await;
    assert_eq!(v["paid"], true, "campo ausente não pode zerar o paid");
    let (_, v) = send(&a.app, patch(&id, r#"{"paid":null}"#)).await;
    assert_eq!(v["paid"], Value::Null);
}

#[tokio::test]
async fn paid_true_com_nada_a_recuperar_continua_zero() {
    let a = amb();
    let id = id_de(&demo(&a, "telecom-3").await);
    let (st, v) = send(&a.app, patch(&id, r#"{"paid":true}"#)).await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(v["recoverable_cents"], 0);
}

#[tokio::test]
async fn mesma_resposta_enviada_duas_vezes_continua_valida() {
    // A pergunta some da lista depois de respondida, mas segue aceita (idempotência).
    let a = amb();
    let id = id_de(&demo(&a, "telecom-1").await);
    let corpo = r#"{"answers":{"TEL-01:i2":{"type":"no"}}}"#;
    let (st1, v1) = send(&a.app, patch(&id, corpo)).await;
    let (st2, v2) = send(&a.app, patch(&id, corpo)).await;
    assert_eq!((st1, st2), (StatusCode::OK, StatusCode::OK));
    assert_eq!(v1["recoverable_cents"], v2["recoverable_cents"]);
}

#[tokio::test]
async fn ids_de_pergunta_quase_certos_sao_400_e_nada_e_gravado() {
    let a = amb();
    let id = id_de(&demo(&a, "telecom-1").await);
    let ids = [
        "",
        "tel-01:i2",
        "TEL-01:i2 ",
        " TEL-01:i2",
        "TEL-01:í2",
        "TEL-01:i2\n",
        "TEL-01:",
    ];
    for q in ids {
        let corpo = json!({"answers": {q: {"type": "no"}}});
        let (st, v) = send(&a.app, patch_json(&id, &corpo)).await;
        assert_eq!(st, StatusCode::BAD_REQUEST, "{q:?}");
        assert_eq!(v["error"]["code"], "unknown_question", "{q:?}");
    }
    // Um id bom misturado com um ruim derruba o pedido inteiro.
    let corpo = json!({"answers": {"TEL-01:i2": {"type": "no"}, "XYZ": {"type": "no"}}});
    let (st, _) = send(&a.app, patch_json(&id, &corpo)).await;
    assert_eq!(st, StatusCode::BAD_REQUEST);
    assert_eq!(a.repo.updates(), 0);
    let (_, v) = send(&a.app, get(&format!("/analyses/{id}"))).await;
    assert_eq!(v["recoverable_cents"], 0);
}

#[tokio::test]
async fn valores_extremos_na_resposta_de_valor_nao_derrubam_o_handler() {
    let a = amb();
    let id = id_de(&demo(&a, "telecom-2").await);
    for valor in [0_i64, -1, 1, i64::MAX, i64::MIN] {
        let corpo =
            json!({"answers": {"TEL-05:plano": {"type": "amount", "value": valor}}, "paid": true});
        let r = send_sem_panico(&a.app, patch_json(&id, &corpo)).await;
        let (st, v) = r.unwrap_or_else(|| panic!("handler deu panic com valor {valor}"));
        assert_eq!(st, StatusCode::OK, "valor {valor}");
        assert!(
            v["recoverable_cents"].as_i64().unwrap() >= 0,
            "valor {valor}"
        );
    }
}

// ───────────────────────────── PATCH: correção da fatura (RF05) ─────────────────────────────

/// Pega a fatura da demo, aplica `f` e devolve (status, resposta) do PATCH.
async fn corrigir(a: &Amb, fixture: &str, f: impl FnOnce(&mut Value)) -> (StatusCode, Value) {
    let v = demo(a, fixture).await;
    let id = id_de(&v);
    let mut bill = v["bill"].clone();
    f(&mut bill);
    send(&a.app, patch_json(&id, &json!({ "bill": bill }))).await
}

#[tokio::test]
async fn correcao_com_total_negativo_e_400() {
    let a = amb();
    let (st, v) = corrigir(&a, "telecom-3", |b| b["total_cents"] = json!(-1)).await;
    assert_eq!(st, StatusCode::BAD_REQUEST);
    assert_eq!(v["error"]["code"], "invalid_correction");
}

#[tokio::test]
async fn correcao_com_mes_de_referencia_invalido_e_400() {
    let a = amb();
    for mes in [
        "2026-13",
        "2026-00",
        "26-10",
        "2026/10",
        "",
        "2026-1",
        "２０２６-１０",
        "2026-10-01",
        " 2026-10",
    ] {
        let (st, v) = corrigir(&a, "telecom-3", |b| b["reference_month"] = json!(mes)).await;
        assert_eq!(st, StatusCode::BAD_REQUEST, "mês {mes:?}");
        assert_eq!(v["error"]["code"], "invalid_correction", "mês {mes:?}");
    }
    // Bordas válidas.
    for mes in ["2026-01", "2026-12", "0001-01", "9999-12"] {
        let (st, _) = corrigir(&a, "telecom-3", |b| b["reference_month"] = json!(mes)).await;
        assert_eq!(st, StatusCode::OK, "mês {mes:?}");
    }
}

#[tokio::test]
async fn correcao_com_confianca_fora_de_0_a_1_e_400_e_nas_bordas_e_200() {
    let a = amb();
    for c in [-0.01, 1.01, 100.0, -1.0] {
        let (st, _) = corrigir(&a, "telecom-3", |b| b["extraction_confidence"] = json!(c)).await;
        assert_eq!(st, StatusCode::BAD_REQUEST, "confiança {c}");
    }
    for c in [0.0, 1.0, 0.5] {
        let (st, _) = corrigir(&a, "telecom-3", |b| b["extraction_confidence"] = json!(c)).await;
        assert_eq!(st, StatusCode::OK, "confiança {c}");
    }
}

#[tokio::test]
async fn correcao_com_ids_de_item_duplicados_e_400() {
    let a = amb();
    let (st, v) = corrigir(&a, "telecom-3", |b| b["items"][1]["id"] = json!("i1")).await;
    assert_eq!(st, StatusCode::BAD_REQUEST);
    assert_eq!(v["error"]["code"], "invalid_correction");
}

#[tokio::test]
async fn correcao_de_energia_com_leituras_invertidas_e_400() {
    let a = amb();
    let (st, v) = corrigir(&a, "energia-1", |b| {
        let ant = b["energy"]["reading_prev_date"].clone();
        let atu = b["energy"]["reading_curr_date"].clone();
        b["energy"]["reading_prev_date"] = atu;
        b["energy"]["reading_curr_date"] = ant;
    })
    .await;
    assert_eq!(st, StatusCode::BAD_REQUEST);
    assert_eq!(v["error"]["code"], "invalid_correction");
}

#[tokio::test]
async fn correcao_trocando_o_tipo_nos_dois_sentidos_e_400() {
    let a = amb();
    let (st, v) = corrigir(&a, "energia-1", |b| b["kind"] = json!("telecom")).await;
    assert_eq!(st, StatusCode::BAD_REQUEST);
    assert_eq!(v["error"]["code"], "invalid_correction");
    let (st, v) = corrigir(&a, "telecom-3", |b| b["kind"] = json!("energy")).await;
    assert_eq!(st, StatusCode::BAD_REQUEST);
    assert_eq!(v["error"]["code"], "invalid_correction");
}

#[tokio::test]
async fn correcao_com_item_negativo_e_zero_nao_derruba_o_handler() {
    // Crédito/desconto na fatura é legítimo (valor negativo); tudo zerado também.
    let a = amb();
    let v = demo(&a, "telecom-3").await;
    let id = id_de(&v);
    let mut bill = v["bill"].clone();
    bill["items"][0]["amount_cents"] = json!(-500);
    bill["items"][1]["amount_cents"] = json!(500);
    bill["total_cents"] = json!(0);
    let r = send_sem_panico(&a.app, patch_json(&id, &json!({ "bill": bill }))).await;
    let (st, _) = r.expect("handler deu panic com item negativo");
    assert_eq!(st, StatusCode::OK);

    let mut zerada = v["bill"].clone();
    for i in zerada["items"].as_array_mut().unwrap() {
        i["amount_cents"] = json!(0);
    }
    zerada["total_cents"] = json!(0);
    let r = send_sem_panico(&a.app, patch_json(&id, &json!({ "bill": zerada }))).await;
    let (st, v) = r.expect("handler deu panic com fatura zerada");
    assert_eq!(st, StatusCode::OK);
    assert_eq!(v["recoverable_cents"], 0);
}

#[tokio::test]
async fn correcao_com_unicode_e_textos_longos_volta_igual_no_get() {
    let a = amb();
    let v = demo(&a, "telecom-3").await;
    let id = id_de(&v);
    let nome = "Vivaz Telecom 🚀 ção — “aspas” \t ñ 日本語";
    let longo = "ã".repeat(100_000);
    let mut bill = v["bill"].clone();
    bill["provider_name"] = json!(nome);
    bill["items"][0]["description"] = json!(longo);
    let (st, _) = send(&a.app, patch_json(&id, &json!({ "bill": bill }))).await;
    assert_eq!(st, StatusCode::OK);
    let (_, v2) = send(&a.app, get(&format!("/analyses/{id}"))).await;
    assert_eq!(v2["bill"]["provider_name"], nome);
    assert_eq!(v2["bill"]["items"][0]["description"], longo.as_str());
}

#[tokio::test]
async fn correcao_com_500_itens_e_aceita() {
    let a = amb();
    let v = demo(&a, "telecom-3").await;
    let id = id_de(&v);
    let modelo = v["bill"]["items"][0].clone();
    let itens: Vec<Value> = (0..500)
        .map(|n| {
            let mut i = modelo.clone();
            i["id"] = json!(format!("x{n}"));
            i["amount_cents"] = json!(10);
            i
        })
        .collect();
    let mut bill = v["bill"].clone();
    bill["items"] = json!(itens);
    bill["total_cents"] = json!(5000);
    let r = send_sem_panico(&a.app, patch_json(&id, &json!({ "bill": bill }))).await;
    let (st, v) = r.expect("handler deu panic com 500 itens");
    assert_eq!(st, StatusCode::OK);
    assert_eq!(v["bill"]["items"].as_array().unwrap().len(), 500);
}

#[tokio::test]
async fn correcao_e_respostas_no_mesmo_pedido_funcionam() {
    let a = amb();
    let v = demo(&a, "telecom-1").await;
    let id = id_de(&v);
    let corpo = json!({
        "answers": {"TEL-01:i2": {"type": "no"}},
        "paid": true,
        "bill": v["bill"].clone(),
    });
    let (st, v2) = send(&a.app, patch_json(&id, &corpo)).await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(v2["recoverable_cents"], 2580);
    assert_eq!(v2["paid"], true);
}

// Valores monetários enormes não podem derrubar o handler (overflow de i64).
#[tokio::test]
async fn correcao_com_valores_que_estouram_i64_na_soma_nao_causa_panic() {
    let a = amb();
    let v = demo(&a, "telecom-1").await;
    let id = id_de(&v);
    let mut bill = v["bill"].clone();
    for i in bill["items"].as_array_mut().unwrap() {
        i["amount_cents"] = json!(i64::MAX);
    }
    bill["total_cents"] = json!(i64::MAX);
    let r = send_sem_panico(&a.app, patch_json(&id, &json!({ "bill": bill }))).await;
    let (st, _) = r.expect("handler deu panic: soma dos itens estourou i64 em validate_bill");
    assert!(st.is_client_error() || st.is_success(), "status {st}");
}

// A contestação de achados com valores enormes não pode estourar i64.
#[tokio::test]
async fn contestacao_de_valores_enormes_pagos_em_dobro_nao_causa_panic() {
    let a = amb();
    let v = demo(&a, "telecom-1").await;
    let id = id_de(&v);
    // Dois serviços adicionais de 4e18 centavos (a soma ainda cabe em i64).
    let enorme: i64 = 4_000_000_000_000_000_000;
    let mut bill = v["bill"].clone();
    let itens = bill["items"].as_array_mut().unwrap();
    itens.truncate(3);
    for (n, i) in itens.iter_mut().enumerate() {
        i["category"] = json!(if n == 0 { "plano" } else { "servico_adicional" });
        i["amount_cents"] = json!(if n == 0 { 100 } else { enorme });
    }
    bill["total_cents"] = json!(100 + 2 * enorme);
    let (st, _) = send(&a.app, patch_json(&id, &json!({ "bill": bill }))).await;
    assert_eq!(st, StatusCode::OK);
    let (st, v) = send(
        &a.app,
        patch(
            &id,
            r#"{"answers":{"TEL-01:i2":{"type":"no"},"TEL-01:i3":{"type":"no"}},"paid":true}"#,
        ),
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    let ids: Vec<&str> = achados(&v)
        .iter()
        .filter(|f| f["severity"] == "indevido")
        .map(|f| f["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids.len(), 2, "esperava 2 achados indevidos: {v}");
    let r = send_sem_panico(
        &a.app,
        dispute(&id, &json!({"finding_ids": ids}).to_string()),
    )
    .await;
    let (st, _) = r.expect("handler deu panic: total*2 da contestação estourou i64");
    assert!(st.is_success() || st.is_client_error(), "status {st}");
}

// ───────────────────────────── contestação ─────────────────────────────

/// Ids dos achados "indevido" de uma demo.
fn ids_indevidos(v: &Value) -> Vec<String> {
    achados(v)
        .iter()
        .filter(|f| f["severity"] == "indevido")
        .map(|f| f["id"].as_str().unwrap().to_string())
        .collect()
}

#[tokio::test]
async fn protocolo_no_limite_de_40_caracteres_passa_e_41_nao() {
    let a = amb();
    let v = demo(&a, "telecom-2").await;
    let id = id_de(&v);
    let ids = ids_indevidos(&v);
    let p40 = "A".repeat(40);
    let p41 = "A".repeat(41);
    let (st, d) = send(
        &a.app,
        dispute(
            &id,
            &json!({"finding_ids": ids, "protocol": p40}).to_string(),
        ),
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert!(d["text"].as_str().unwrap().contains(&p40));
    let (st, e) = send(
        &a.app,
        dispute(
            &id,
            &json!({"finding_ids": ids, "protocol": p41}).to_string(),
        ),
    )
    .await;
    assert_eq!(st, StatusCode::BAD_REQUEST);
    assert_eq!(e["error"]["code"], "invalid_protocol");
}

#[tokio::test]
async fn protocolo_com_caracteres_fora_da_lista_e_400() {
    let a = amb();
    let v = demo(&a, "telecom-2").await;
    let id = id_de(&v);
    let ids = ids_indevidos(&v);
    let ruins = [
        "protocolo-ção",
        "123\n456",
        "123\t456",
        "123\r\n",
        "'; DROP TABLE analyses; --",
        "<b>x</b>",
        "{{7*7}}",
        "%00",
        "a\u{0}b",
        "ｆｕｌｌ",
        "🙂",
        "123;456",
        "123,456",
    ];
    for p in ruins {
        let corpo = json!({"finding_ids": ids, "protocol": p});
        let (st, e) = send(&a.app, dispute(&id, &corpo.to_string())).await;
        assert_eq!(st, StatusCode::BAD_REQUEST, "protocolo {p:?}");
        assert_eq!(e["error"]["code"], "invalid_protocol", "protocolo {p:?}");
    }
}

#[tokio::test]
async fn protocolo_com_pontuacao_permitida_ou_nulo_ou_vazio_funciona() {
    let a = amb();
    let v = demo(&a, "telecom-2").await;
    let id = id_de(&v);
    let ids = ids_indevidos(&v);
    for p in [
        json!("2026/11-10.A 1"),
        json!(null),
        json!(""),
        json!("   "),
        json!("0"),
    ] {
        let corpo = json!({"finding_ids": ids, "protocol": p});
        let r = send_sem_panico(&a.app, dispute(&id, &corpo.to_string())).await;
        let (st, d) = r.unwrap_or_else(|| panic!("panic com protocolo {p}"));
        assert_eq!(st, StatusCode::OK, "protocolo {p}");
        assert!(!d["text"].as_str().unwrap().is_empty());
    }
    // Protocolo com tipo errado é corpo inválido.
    let corpo = json!({"finding_ids": ids, "protocol": 123});
    let (st, e) = send(&a.app, dispute(&id, &corpo.to_string())).await;
    assert_eq!(st, StatusCode::BAD_REQUEST);
    assert_eq!(e["error"]["code"], "invalid_body");
}

#[tokio::test]
async fn protocolo_invalido_e_validado_antes_de_procurar_a_analise() {
    let a = amb();
    let inexistente = Uuid::new_v4().to_string();
    let (st, e) = send(
        &a.app,
        dispute(
            &inexistente,
            r#"{"finding_ids":["x"],"protocol":"<script>"}"#,
        ),
    )
    .await;
    assert_eq!(st, StatusCode::BAD_REQUEST);
    assert_eq!(e["error"]["code"], "invalid_protocol");
    // Com protocolo bom, a análise inexistente aí sim é 404.
    let (st, e) = send(&a.app, dispute(&inexistente, r#"{"finding_ids":["x"]}"#)).await;
    assert_eq!(st, StatusCode::NOT_FOUND);
    assert_eq!(e["error"]["code"], "not_found");
}

#[tokio::test]
async fn contestacao_com_corpo_malformado_e_400_invalid_body() {
    let a = amb();
    let id = id_de(&demo(&a, "telecom-2").await);
    let corpos = [
        "",
        "null",
        "[]",
        "{",
        r#"{"finding_ids":null}"#,
        r#"{"finding_ids":"TEL-03:i2"}"#,
        r#"{"finding_ids":[1,2]}"#,
        r#"{"finding_ids":[null]}"#,
        r#"{"protocol":"1"}"#,
        r#"{"finding_ids":[],"extra":true}"#,
    ];
    for c in corpos {
        let (st, e) = send(&a.app, dispute(&id, c)).await;
        assert_eq!(st, StatusCode::BAD_REQUEST, "corpo: {c}");
        assert_eq!(e["error"]["code"], "invalid_body", "corpo: {c}");
    }
}

#[tokio::test]
async fn ids_de_achado_quase_certos_ou_vazios_sao_unknown_finding() {
    let a = amb();
    let id = id_de(&demo(&a, "telecom-2").await);
    let ids = [
        "",
        "tel-03:i2",
        "TEL-03:i2 ",
        " TEL-03:i2",
        "TEL-03:í2",
        "TEL-03:i2\n",
        "%00",
    ];
    for f in ids {
        let corpo = json!({"finding_ids": [f]});
        let (st, e) = send(&a.app, dispute(&id, &corpo.to_string())).await;
        assert_eq!(st, StatusCode::BAD_REQUEST, "{f:?}");
        assert_eq!(e["error"]["code"], "unknown_finding", "{f:?}");
    }
}

#[tokio::test]
async fn um_id_inexistente_no_meio_derruba_a_contestacao_inteira() {
    let a = amb();
    let v = demo(&a, "telecom-2").await;
    let id = id_de(&v);
    let bom = ids_indevidos(&v).remove(0);
    let corpo = json!({"finding_ids": [bom, "XYZ:i9"]});
    let (st, e) = send(&a.app, dispute(&id, &corpo.to_string())).await;
    assert_eq!(st, StatusCode::BAD_REQUEST);
    assert_eq!(e["error"]["code"], "unknown_finding");
}

#[tokio::test]
async fn ids_de_achado_repetidos_sao_contados_uma_vez_so() {
    let a = amb();
    let v = demo(&a, "telecom-2").await;
    let id = id_de(&v);
    let ids = ids_indevidos(&v);
    let (_, base) = send(
        &a.app,
        dispute(&id, &json!({"finding_ids": ids}).to_string()),
    )
    .await;
    let mut repetidos: Vec<String> = Vec::new();
    for _ in 0..5000 {
        repetidos.extend(ids.iter().cloned());
    }
    let (st, d) = send(
        &a.app,
        dispute(&id, &json!({"finding_ids": repetidos}).to_string()),
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(d["total_cents"], base["total_cents"]);
    assert_eq!(d["text"], base["text"]);
}

#[tokio::test]
async fn achado_ok_nao_pode_ser_contestado_nem_misturado_com_indevido() {
    let a = amb();
    let v = demo(&a, "telecom-1").await;
    let id = id_de(&v);
    // Responder "contratei" transforma o TEL-01:i2 em achado "ok"; o i3 segue em aberto.
    let (_, v) = send(
        &a.app,
        patch(
            &id,
            r#"{"answers":{"TEL-01:i2":{"type":"yes"},"TEL-01:i3":{"type":"no"}}}"#,
        ),
    )
    .await;
    let ok = achados(&v)
        .iter()
        .find(|f| f["severity"] == "ok")
        .expect("esperava um achado ok")["id"]
        .as_str()
        .unwrap()
        .to_string();
    let bom = ids_indevidos(&v).remove(0);
    for lista in [vec![ok.clone()], vec![bom, ok]] {
        let (st, e) = send(
            &a.app,
            dispute(&id, &json!({"finding_ids": lista}).to_string()),
        )
        .await;
        assert_eq!(st, StatusCode::BAD_REQUEST);
        assert_eq!(e["error"]["code"], "not_disputable");
    }
}

#[tokio::test]
async fn contestacao_nao_altera_a_analise_guardada() {
    let a = amb();
    let v = demo(&a, "telecom-2").await;
    let id = id_de(&v);
    let ids = ids_indevidos(&v);
    let (st, _) = send(
        &a.app,
        dispute(&id, &json!({"finding_ids": ids}).to_string()),
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(a.repo.updates(), 0);
    let (_, v2) = send(&a.app, get(&format!("/analyses/{id}"))).await;
    assert_eq!(v2, v);
}

#[tokio::test]
async fn contestacao_pedindo_dobro_so_quando_a_fatura_foi_paga() {
    let a = amb();
    let v = demo(&a, "telecom-2").await;
    let id = id_de(&v);
    let corpo = json!({"finding_ids": ids_indevidos(&v)}).to_string();
    let (_, sem) = send(&a.app, dispute(&id, &corpo)).await;
    send(&a.app, patch(&id, r#"{"paid":true}"#)).await;
    let (_, pago) = send(&a.app, dispute(&id, &corpo)).await;
    assert!(
        !sem["text"]
            .as_str()
            .unwrap()
            .contains("devolução em dobro do valor cobrado")
    );
    assert!(
        pago["text"]
            .as_str()
            .unwrap()
            .contains("devolução em dobro do valor cobrado")
    );
    // O total contestado é o mesmo; só o texto do pedido muda.
    assert_eq!(sem["total_cents"], pago["total_cents"]);
}

#[tokio::test]
async fn contestacao_de_energia_funciona_e_cita_a_norma_da_aneel() {
    let a = amb();
    let mut achou = false;
    for s in ["energia-1", "energia-2", "energia-3"] {
        let v = demo(&a, s).await;
        let ids = ids_indevidos(&v);
        if ids.is_empty() {
            continue;
        }
        achou = true;
        let id = id_de(&v);
        send(&a.app, patch(&id, r#"{"paid":true}"#)).await;
        let (st, d) = send(
            &a.app,
            dispute(&id, &json!({"finding_ids": ids}).to_string()),
        )
        .await;
        assert_eq!(st, StatusCode::OK, "{s}");
        assert!(d["text"].as_str().unwrap().contains("REN ANEEL"), "{s}");
        assert!(!d["steps"].as_array().unwrap().is_empty(), "{s}");
    }
    assert!(achou, "nenhuma demo de energia tem achado indevido");
}

// ───────────────────────────── rate limit ─────────────────────────────

fn com_ip(mut r: Request<Body>, ip: &str) -> Request<Body> {
    r.headers_mut()
        .insert("x-forwarded-for", ip.parse().unwrap());
    r
}

fn limites(upload_burst: u32, write_burst: u32, refill: StdDuration) -> Limits {
    Limits {
        upload_burst,
        upload_refill: refill,
        write_burst,
        write_refill: refill,
    }
}

#[tokio::test]
async fn sem_ip_identificavel_o_limitador_responde_400_em_json() {
    // Sem X-Forwarded-For e sem ConnectInfo (como no oneshot) não há chave para o limite.
    let a = amb_com_limites(limites(5, 5, StdDuration::from_secs(60)));
    let (st, v) = send(&a.app, post_vazio("/analyses/demo/telecom-1")).await;
    assert_eq!(st, StatusCode::BAD_REQUEST);
    assert_eq!(v["error"]["code"], "bad_request");
    assert_eq!(a.repo.inserts(), 0);
}

#[tokio::test]
async fn x_forwarded_for_invalido_nao_vira_chave_e_da_400() {
    let a = amb_com_limites(limites(5, 5, StdDuration::from_secs(60)));
    for ip in ["lixo", "999.999.999.999", "", "1.2.3"] {
        let (st, v) = send(&a.app, com_ip(post_vazio("/analyses/demo/telecom-1"), ip)).await;
        assert_eq!(st, StatusCode::BAD_REQUEST, "xff {ip:?}");
        assert_eq!(v["error"]["code"], "bad_request", "xff {ip:?}");
    }
}

#[tokio::test]
async fn limite_de_escrita_conta_so_o_primeiro_ip_da_lista_e_aceita_ipv6() {
    let a = amb_com_limites(limites(5, 2, StdDuration::from_secs(3600)));
    let req = |ip: &str| com_ip(post_vazio("/analyses/demo/telecom-1"), ip);
    // Mesmo 1º IP, proxies diferentes depois: mesmo balde.
    assert_eq!(
        send(&a.app, req("203.0.113.7, 10.0.0.1")).await.0,
        StatusCode::CREATED
    );
    assert_eq!(
        send(&a.app, req("203.0.113.7, 10.0.0.2")).await.0,
        StatusCode::CREATED
    );
    let (st, v) = send(&a.app, req("203.0.113.7, 10.0.0.3")).await;
    assert_eq!(st, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(v["error"]["code"], "rate_limited");
    // IPv6 e outros IPv4 têm baldes próprios.
    assert_eq!(
        send(&a.app, req("2001:db8::1")).await.0,
        StatusCode::CREATED
    );
    assert_eq!(
        send(&a.app, req("203.0.113.8")).await.0,
        StatusCode::CREATED
    );
}

#[tokio::test]
async fn rate_limit_de_escrita_vale_para_get_patch_e_contestacao_mas_nao_para_health() {
    let a = amb_com_limites(limites(5, 3, StdDuration::from_secs(3600)));
    let ip = "198.51.100.9";
    let (st, v) = send(&a.app, com_ip(post_vazio("/analyses/demo/telecom-2"), ip)).await; // 1
    assert_eq!(st, StatusCode::CREATED);
    let id = id_de(&v);
    let (st, _) = send(&a.app, com_ip(get(&format!("/analyses/{id}")), ip)).await; // 2
    assert_eq!(st, StatusCode::OK);
    let (st, _) = send(&a.app, com_ip(patch(&id, "{}"), ip)).await; // 3
    assert_eq!(st, StatusCode::OK);
    for req in [
        get(&format!("/analyses/{id}")),
        patch(&id, "{}"),
        dispute(&id, r#"{"finding_ids":["x"]}"#),
        post_vazio("/analyses/demo/telecom-1"),
    ] {
        let (st, v) = send(&a.app, com_ip(req, ip)).await;
        assert_eq!(st, StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(v["error"]["code"], "rate_limited");
    }
    // /health é o keep-alive do Render: nunca pode ser limitado nem exigir IP.
    for _ in 0..50 {
        let (st, _) = send(&a.app, get("/health")).await;
        assert_eq!(st, StatusCode::OK);
    }
}

#[tokio::test]
async fn baldes_de_upload_e_de_escrita_sao_independentes() {
    let a = amb_com_limites(limites(1, 5, StdDuration::from_secs(3600)));
    let ip = "192.0.2.50";
    let up = || com_ip(multipart(vec![consent("true"), arquivo(PDF)]), ip);
    assert_eq!(send(&a.app, up()).await.0, StatusCode::CREATED);
    assert_eq!(send(&a.app, up()).await.0, StatusCode::TOO_MANY_REQUESTS);
    // Esgotar o upload não bloqueia a demo nem o resto.
    assert_eq!(
        send(&a.app, com_ip(post_vazio("/analyses/demo/telecom-1"), ip))
            .await
            .0,
        StatusCode::CREATED
    );
    // E o upload barrado pelo limite não chegou a chamar a IA.
    assert_eq!(a.ext.as_ref().unwrap().chamadas(), 1);
}

#[tokio::test]
async fn limite_se_recupera_depois_do_tempo_de_reposicao() {
    let a = amb_com_limites(limites(5, 1, StdDuration::from_millis(100)));
    let ip = "192.0.2.77";
    let req = || com_ip(post_vazio("/analyses/demo/telecom-3"), ip);
    assert_eq!(send(&a.app, req()).await.0, StatusCode::CREATED);
    assert_eq!(send(&a.app, req()).await.0, StatusCode::TOO_MANY_REQUESTS);
    tokio::time::sleep(StdDuration::from_millis(500)).await;
    assert_eq!(send(&a.app, req()).await.0, StatusCode::CREATED);
}

// Corpo `[]`: o serde aceita sequência posicional numa struct.
// Não derruba nada, mas não deveria virar um PATCH válido; aqui só garantimos que não dá 5xx.
#[tokio::test]
async fn patch_com_array_no_lugar_do_objeto_nao_derruba_o_handler() {
    let a = amb();
    let id = id_de(&demo(&a, "telecom-1").await);
    for corpo in ["[]", "[null]", "[null,null,null]", "[{}]"] {
        let r = send_sem_panico(&a.app, patch(&id, corpo)).await;
        let (st, _) = r.unwrap_or_else(|| panic!("panic com corpo {corpo}"));
        assert!(
            st == StatusCode::OK || st == StatusCode::BAD_REQUEST,
            "{corpo} -> {st}"
        );
    }
}
