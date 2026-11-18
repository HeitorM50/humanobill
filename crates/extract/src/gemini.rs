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
    /// Lista separada por vírgula, tentada em ordem quando o anterior está indisponível.
    pub const DEFAULT_FALLBACK_MODEL: &'static str = "gemini-3.5-flash-lite,gemini-3.1-flash-lite";
    /// Tentativas por modelo em erro transitório (5xx / rede).
    /// Poucas tentativas por modelo: com o Google sobrecarregado (503), vale mais
    /// passar para o próximo modelo do que insistir no mesmo.
    pub const ATTEMPTS_PER_MODEL: u32 = 2;
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
            std::iter::once(self.cfg.model.as_str()).chain(fallback_list(&self.cfg.fallback_model));
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

/// `"a, b,,c"` -> `["a", "b", "c"]`.
fn fallback_list(raw: &Option<String>) -> impl Iterator<Item = &str> {
    raw.iter()
        .flat_map(|s| s.split(','))
        .map(str::trim)
        .filter(|s| !s.is_empty())
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
    fn fallback_aceita_lista_separada_por_virgula() {
        let raw = Some(" gemini-3.5-flash-lite, ,gemini-3.1-flash-lite ".to_string());
        let v: Vec<&str> = fallback_list(&raw).collect();
        assert_eq!(v, ["gemini-3.5-flash-lite", "gemini-3.1-flash-lite"]);
        assert_eq!(fallback_list(&None).count(), 0);
        let padrao = Some(GeminiConfig::DEFAULT_FALLBACK_MODEL.to_string());
        assert_eq!(fallback_list(&padrao).count(), 2);
    }

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

/// Testes de borda das funções privadas do cliente Gemini. Sem rede.
#[cfg(test)]
mod tests_bordas {
    use super::*;

    fn extrator() -> GeminiExtractor {
        GeminiExtractor::new(GeminiConfig {
            api_key: "chave-sintetica-123".into(),
            model: GeminiConfig::DEFAULT_MODEL.into(),
            fallback_model: None,
            base_url: GeminiConfig::DEFAULT_BASE_URL.into(),
        })
    }

    // ---------- fallback_list ----------

    #[test]
    fn fallback_vazio_so_virgulas_ou_so_espacos_nao_gera_modelos() {
        for raw in ["", " ", ",", ",,,", " , , ", "\t,\n"] {
            let v = Some(raw.to_string());
            assert_eq!(fallback_list(&v).count(), 0, "raw {raw:?}");
        }
    }

    #[test]
    fn fallback_com_um_unico_modelo_e_com_espacos_unicode() {
        let v = Some("\u{a0}gemini-x\u{2003}".to_string());
        assert_eq!(fallback_list(&v).collect::<Vec<_>>(), ["gemini-x"]);
    }

    #[test]
    fn fallback_mantem_ordem_e_nao_remove_duplicados() {
        let v = Some("b,a,b".to_string());
        assert_eq!(fallback_list(&v).collect::<Vec<_>>(), ["b", "a", "b"]);
    }

    #[test]
    fn fallback_nao_divide_por_ponto_e_virgula_nem_espaco() {
        let v = Some("a;b c".to_string());
        assert_eq!(fallback_list(&v).collect::<Vec<_>>(), ["a;b c"]);
    }

    #[test]
    fn fallback_padrao_nao_repete_o_modelo_principal() {
        let lista: Vec<&str> = GeminiConfig::DEFAULT_FALLBACK_MODEL.split(',').collect();
        assert!(!lista.contains(&GeminiConfig::DEFAULT_MODEL));
    }

    // ---------- backoff e constantes de tempo ----------

    #[test]
    fn backoff_segue_potencias_de_tres() {
        let esperado = [1, 3, 9, 27, 81];
        for (i, s) in esperado.iter().enumerate() {
            assert_eq!(backoff(i as u32 + 1).as_secs(), *s);
        }
    }

    #[test]
    fn backoff_aguenta_tentativas_altas_sem_overflow() {
        // 3^40 ainda cabe em u64; é o maior expoente seguro.
        assert_eq!(backoff(41).as_secs(), 3u64.pow(40));
    }

    const _: () = {
        assert!(GeminiConfig::ATTEMPTS_PER_MODEL >= 1);
        // A chamada individual precisa caber no orçamento total, senão o orçamento nunca vale.
        assert!(GeminiConfig::CALL_TIMEOUT.as_secs() < GeminiConfig::TOTAL_BUDGET.as_secs());
    };

    #[test]
    fn backoff_maximo_da_configuracao_cabe_no_orcamento() {
        let pior_espera = backoff(GeminiConfig::ATTEMPTS_PER_MODEL - 1);
        assert!(pior_espera < GeminiConfig::TOTAL_BUDGET);
    }

    #[test]
    fn url_base_padrao_e_https_sem_barra_final() {
        assert!(GeminiConfig::DEFAULT_BASE_URL.starts_with("https://"));
        assert!(!GeminiConfig::DEFAULT_BASE_URL.ends_with('/'));
    }

    // ---------- response_text ----------

    #[test]
    fn resposta_sem_candidatos_ou_com_formato_errado_nao_tem_texto() {
        for v in [
            json!(null),
            json!([]),
            json!({"candidates": null}),
            json!({"candidates": []}),
            json!({"candidates": {}}),
            json!({"candidates": [null]}),
            json!({"candidates": [{}]}),
            json!({"candidates": [{"content": {}}]}),
            json!({"candidates": [{"content": {"parts": null}}]}),
            json!({"candidates": [{"content": {"parts": "texto"}}]}),
            json!({"candidates": [{"content": {"parts": []}}]}),
        ] {
            assert_eq!(response_text(&v), None, "valor {v}");
        }
    }

    #[test]
    fn resposta_bloqueada_por_seguranca_nao_tem_texto() {
        let v = json!({"candidates": [{"finishReason": "SAFETY"}],
                       "promptFeedback": {"blockReason": "SAFETY"}});
        assert_eq!(response_text(&v), None);
    }

    #[test]
    fn partes_vazias_ou_sem_texto_sao_ignoradas() {
        let v = json!({"candidates": [{"content": {"parts": [
            {"text": ""}, {"inlineData": {"mimeType": "x", "data": "y"}},
            {"text": 42}, {"text": null}, {"text": "ok"}, {}
        ]}}]});
        assert_eq!(response_text(&v).as_deref(), Some("ok"));
    }

    #[test]
    fn todas_as_partes_vazias_resultam_em_none() {
        let v = json!({"candidates": [{"content": {"parts": [{"text": ""}, {"text": ""}]}}]});
        assert_eq!(response_text(&v), None);
    }

    #[test]
    fn so_o_primeiro_candidato_e_usado() {
        let v = json!({"candidates": [
            {"content": {"parts": [{"text": "primeiro"}]}},
            {"content": {"parts": [{"text": "segundo"}]}}
        ]});
        assert_eq!(response_text(&v).as_deref(), Some("primeiro"));
    }

    #[test]
    fn candidato_vazio_nao_cai_para_o_segundo() {
        let v = json!({"candidates": [
            {"content": {"parts": []}},
            {"content": {"parts": [{"text": "segundo"}]}}
        ]});
        assert_eq!(response_text(&v), None);
    }

    #[test]
    fn texto_unicode_e_espacos_sao_preservados_e_concatenados_sem_separador() {
        let v = json!({"candidates": [{"content": {"parts": [
            {"text": "ação "}, {"text": "🚀\n"}, {"text": " fim"}
        ]}}]});
        assert_eq!(response_text(&v).as_deref(), Some("ação 🚀\n fim"));
    }

    #[test]
    fn texto_so_com_espacos_conta_como_texto() {
        // Só a string vazia vira None; "  " segue adiante e falha no parse depois.
        let v = json!({"candidates": [{"content": {"parts": [{"text": "  "}]}}]});
        assert_eq!(response_text(&v).as_deref(), Some("  "));
    }

    #[test]
    fn resposta_enorme_e_concatenada_por_inteiro() {
        let parte = "x".repeat(1_000_000);
        let v = json!({"candidates": [{"content": {"parts": [{"text": parte}, {"text": parte}]}}]});
        assert_eq!(response_text(&v).unwrap().len(), 2_000_000);
    }

    // ---------- request_body ----------

    #[test]
    fn corpo_sem_retry_tem_arquivo_e_instrucao_e_nada_mais() {
        let b = extrator().request_body(b"%PDF", InputMime::Pdf, None);
        let parts = b.pointer("/contents/0/parts").unwrap().as_array().unwrap();
        assert_eq!(parts.len(), 2);
        assert!(parts[0].get("inline_data").is_some());
        assert!(parts[1]["text"].as_str().unwrap().contains("Extraia"));
    }

    #[test]
    fn corpo_com_retry_acrescenta_a_nota_depois_do_prefixo() {
        let b = extrator().request_body(b"x", InputMime::Png, Some("o JSON não segue o schema"));
        let parts = b.pointer("/contents/0/parts").unwrap().as_array().unwrap();
        assert_eq!(parts.len(), 3);
        let t = parts[2]["text"].as_str().unwrap();
        assert!(t.starts_with(prompt::RETRY));
        assert!(t.contains("o JSON não segue o schema"));
        assert!(t.ends_with("Corrija e responda de novo."));
    }

    #[test]
    fn nota_de_retry_com_aspas_quebras_e_unicode_continua_json_valido() {
        let nota = "linha1\nlinha2 \"aspas\" \\ barra \u{0} nulo 🚀 日本語";
        let b = extrator().request_body(b"x", InputMime::Jpeg, Some(nota));
        let txt = b.to_string();
        let volta: Value = serde_json::from_str(&txt).unwrap();
        assert_eq!(volta, b);
        assert!(
            volta
                .pointer("/contents/0/parts/2/text")
                .unwrap()
                .as_str()
                .unwrap()
                .contains(nota)
        );
    }

    #[test]
    fn nota_de_retry_vazia_ainda_gera_a_parte_de_texto() {
        let b = extrator().request_body(b"x", InputMime::Webp, Some(""));
        let parts = b.pointer("/contents/0/parts").unwrap().as_array().unwrap();
        assert_eq!(parts.len(), 3);
    }

    #[test]
    fn arquivo_vazio_gera_base64_vazio_sem_panic() {
        let b = extrator().request_body(&[], InputMime::Pdf, None);
        assert_eq!(
            b.pointer("/contents/0/parts/0/inline_data/data").unwrap(),
            ""
        );
    }

    #[test]
    fn todos_os_256_valores_de_byte_voltam_iguais_do_base64() {
        let bytes: Vec<u8> = (0..=255u8).collect();
        let b = extrator().request_body(&bytes, InputMime::Png, None);
        let data = b
            .pointer("/contents/0/parts/0/inline_data/data")
            .unwrap()
            .as_str()
            .unwrap();
        assert_eq!(STANDARD.decode(data).unwrap(), bytes);
    }

    #[test]
    fn tamanho_do_base64_acompanha_o_padding_para_tamanhos_de_borda() {
        for n in [1usize, 2, 3, 4, 5, 1023, 1024, 1025] {
            let b = extrator().request_body(&vec![7u8; n], InputMime::Pdf, None);
            let d = b
                .pointer("/contents/0/parts/0/inline_data/data")
                .unwrap()
                .as_str()
                .unwrap();
            assert_eq!(d.len(), n.div_ceil(3) * 4, "n={n}");
        }
    }

    #[test]
    fn cada_mime_vai_para_o_inline_data() {
        for m in [
            InputMime::Pdf,
            InputMime::Png,
            InputMime::Jpeg,
            InputMime::Webp,
        ] {
            let b = extrator().request_body(b"x", m, None);
            assert_eq!(
                b.pointer("/contents/0/parts/0/inline_data/mime_type")
                    .unwrap(),
                m.as_str()
            );
        }
    }

    #[test]
    fn corpo_nunca_carrega_a_chave_de_api() {
        let b = extrator()
            .request_body(b"conteudo", InputMime::Pdf, Some("nota"))
            .to_string();
        assert!(!b.contains("chave-sintetica-123"));
        assert!(!b.contains("x-goog-api-key"));
    }

    #[test]
    fn corpo_forca_json_com_temperatura_zero_e_instrucao_de_sistema() {
        let b = extrator().request_body(b"x", InputMime::Pdf, None);
        assert_eq!(
            b.pointer("/generationConfig/responseMimeType").unwrap(),
            "application/json"
        );
        assert_eq!(b.pointer("/generationConfig/temperature").unwrap(), 0);
        assert_eq!(
            b.pointer("/system_instruction/parts/0/text")
                .unwrap()
                .as_str()
                .unwrap(),
            prompt::SYSTEM
        );
        assert_eq!(b.pointer("/contents/0/role").unwrap(), "user");
    }

    #[test]
    fn corpo_e_deterministico_para_a_mesma_entrada() {
        // A chave do cache depende disso.
        let e = extrator();
        let a = e.request_body(b"abc", InputMime::Pdf, None).to_string();
        let b = e.request_body(b"abc", InputMime::Pdf, None).to_string();
        assert_eq!(a, b);
        let c = e.request_body(b"abd", InputMime::Pdf, None).to_string();
        assert_ne!(a, c);
    }

    #[test]
    fn contador_de_chamadas_comeca_em_zero() {
        assert_eq!(extrator().http_calls(), 0);
    }
}
