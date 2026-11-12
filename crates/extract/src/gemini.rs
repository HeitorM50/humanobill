//! Cliente REST do Gemini (`generateContent`) com saída forçada por JSON Schema.

use std::time::Instant;

use async_trait::async_trait;
use base64::{Engine, engine::general_purpose::STANDARD};
use humanobill_core::ExtractedBill;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::sync::atomic::{AtomicU32, Ordering};

use crate::{BillExtractor, ExtractError, InputMime, parse_and_validate, prompt, schema};

#[derive(Debug, Clone)]
pub struct GeminiConfig {
    pub api_key: String,
    /// Ex.: "gemini-3.6-flash" (env `GEMINI_MODEL`).
    pub model: String,
    /// Usado quando o modelo principal está indisponível (env `GEMINI_FALLBACK_MODEL`).
    pub fallback_model: Option<String>,
    pub base_url: String,
}

impl GeminiConfig {
    pub const DEFAULT_MODEL: &'static str = "gemini-3.6-flash";
    pub const DEFAULT_FALLBACK_MODEL: &'static str = "gemini-3.5-flash-lite";
    /// Tentativas por modelo em erro transitório (5xx / rede).
    pub const ATTEMPTS_PER_MODEL: u32 = 3;
    /// Orçamento total de uma extração (todas as tentativas e o retry de validação).
    /// Acima disso o usuário recebe "tente de novo" em vez de esperar (RNF01).
    pub const TOTAL_BUDGET: std::time::Duration = std::time::Duration::from_secs(40);
    /// Tempo máximo de uma única chamada HTTP; modelo lento passa para o fallback.
    pub const CALL_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(25);
    pub const DEFAULT_BASE_URL: &'static str = "https://generativelanguage.googleapis.com/v1beta";
}

pub struct GeminiExtractor {
    cfg: GeminiConfig,
    http: reqwest::Client,
    /// Chamadas HTTP realmente feitas ao provedor (medição de quota).
    http_calls: AtomicU32,
}

impl GeminiExtractor {
    pub fn new(cfg: GeminiConfig) -> Self {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(60)) // teto; cada chamada usa CALL_TIMEOUT
            .build()
            .expect("cliente HTTP");
        Self {
            cfg,
            http,
            http_calls: AtomicU32::new(0),
        }
    }

    /// Quantas chamadas HTTP ao Gemini este extrator já fez (cache hit não conta).
    pub fn http_calls(&self) -> u32 {
        self.http_calls.load(Ordering::Relaxed)
    }

    /// Cache de respostas brutas, só ativo com `EXTRACT_CACHE_DIR` (testes/medição).
    /// Chave = sha256(modelos + corpo da requisição, que inclui prompt, schema e arquivo).
    fn cache_path(&self, body: &Value) -> Option<std::path::PathBuf> {
        let dir = std::env::var_os("EXTRACT_CACHE_DIR")?;
        let mut h = Sha256::new();
        h.update(self.cfg.model.as_bytes());
        h.update(self.cfg.fallback_model.as_deref().unwrap_or("").as_bytes());
        h.update(body.to_string().as_bytes());
        let hex: String = h.finalize().iter().map(|b| format!("{b:02x}")).collect();
        Some(std::path::PathBuf::from(dir).join(format!("{hex}.json")))
    }

    fn request_body(&self, bytes: &[u8], mime: InputMime, retry_note: Option<&str>) -> Value {
        let mut parts = vec![
            json!({"inline_data": {"mime_type": mime.as_str(), "data": STANDARD.encode(bytes)}}),
            json!({"text": "Extraia os dados desta fatura no schema."}),
        ];
        if let Some(note) = retry_note {
            parts.push(
                json!({"text": format!("{}{note}. Corrija e responda de novo.", prompt::RETRY)}),
            );
        }
        json!({
            "system_instruction": {"parts": [{"text": prompt::SYSTEM}]},
            "contents": [{"role": "user", "parts": parts}],
            "generationConfig": {
                "temperature": 0,
                "responseMimeType": "application/json",
                "responseJsonSchema": schema::extracted_bill_schema()
            }
        })
    }

    /// Uma chamada a um modelo. `Err(Transient)` = vale tentar de novo.
    async fn call_once(
        &self,
        model: &str,
        body: &Value,
        timeout: std::time::Duration,
    ) -> Result<String, CallError> {
        let url = format!("{}/models/{}:generateContent", self.cfg.base_url, model);
        let started = std::time::Instant::now();
        self.http_calls.fetch_add(1, Ordering::Relaxed);
        let resp = self
            .http
            .post(url)
            .header("x-goog-api-key", &self.cfg.api_key)
            .json(body)
            .timeout(timeout)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    // Modelo lento: não insiste nele, vai para o fallback.
                    CallError::NextModel(format!("timeout ({model})"))
                } else {
                    CallError::Transient(e.without_url().to_string())
                }
            })?;
        let status = resp.status();
        // Só metadados no log (RNF13).
        tracing::info!(status = status.as_u16(), ms = started.elapsed().as_millis() as u64, %model, "gemini");
        if status.is_success() {
            let v: Value = resp
                .json()
                .await
                .map_err(|e| CallError::Fatal(ExtractError::Provider(e.to_string())))?;
            return response_text(&v).ok_or_else(|| {
                CallError::Fatal(ExtractError::Provider("resposta sem texto".into()))
            });
        }
        // A mensagem de erro do Google não contém o conteúdo enviado.
        let detail = resp
            .json::<Value>()
            .await
            .ok()
            .and_then(|v| v.pointer("/error/message")?.as_str().map(str::to_owned))
            .unwrap_or_default();
        let msg = format!("HTTP {status} ({model}): {detail}");
        tracing::warn!(status = status.as_u16(), %model, %detail, "gemini falhou");
        Err(match status.as_u16() {
            429 => CallError::Fatal(ExtractError::RateLimited),
            // 404 = modelo inexistente/descontinuado: pula direto para o fallback.
            404 => CallError::NextModel(msg),
            500 | 502 | 503 | 504 => CallError::Transient(msg),
            _ => CallError::Fatal(ExtractError::Provider(msg)),
        })
    }

    /// Retry com backoff em erro transitório e fallback de modelo.
    async fn call(&self, body: &Value, deadline: Instant) -> Result<String, ExtractError> {
        let cache = self.cache_path(body);
        if let Some(text) = cache.as_ref().and_then(|p| std::fs::read_to_string(p).ok()) {
            return Ok(text);
        }
        let text = self.call_uncached(body, deadline).await?;
        if let Some(p) = cache {
            if let Some(dir) = p.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            let _ = std::fs::write(p, &text);
        }
        Ok(text)
    }

    async fn call_uncached(&self, body: &Value, deadline: Instant) -> Result<String, ExtractError> {
        let models =
            std::iter::once(self.cfg.model.as_str()).chain(self.cfg.fallback_model.as_deref());
        let mut last = String::new();
        for model in models {
            for attempt in 0..GeminiConfig::ATTEMPTS_PER_MODEL {
                if attempt > 0 {
                    let wait = backoff(attempt);
                    if Instant::now() + wait >= deadline {
                        break;
                    }
                    tokio::time::sleep(wait).await;
                }
                let remaining = deadline.saturating_duration_since(Instant::now());
                if remaining < std::time::Duration::from_secs(2) {
                    tracing::warn!("orçamento de tempo da extração esgotado");
                    return Err(ExtractError::Provider(format!(
                        "orçamento esgotado; último erro: {last}"
                    )));
                }
                match self
                    .call_once(model, body, remaining.min(GeminiConfig::CALL_TIMEOUT))
                    .await
                {
                    Ok(text) => return Ok(text),
                    Err(CallError::Fatal(e)) => return Err(e),
                    Err(CallError::NextModel(m)) => {
                        last = m;
                        break;
                    }
                    Err(CallError::Transient(m)) => last = m,
                }
            }
        }
        Err(ExtractError::Provider(last))
    }
}

enum CallError {
    Transient(String),
    NextModel(String),
    Fatal(ExtractError),
}

/// 1s, 3s, 9s...
fn backoff(attempt: u32) -> std::time::Duration {
    std::time::Duration::from_secs(3u64.pow(attempt - 1))
}

/// Junta os `text` de `candidates[0].content.parts`.
fn response_text(v: &Value) -> Option<String> {
    let parts = v.pointer("/candidates/0/content/parts")?.as_array()?;
    let text: String = parts
        .iter()
        .filter_map(|p| p.get("text")?.as_str())
        .collect();
    (!text.is_empty()).then_some(text)
}

#[async_trait]
impl BillExtractor for GeminiExtractor {
    async fn extract(&self, bytes: &[u8], mime: InputMime) -> Result<ExtractedBill, ExtractError> {
        let deadline = Instant::now() + GeminiConfig::TOTAL_BUDGET;
        let first = self
            .call(&self.request_body(bytes, mime, None), deadline)
            .await?;
        let issues = match parse_and_validate(&first) {
            Ok(bill) => return Ok(bill),
            Err(issues) => issues,
        };
        // Uma única nova tentativa, explicando o que falhou (sem conteúdo da fatura).
        let note = if issues.is_empty() {
            "o JSON não segue o schema".to_string()
        } else {
            format!("{issues:?}")
        };
        tracing::warn!(issues = issues.len(), "extração inválida, tentando de novo");
        let second = self
            .call(&self.request_body(bytes, mime, Some(&note)), deadline)
            .await?;
        parse_and_validate(&second).map_err(|issues| ExtractError::Invalid { issues })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_cresce() {
        assert_eq!(backoff(1).as_secs(), 1);
        assert_eq!(backoff(2).as_secs(), 3);
    }

    #[test]
    fn extrai_texto_da_resposta() {
        let v =
            json!({"candidates": [{"content": {"parts": [{"text": "{\"a\":"}, {"text": "1}"}]}}]});
        assert_eq!(response_text(&v).as_deref(), Some("{\"a\":1}"));
        assert_eq!(response_text(&json!({})), None);
    }

    #[test]
    fn corpo_da_requisicao_tem_schema_e_pdf_inline() {
        let ex = GeminiExtractor::new(GeminiConfig {
            api_key: "x".into(),
            model: GeminiConfig::DEFAULT_MODEL.into(),
            fallback_model: None,
            base_url: GeminiConfig::DEFAULT_BASE_URL.into(),
        });
        let body = ex.request_body(b"%PDF-1.7", InputMime::Pdf, None);
        assert_eq!(
            body.pointer("/contents/0/parts/0/inline_data/mime_type")
                .unwrap(),
            "application/pdf"
        );
        assert_eq!(body.pointer("/generationConfig/temperature").unwrap(), 0);
        assert!(
            body.pointer("/generationConfig/responseJsonSchema/properties/items")
                .is_some()
        );
    }

    /// Teste live contra o Gemini (gasta quota). Rode com:
    /// GEMINI_API_KEY=... cargo test -p humanobill-extract -- --ignored live
    #[tokio::test]
    #[ignore]
    async fn live_extrai_fixture_telecom_1() {
        let key = std::env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY");
        let model =
            std::env::var("GEMINI_MODEL").unwrap_or_else(|_| GeminiConfig::DEFAULT_MODEL.into());
        let ex = GeminiExtractor::new(GeminiConfig {
            api_key: key,
            model,
            fallback_model: Some(GeminiConfig::DEFAULT_FALLBACK_MODEL.into()),
            base_url: GeminiConfig::DEFAULT_BASE_URL.into(),
        });
        let pdf = std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../fixtures/bills/telecom-1.pdf"
        ))
        .unwrap();
        let bill = ex.extract(&pdf, InputMime::Pdf).await.expect("extração");
        assert_eq!(bill.provider_name, "Vivaz Telecom");
    }
}
