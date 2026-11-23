//! #58: bot no WhatsApp (Cloud API da Meta) e simulador de conversa.
//!
//! Duas partes com a mesma lógica de conversa:
//! - `GET/POST /whatsapp/webhook`: o bot de verdade. **Desligado por padrão**: sem as quatro
//!   variáveis `WHATSAPP_*` as duas rotas respondem 404. Valida a assinatura do corpo,
//!   baixa a mídia pela Graph API, usa o mesmo pipeline do upload e responde pela Graph API.
//! - `POST /whatsapp/simulador`: ligado em produção, para a demo. Só aceita texto ou o id de
//!   um exemplo, nunca arquivo (não gasta a quota da IA).
//!
//! Privacidade (RNF13): nunca vai para log o número de telefone, o texto da mensagem, o id da
//! mídia nem o conteúdo da fatura. Só ids de análise, latência e códigos de erro. Os tipos
//! que carregam esses dados têm `Debug` que esconde o conteúdo.

use std::{
    collections::{HashMap, HashSet, VecDeque},
    fmt,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use async_trait::async_trait;
use axum::{
    Json, Router,
    body::Bytes,
    extract::{DefaultBodyLimit, Query, State, rejection::JsonRejection},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use hmac::{Hmac, KeyInit, Mac};
use humanobill_core::{Analysis, Severity};
use humanobill_extract::BillExtractor;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

use super::analysis_from_sample;
use crate::{
    config::WhatsappConfig,
    error::ApiError,
    limits::Limits,
    repo::Repo,
    upload::{self, MAX_BYTES},
};

/// Quantos achados aparecem na resposta. O resto fica no link.
const MAX_FINDINGS_IN_REPLY: usize = 3;
/// Tamanho máximo do texto aceito no simulador.
const MAX_SIM_TEXT_CHARS: usize = 300;
/// Corpo máximo do webhook (os payloads da Meta têm poucos KB).
const MAX_WEBHOOK_BYTES: usize = 256 * 1024;
/// Por remetente: faturas lidas pela IA por janela (protege a quota do Gemini).
const UPLOADS_PER_SENDER: usize = 5;
/// Para todos juntos: faturas lidas pela IA por janela.
const UPLOADS_GLOBAL: usize = 40;
/// Por remetente: exemplos pedidos por janela.
const SAMPLES_PER_SENDER: usize = 20;
const THROTTLE_WINDOW: Duration = Duration::from_secs(3600);
/// Mensagens já vistas (a Meta reenvia o webhook quando demora a responder).
const DEDUP_CAPACITY: usize = 2048;

/// Exemplos que o bot conhece. O simulador aceita só estes ids na lista de atalhos.
const SAMPLES: [&str; 6] = [
    "telecom-1",
    "telecom-2",
    "telecom-3",
    "energia-1",
    "energia-2",
    "energia-3",
];

// ---------------------------------------------------------------------------
// Textos do bot (funções puras)
// ---------------------------------------------------------------------------

/// Uma mensagem do bot. `resultado_id` é preenchido quando ela traz o link de um resultado,
/// para o simulador abrir `/r/<id>` dentro do app.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BotMessage {
    pub texto: String,
    pub resultado_id: Option<String>,
}

impl BotMessage {
    fn plain(texto: impl Into<String>) -> Self {
        Self {
            texto: texto.into(),
            resultado_id: None,
        }
    }
}

/// Motivos de falha que o usuário vê. Nenhum guarda detalhe da causa.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BotError {
    /// Arquivo que não é foto nem PDF, ou PDF com páginas demais.
    Unreadable,
    TooLarge,
    /// A IA não conseguiu ler a fatura.
    ReadFailed,
    RateLimited,
    /// Sem chave da IA: só os exemplos funcionam.
    Unavailable,
    /// Não deu para baixar a mídia na Graph API.
    DownloadFailed,
    /// Áudio, figurinha, localização etc.
    Unsupported,
    Internal,
}

impl From<&ApiError> for BotError {
    fn from(e: &ApiError) -> Self {
        match e.code {
            "too_large" => BotError::TooLarge,
            "invalid_file" => BotError::Unreadable,
            "extraction_failed" => BotError::ReadFailed,
            "rate_limited" => BotError::RateLimited,
            "extraction_unavailable" => BotError::Unavailable,
            _ => BotError::Internal,
        }
    }
}

/// Mensagem amigável para cada erro. Diz o que fazer, sem termo técnico.
pub fn error_text(e: BotError) -> &'static str {
    match e {
        BotError::Unreadable => {
            "Não consegui abrir esse arquivo. Envie uma foto da fatura (JPG ou PNG) ou o PDF dela, com até 5 páginas."
        }
        BotError::TooLarge => {
            "Esse arquivo é grande demais (o limite é 10 MB). Tire uma foto nova ou envie um PDF mais leve."
        }
        BotError::ReadFailed => {
            "Não consegui ler bem essa fatura. Tente de novo com uma foto mais nítida, com a fatura inteira na imagem e sem reflexo."
        }
        BotError::RateLimited => {
            "Você pediu muitas análises em pouco tempo. Espere alguns minutos e tente de novo. Enquanto isso, digite EXEMPLO para ver como funciona."
        }
        BotError::Unavailable => {
            "A leitura de faturas está fora do ar agora. Digite EXEMPLO para ver como o HumanoBill funciona e tente a sua fatura mais tarde."
        }
        BotError::DownloadFailed => {
            "Não consegui baixar o arquivo que você enviou. Envie de novo, por favor."
        }
        BotError::Unsupported => {
            "Só entendo texto, foto ou PDF. Envie a foto ou o PDF da fatura, ou digite MENU."
        }
        BotError::Internal => "Tive um problema do nosso lado. Tente de novo em instantes.",
    }
}

const ACK_TEXT: &str = "Recebi! Estou lendo a sua fatura. Pode levar até 1 minuto.";

fn link(web_url: &str, path: &str) -> String {
    format!("{web_url}{path}")
}

/// Menu inicial. Também diz, antes do envio, que a fatura é lida por IA (consentimento).
pub fn menu_text(web_url: &str) -> String {
    format!(
        "Oi! Eu sou o HumanoBill. Explico a sua fatura em português claro e aviso se tem cobrança indevida.\n\n\
         Como usar:\n\
         1. Envie a foto ou o PDF da fatura de celular, internet, TV ou luz.\n\
         2. Ou digite EXEMPLO para testar com uma fatura de mentirinha (EXEMPLO LUZ para conta de luz).\n\n\
         Ao enviar a sua fatura, você concorda que uma IA do Google faça a leitura. Eu não guardo o arquivo e não leio nome, CPF nem endereço. Detalhes: {}",
        link(web_url, "/privacidade")
    )
}

fn unknown_text() -> &'static str {
    "Não entendi. Envie a foto ou o PDF da fatura, ou digite EXEMPLO para testar. Digite MENU para ver as opções."
}

/// Centavos em reais: 123456 -> "R$ 1.234,56".
fn brl(cents: i64) -> String {
    let sign = if cents < 0 { "-" } else { "" };
    let abs = cents.unsigned_abs();
    let reais = (abs / 100).to_string();
    let mut grouped = String::new();
    for (i, ch) in reais.chars().enumerate() {
        if i > 0 && (reais.len() - i).is_multiple_of(3) {
            grouped.push('.');
        }
        grouped.push(ch);
    }
    format!("{sign}R$ {grouped},{:02}", abs % 100)
}

/// "2026-10" -> "outubro de 2026". Formato inesperado volta como veio.
fn month_label(reference_month: &str) -> String {
    const MESES: [&str; 12] = [
        "janeiro",
        "fevereiro",
        "março",
        "abril",
        "maio",
        "junho",
        "julho",
        "agosto",
        "setembro",
        "outubro",
        "novembro",
        "dezembro",
    ];
    reference_month
        .split_once('-')
        .and_then(|(y, m)| {
            let m: usize = m.parse().ok()?;
            (y.len() == 4 && (1..=12).contains(&m)).then(|| format!("{} de {y}", MESES[m - 1]))
        })
        .unwrap_or_else(|| reference_month.to_string())
}

/// Resposta com o resultado de uma análise: empresa e mês, total, quantos problemas, quanto
/// parece indevido, até 3 achados em uma linha cada e o link. Só lê o que o motor já decidiu.
pub fn analysis_reply(a: &Analysis, web_url: &str) -> String {
    let problems: Vec<_> = a
        .findings
        .iter()
        .filter(|f| f.severity != Severity::Ok)
        .collect();
    let mut out = format!(
        "{} · {}\nTotal da fatura: {}\n\n",
        a.bill.provider_name,
        month_label(&a.bill.reference_month),
        brl(a.bill.total_cents)
    );
    if problems.is_empty() {
        out.push_str("✅ Não encontrei nada de errado nessa fatura.\n");
    } else {
        let n = problems.len();
        out.push_str(&format!(
            "⚠️ Achei {n} {} para conferir.\n",
            if n == 1 { "ponto" } else { "pontos" }
        ));
        if a.recoverable_cents > 0 {
            out.push_str(&format!(
                "Desse total, {} parece indevido.\n",
                brl(a.recoverable_cents)
            ));
        } else {
            out.push_str("Nada está confirmado como indevido, mas vale conferir.\n");
        }
        out.push('\n');
        for f in problems.iter().take(MAX_FINDINGS_IN_REPLY) {
            let rotulo = if f.severity == Severity::Unlawful {
                "Indevido"
            } else {
                "Suspeito"
            };
            if f.amount_cents > 0 {
                out.push_str(&format!(
                    "• {rotulo}: {} ({})\n",
                    f.title,
                    brl(f.amount_cents)
                ));
            } else {
                out.push_str(&format!("• {rotulo}: {}\n", f.title));
            }
        }
        if n > MAX_FINDINGS_IN_REPLY {
            let resto = n - MAX_FINDINGS_IN_REPLY;
            out.push_str(&format!(
                "+ {resto} {} no link.\n",
                if resto == 1 {
                    "outro ponto"
                } else {
                    "outros pontos"
                }
            ));
        }
    }
    out.push_str(&format!(
        "\nVeja tudo, com a lei e a carta pronta (o link vale por 30 dias):\n{}\n\nConteúdo informativo, não é orientação jurídica.",
        link(web_url, &format!("/r/{}", a.id))
    ));
    out
}

// ---------------------------------------------------------------------------
// Entendimento do texto (função pura)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Intent {
    Menu,
    /// Id de um exemplo de `SAMPLES`.
    Sample(&'static str),
    Unknown,
}

/// Minúsculas e sem acento, para "EXEMPLO", "Olá" e "exemplo luz" valerem igual.
fn fold(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| match c {
            'á' | 'à' | 'â' | 'ã' | 'ä' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' => 'i',
            'ó' | 'ò' | 'ô' | 'õ' | 'ö' => 'o',
            'ú' | 'ù' | 'û' | 'ü' => 'u',
            'ç' => 'c',
            c => c,
        })
        .collect()
}

pub fn parse_intent(text: &str) -> Intent {
    let folded = fold(text);
    let tokens: Vec<&str> = folded
        .split(|c: char| !(c.is_ascii_alphanumeric() || c == '-'))
        .filter(|t| !t.is_empty())
        .collect();
    let Some(&first) = tokens.first() else {
        return Intent::Unknown;
    };
    if let Some(id) = SAMPLES.iter().find(|s| **s == first) {
        return Intent::Sample(id);
    }
    match first {
        "exemplo" | "exemplos" | "teste" | "demo" | "demonstracao" => {
            let energia = tokens[1..]
                .iter()
                .any(|t| matches!(*t, "luz" | "energia" | "eletrica"));
            let numero = tokens[1..]
                .iter()
                .find_map(|t| t.parse::<usize>().ok().filter(|n| (1..=3).contains(n)))
                .unwrap_or(1);
            let prefixo = if energia { "energia" } else { "telecom" };
            let id = SAMPLES
                .iter()
                .find(|s| **s == format!("{prefixo}-{numero}"))
                .expect("combinações de exemplo existem em SAMPLES");
            Intent::Sample(id)
        }
        "menu" | "ajuda" | "oi" | "ola" | "bom" | "boa" | "inicio" | "comecar" | "start"
        | "help" | "hi" | "hello" => Intent::Menu,
        _ => Intent::Unknown,
    }
}

// ---------------------------------------------------------------------------
// Graph API (atrás de um trait, para mock nos testes)
// ---------------------------------------------------------------------------

/// Falhas da Graph API. Sem detalhe de propósito: a causa pode conter número ou URL de mídia.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphError {
    TooLarge,
    Failed,
}

#[async_trait]
pub trait GraphClient: Send + Sync {
    /// Baixa a mídia de uma mensagem (2 chamadas: metadados e arquivo). Respeita `max_bytes`.
    async fn download_media(&self, media_id: &str, max_bytes: usize)
    -> Result<Vec<u8>, GraphError>;
    /// Envia uma mensagem de texto ao número `to`.
    async fn send_text(&self, to: &str, body: &str) -> Result<(), GraphError>;
}

pub struct HttpGraphClient {
    http: reqwest::Client,
    cfg: Arc<WhatsappConfig>,
}

impl HttpGraphClient {
    pub fn new(cfg: Arc<WhatsappConfig>) -> Self {
        let http = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(30))
            .build()
            .expect("cliente HTTP válido");
        Self { http, cfg }
    }
}

/// A mídia só vem por https. O http fica para a Graph de teste, quando a própria `graph_url`
/// é http (localhost).
fn media_url_allowed(url: &str, graph_url: &str) -> bool {
    url.starts_with("https://") || (url.starts_with("http://") && graph_url.starts_with("http://"))
}

#[derive(Deserialize)]
struct MediaMeta {
    url: Option<String>,
    file_size: Option<u64>,
}

#[async_trait]
impl GraphClient for HttpGraphClient {
    async fn download_media(
        &self,
        media_id: &str,
        max_bytes: usize,
    ) -> Result<Vec<u8>, GraphError> {
        // O id vem do payload: só caracteres de id, para nunca mexer no caminho da URL.
        if media_id.is_empty()
            || !media_id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        {
            return Err(GraphError::Failed);
        }
        let token = &self.cfg.token.0;
        let meta: MediaMeta = self
            .http
            .get(format!("{}/{media_id}", self.cfg.graph_url))
            .bearer_auth(token)
            .send()
            .await
            .and_then(|r| r.error_for_status())
            .map_err(|_| GraphError::Failed)?
            .json()
            .await
            .map_err(|_| GraphError::Failed)?;
        if meta.file_size.is_some_and(|s| s > max_bytes as u64) {
            return Err(GraphError::TooLarge);
        }
        let url = meta.url.ok_or(GraphError::Failed)?;
        if !media_url_allowed(&url, &self.cfg.graph_url) {
            return Err(GraphError::Failed);
        }
        let mut resp = self
            .http
            .get(url)
            .bearer_auth(token)
            .send()
            .await
            .and_then(|r| r.error_for_status())
            .map_err(|_| GraphError::Failed)?;
        if resp
            .content_length()
            .is_some_and(|len| len > max_bytes as u64)
        {
            return Err(GraphError::TooLarge);
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = resp.chunk().await.map_err(|_| GraphError::Failed)? {
            if bytes.len() + chunk.len() > max_bytes {
                return Err(GraphError::TooLarge);
            }
            bytes.extend_from_slice(&chunk);
        }
        Ok(bytes)
    }

    async fn send_text(&self, to: &str, body: &str) -> Result<(), GraphError> {
        self.http
            .post(format!(
                "{}/{}/messages",
                self.cfg.graph_url, self.cfg.phone_id
            ))
            .bearer_auth(&self.cfg.token.0)
            .json(&serde_json::json!({
                "messaging_product": "whatsapp",
                "to": to,
                "type": "text",
                "text": { "preview_url": true, "body": body },
            }))
            .send()
            .await
            .and_then(|r| r.error_for_status())
            .map(|_| ())
            .map_err(|_| GraphError::Failed)
    }
}

// ---------------------------------------------------------------------------
// Assinatura e payload do webhook
// ---------------------------------------------------------------------------

fn decode_hex(s: &str) -> Option<Vec<u8>> {
    if !s.len().is_multiple_of(2) || !s.is_ascii() {
        return None;
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).ok())
        .collect()
}

/// `X-Hub-Signature-256: sha256=<hex>`: HMAC-SHA256 do corpo cru com o segredo do app.
/// A comparação é em tempo constante (`subtle`).
pub fn verify_signature(app_secret: &str, header_value: Option<&str>, body: &[u8]) -> bool {
    let Some(sig) = header_value
        .and_then(|h| h.strip_prefix("sha256="))
        .and_then(decode_hex)
    else {
        return false;
    };
    let Ok(mut mac) = <Hmac<Sha256> as KeyInit>::new_from_slice(app_secret.as_bytes()) else {
        return false;
    };
    mac.update(body);
    let expected = mac.finalize().into_bytes();
    expected.as_slice().ct_eq(&sig).into()
}

/// Uma mensagem recebida. O `Debug` esconde tudo: número e texto nunca podem ir para log.
#[derive(Clone, PartialEq, Eq)]
pub struct Inbound {
    pub from: String,
    pub id: String,
    pub kind: InboundKind,
}

#[derive(Clone, PartialEq, Eq)]
pub enum InboundKind {
    Text(String),
    /// Foto ou documento: guarda só o id da mídia na Graph API.
    Media(String),
    Unsupported,
}

impl fmt::Debug for Inbound {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Inbound(<oculto>)")
    }
}
impl fmt::Debug for InboundKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("InboundKind(<oculto>)")
    }
}

// Formato da Cloud API: entry[].changes[].value.{metadata, messages[]}. Tudo opcional, para
// aceitar também os avisos de status (entregue, lido), que não trazem `messages`.
#[derive(Deserialize)]
struct Payload {
    entry: Option<Vec<Entry>>,
}
#[derive(Deserialize)]
struct Entry {
    changes: Option<Vec<Change>>,
}
#[derive(Deserialize)]
struct Change {
    value: Option<ChangeValue>,
}
#[derive(Deserialize)]
struct ChangeValue {
    metadata: Option<Metadata>,
    messages: Option<Vec<RawMessage>>,
}
#[derive(Deserialize)]
struct Metadata {
    phone_number_id: Option<String>,
}
#[derive(Deserialize)]
struct RawMessage {
    from: Option<String>,
    id: Option<String>,
    #[serde(rename = "type")]
    kind: Option<String>,
    text: Option<RawText>,
    image: Option<RawMedia>,
    document: Option<RawMedia>,
}
#[derive(Deserialize)]
struct RawText {
    body: Option<String>,
}
#[derive(Deserialize)]
struct RawMedia {
    id: Option<String>,
}

/// Lê as mensagens do payload. Avisos de status e mensagens de outro número são ignorados.
/// `Err` só para JSON inválido.
pub fn parse_inbound(body: &[u8], phone_id: &str) -> Result<Vec<Inbound>, ()> {
    let payload: Payload = serde_json::from_slice(body).map_err(|_| ())?;
    let mut out = Vec::new();
    for change in payload
        .entry
        .into_iter()
        .flatten()
        .flat_map(|e| e.changes.into_iter().flatten())
    {
        let Some(value) = change.value else { continue };
        let other_number = value
            .metadata
            .and_then(|m| m.phone_number_id)
            .is_some_and(|id| id != phone_id);
        if other_number {
            continue;
        }
        for m in value.messages.into_iter().flatten() {
            let (Some(from), Some(id)) = (m.from, m.id) else {
                continue;
            };
            let kind = match m.kind.as_deref() {
                Some("text") => m
                    .text
                    .and_then(|t| t.body)
                    .map_or(InboundKind::Unsupported, InboundKind::Text),
                Some("image") => m
                    .image
                    .and_then(|i| i.id)
                    .map_or(InboundKind::Unsupported, InboundKind::Media),
                Some("document") => m
                    .document
                    .and_then(|d| d.id)
                    .map_or(InboundKind::Unsupported, InboundKind::Media),
                _ => InboundKind::Unsupported,
            };
            out.push(Inbound { from, id, kind });
        }
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// Limites e deduplicação (em memória; o número só aparece como hash)
// ---------------------------------------------------------------------------

type Key = [u8; 32];

fn hash_key(s: &str) -> Key {
    Sha256::digest(s.as_bytes()).into()
}

/// Janela deslizante: no máximo `max` pedidos por chave dentro de `window`.
struct Throttle {
    max: usize,
    window: Duration,
    hits: Mutex<HashMap<Key, VecDeque<Instant>>>,
}

impl Throttle {
    fn new(max: usize, window: Duration) -> Self {
        Self {
            max,
            window,
            hits: Mutex::new(HashMap::new()),
        }
    }

    fn allow(&self, key: Key, now: Instant) -> bool {
        let mut hits = self.hits.lock().unwrap_or_else(|e| e.into_inner());
        // Mantém a memória limitada: descarta quem não pede há mais que a janela.
        if hits.len() > 10_000 {
            hits.retain(|_, q| {
                q.back()
                    .is_some_and(|t| now.duration_since(*t) < self.window)
            });
        }
        let q = hits.entry(key).or_default();
        while q
            .front()
            .is_some_and(|t| now.duration_since(*t) >= self.window)
        {
            q.pop_front();
        }
        if q.len() >= self.max {
            return false;
        }
        q.push_back(now);
        true
    }
}

/// Lembra os últimos ids de mensagem para não processar a mesma entrega duas vezes.
#[derive(Default)]
struct Dedup {
    seen: Mutex<(VecDeque<Key>, HashSet<Key>)>,
}

impl Dedup {
    fn first_time(&self, message_id: &str) -> bool {
        let key = hash_key(message_id);
        let mut g = self.seen.lock().unwrap_or_else(|e| e.into_inner());
        if g.1.contains(&key) {
            return false;
        }
        if g.0.len() >= DEDUP_CAPACITY
            && let Some(old) = g.0.pop_front()
        {
            g.1.remove(&old);
        }
        g.0.push_back(key);
        g.1.insert(key);
        true
    }
}

struct Guard {
    uploads: Throttle,
    uploads_global: Throttle,
    samples: Throttle,
    dedup: Dedup,
}

impl Guard {
    fn new() -> Self {
        Self {
            uploads: Throttle::new(UPLOADS_PER_SENDER, THROTTLE_WINDOW),
            uploads_global: Throttle::new(UPLOADS_GLOBAL, THROTTLE_WINDOW),
            samples: Throttle::new(SAMPLES_PER_SENDER, THROTTLE_WINDOW),
            dedup: Dedup::default(),
        }
    }
}

// ---------------------------------------------------------------------------
// Estado e rotas
// ---------------------------------------------------------------------------

/// Estado só do WhatsApp. Fica fora de `AppState` para a API subir (e as outras rotas
/// funcionarem) sem as variáveis `WHATSAPP_*`.
#[derive(Clone)]
pub struct WhatsappState {
    pub repo: Arc<dyn Repo>,
    pub extractor: Option<Arc<dyn BillExtractor>>,
    pub fixtures_dir: PathBuf,
    /// Endereço público do front, sem barra final: vira o prefixo dos links.
    pub web_url: String,
    /// `None` = webhook desligado (404). O simulador não depende disso.
    pub config: Option<Arc<WhatsappConfig>>,
    pub graph: Option<Arc<dyn GraphClient>>,
    /// `true` em produção: o webhook responde 200 logo e processa em segundo plano (a Meta
    /// reenvia se demora). Os testes usam `false` para conferir o resultado na própria chamada.
    pub background: bool,
    guard: Arc<Guard>,
}

impl WhatsappState {
    pub fn new(
        repo: Arc<dyn Repo>,
        extractor: Option<Arc<dyn BillExtractor>>,
        fixtures_dir: PathBuf,
        web_url: String,
        config: Option<WhatsappConfig>,
    ) -> Self {
        let config = config.map(Arc::new);
        let graph = config
            .clone()
            .map(|c| Arc::new(HttpGraphClient::new(c)) as Arc<dyn GraphClient>);
        Self {
            repo,
            extractor,
            fixtures_dir,
            web_url,
            config,
            graph,
            background: true,
            guard: Arc::new(Guard::new()),
        }
    }
}

pub fn router(state: WhatsappState, limits: Option<Limits>) -> Router {
    // O simulador usa o mesmo rate limit por IP das rotas que escrevem. O webhook não: quem
    // chama é a Meta (IPs compartilhados), e o que o protege é a assinatura do corpo.
    let simulator = Router::new()
        .route("/whatsapp/simulador", post(simulate))
        .layer(DefaultBodyLimit::max(8 * 1024));
    let simulator = match limits {
        Some(l) => {
            let (layer, cfg) = crate::limits::layer(l.write_burst, l.write_refill);
            if tokio::runtime::Handle::try_current().is_ok() {
                crate::limits::spawn_cleanup(vec![cfg]);
            }
            simulator.layer(layer)
        }
        None => simulator,
    };
    let webhook = Router::new().route(
        "/whatsapp/webhook",
        get(verify_webhook)
            .post(receive_webhook)
            .layer(DefaultBodyLimit::max(MAX_WEBHOOK_BYTES)),
    );
    webhook.merge(simulator).with_state(state)
}

fn disabled() -> ApiError {
    ApiError::new(StatusCode::NOT_FOUND, "not_found", "Rota não encontrada")
}

#[derive(Debug, Deserialize)]
struct VerifyQuery {
    #[serde(rename = "hub.mode")]
    mode: Option<String>,
    #[serde(rename = "hub.verify_token")]
    verify_token: Option<String>,
    #[serde(rename = "hub.challenge")]
    challenge: Option<String>,
}

/// Verificação do webhook: a Meta chama uma vez, ao cadastrar a URL.
async fn verify_webhook(
    State(s): State<WhatsappState>,
    Query(q): Query<VerifyQuery>,
) -> Result<Response, ApiError> {
    let cfg = s.config.as_ref().ok_or_else(disabled)?;
    let token_ok = q
        .verify_token
        .as_deref()
        .is_some_and(|t| t.as_bytes().ct_eq(cfg.verify_token.0.as_bytes()).into());
    let forbidden = || {
        ApiError::new(
            StatusCode::FORBIDDEN,
            "verification_failed",
            "Verificação recusada",
        )
    };
    if q.mode.as_deref() != Some("subscribe") || !token_ok {
        return Err(forbidden());
    }
    // O desafio é um número; devolver só caracteres seguros evita refletir conteúdo.
    let challenge = q
        .challenge
        .filter(|c| !c.is_empty() && c.len() <= 128 && c.chars().all(|c| c.is_ascii_alphanumeric()))
        .ok_or_else(|| {
            ApiError::new(
                StatusCode::BAD_REQUEST,
                "invalid_challenge",
                "Desafio inválido",
            )
        })?;
    Ok((
        [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
        challenge,
    )
        .into_response())
}

/// Recebe as mensagens. Confere a assinatura do corpo cru antes de ler qualquer coisa.
async fn receive_webhook(
    State(s): State<WhatsappState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<StatusCode, ApiError> {
    let cfg = s.config.clone().ok_or_else(disabled)?;
    let signature = headers
        .get("x-hub-signature-256")
        .and_then(|v| v.to_str().ok());
    if !verify_signature(&cfg.app_secret.0, signature, &body) {
        tracing::warn!("whatsapp: assinatura inválida");
        return Err(ApiError::new(
            StatusCode::UNAUTHORIZED,
            "invalid_signature",
            "Assinatura inválida",
        ));
    }
    let messages = parse_inbound(&body, &cfg.phone_id)
        .map_err(|_| ApiError::new(StatusCode::BAD_REQUEST, "invalid_body", "Payload inválido"))?;
    if messages.is_empty() {
        return Ok(StatusCode::OK);
    }
    if s.background {
        tokio::spawn(async move { s.process(messages).await });
    } else {
        s.process(messages).await;
    }
    Ok(StatusCode::OK)
}

impl WhatsappState {
    /// Resposta do bot a um texto (usada pelo webhook e pelo simulador).
    async fn reply_intent(&self, intent: Intent) -> Result<Vec<BotMessage>, ApiError> {
        match intent {
            Intent::Menu => Ok(vec![BotMessage::plain(menu_text(&self.web_url))]),
            Intent::Unknown => Ok(vec![BotMessage::plain(unknown_text())]),
            Intent::Sample(sample) => self.reply_sample(sample).await,
        }
    }

    /// Caminho da demo: análise a partir de um exemplo, sem IA.
    async fn reply_sample(&self, sample: &str) -> Result<Vec<BotMessage>, ApiError> {
        let analysis = analysis_from_sample(&self.fixtures_dir, sample).await?;
        self.repo.insert(&analysis).await?;
        tracing::info!(id = %analysis.id, %sample, "whatsapp: análise demo criada");
        Ok(vec![BotMessage {
            texto: analysis_reply(&analysis, &self.web_url),
            resultado_id: Some(analysis.id),
        }])
    }

    async fn process(&self, messages: Vec<Inbound>) {
        for m in messages {
            self.handle(m).await;
        }
    }

    async fn handle(&self, m: Inbound) {
        let Some(graph) = self.graph.clone() else {
            return;
        };
        if !self.guard.dedup.first_time(&m.id) {
            return;
        }
        let key = hash_key(&m.from);
        let now = Instant::now();
        let replies = match m.kind {
            InboundKind::Text(text) => match parse_intent(&text) {
                Intent::Sample(_) if !self.guard.samples.allow(key, now) => {
                    vec![BotMessage::plain(error_text(BotError::RateLimited))]
                }
                intent => self
                    .reply_intent(intent)
                    .await
                    .unwrap_or_else(|e| vec![BotMessage::plain(error_text(BotError::from(&e)))]),
            },
            InboundKind::Media(media_id) => {
                self.reply_media(graph.as_ref(), &m.from, key, &media_id, now)
                    .await
            }
            InboundKind::Unsupported => {
                vec![BotMessage::plain(error_text(BotError::Unsupported))]
            }
        };
        for r in replies {
            if graph.send_text(&m.from, &r.texto).await.is_err() {
                tracing::warn!("whatsapp: falha ao enviar a resposta");
            }
        }
    }

    /// Foto ou PDF: baixa pela Graph API e roda o mesmo pipeline do upload (com IA).
    async fn reply_media(
        &self,
        graph: &dyn GraphClient,
        to: &str,
        key: Key,
        media_id: &str,
        now: Instant,
    ) -> Vec<BotMessage> {
        let erro = |e| vec![BotMessage::plain(error_text(e))];
        // Cada leitura gasta a quota da IA: limite por remetente e para todos juntos.
        if !self.guard.uploads.allow(key, now)
            || !self.guard.uploads_global.allow(hash_key("global"), now)
        {
            return erro(BotError::RateLimited);
        }
        if graph.send_text(to, ACK_TEXT).await.is_err() {
            tracing::warn!("whatsapp: falha ao enviar o aviso de recebimento");
        }
        let started = Instant::now();
        let bytes = match graph.download_media(media_id, MAX_BYTES).await {
            Ok(b) => b,
            Err(GraphError::TooLarge) => return erro(BotError::TooLarge),
            Err(GraphError::Failed) => {
                tracing::warn!("whatsapp: falha ao baixar a mídia");
                return erro(BotError::DownloadFailed);
            }
        };
        match upload::analyze_upload(&self.repo, self.extractor.as_ref(), bytes).await {
            Ok(analysis) => {
                tracing::info!(
                    id = %analysis.id,
                    ms = started.elapsed().as_millis() as u64,
                    "whatsapp: análise criada"
                );
                vec![BotMessage {
                    texto: analysis_reply(&analysis, &self.web_url),
                    resultado_id: Some(analysis.id),
                }]
            }
            Err(e) => erro(BotError::from(&e)),
        }
    }
}

// ---------------------------------------------------------------------------
// Simulador
// ---------------------------------------------------------------------------

/// Corpo do `POST /whatsapp/simulador`: ou `texto`, ou `exemplo` (id de uma fatura de exemplo).
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SimBody {
    texto: Option<String>,
    exemplo: Option<String>,
}

#[derive(Debug, Serialize)]
struct SimReply {
    mensagens: Vec<BotMessage>,
}

/// Mesma conversa do bot, sem Meta e sem arquivo: nunca chama a IA.
async fn simulate(
    State(s): State<WhatsappState>,
    body: Result<Json<SimBody>, JsonRejection>,
) -> Result<Json<SimReply>, ApiError> {
    let invalid = |msg: &str| ApiError::new(StatusCode::BAD_REQUEST, "invalid_body", msg);
    let Json(body) = body.map_err(|_| invalid("Envie um texto ou o id de um exemplo"))?;
    let mensagens = match (body.texto, body.exemplo) {
        (Some(texto), None) => {
            let texto = texto.trim();
            if texto.is_empty() || texto.chars().count() > MAX_SIM_TEXT_CHARS {
                return Err(invalid("Escreva uma mensagem curta"));
            }
            s.reply_intent(parse_intent(texto)).await?
        }
        (None, Some(exemplo)) => s.reply_sample(&exemplo).await?,
        _ => return Err(invalid("Envie só um texto ou só o id de um exemplo")),
    };
    Ok(Json(SimReply { mensagens }))
}

#[cfg(test)]
mod tests;
