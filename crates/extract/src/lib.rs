//! Extração de faturas: PDF/imagem -> `ExtractedBill` via IA.
//!
//! - Nunca extrai nome, CPF, endereço ou nº de instalação/linha (RNF04): o
//!   schema enviado ao modelo simplesmente não tem esses campos.
//! - O conteúdo da fatura é tratado como dado, nunca como instrução (RNF07):
//!   a saída é forçada por JSON Schema e validada com `serde` estrito.
//! - Nunca loga o conteúdo extraído (RNF13).

mod gemini;
mod prompt;
pub mod schema;

use async_trait::async_trait;
use humanobill_core::{ExtractedBill, ValidationIssue};

pub use gemini::{GeminiConfig, GeminiExtractor};

/// Tipos de arquivo aceitos (detectados pelos magic bytes na API).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMime {
    Pdf,
    Png,
    Jpeg,
    Webp,
}

impl InputMime {
    pub fn as_str(self) -> &'static str {
        match self {
            InputMime::Pdf => "application/pdf",
            InputMime::Png => "image/png",
            InputMime::Jpeg => "image/jpeg",
            InputMime::Webp => "image/webp",
        }
    }

    pub fn from_mime(mime: &str) -> Option<Self> {
        match mime {
            "application/pdf" => Some(InputMime::Pdf),
            "image/png" => Some(InputMime::Png),
            "image/jpeg" => Some(InputMime::Jpeg),
            "image/webp" => Some(InputMime::Webp),
            _ => None,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ExtractError {
    /// O provedor não está configurado (ex.: sem chave de API).
    #[error("extração indisponível: {0}")]
    Unavailable(String),
    /// Quota do provedor esgotada (429).
    #[error("quota do provedor esgotada")]
    RateLimited,
    /// Falha de rede ou resposta HTTP inesperada.
    #[error("falha ao chamar o provedor: {0}")]
    Provider(String),
    /// A IA devolveu algo fora do schema ou inconsistente, mesmo após o retry.
    #[error("fatura ilegível ou extração inválida")]
    Invalid { issues: Vec<ValidationIssue> },
}

#[async_trait]
pub trait BillExtractor: Send + Sync {
    async fn extract(&self, bytes: &[u8], mime: InputMime) -> Result<ExtractedBill, ExtractError>;
}

/// Converte a resposta textual do modelo em `ExtractedBill` e valida a consistência.
pub fn parse_and_validate(text: &str) -> Result<ExtractedBill, Vec<ValidationIssue>> {
    let mut bill: ExtractedBill =
        serde_json::from_str(strip_code_fence(text)).map_err(|_| Vec::new())?;
    // RF06: garante o limite de 25 palavras mesmo se a IA passar dele.
    for item in &mut bill.items {
        item.plain_explanation = humanobill_core::tidy_explanation(&item.plain_explanation);
    }
    let issues = humanobill_core::validate_bill(&bill);
    if issues.is_empty() {
        Ok(bill)
    } else {
        Err(issues)
    }
}

/// Alguns modelos embrulham o JSON em ```json ... ```.
fn strip_code_fence(s: &str) -> &str {
    let t = s.trim();
    let t = t
        .strip_prefix("```json")
        .or_else(|| t.strip_prefix("```"))
        .unwrap_or(t);
    t.strip_suffix("```").unwrap_or(t).trim()
}

#[cfg(test)]
mod tests {
    use super::*;

    const OK: &str = r#"{"kind":"telecom","provider_name":"Vivaz Telecom","reference_month":"2026-10",
      "due_date":"2026-11-10","total_cents":9990,"items":[{"id":"i1","description":"Plano",
      "amount_cents":9990,"category":"plano","original_text":"PLANO 99,90","page":1,
      "plain_explanation":"Mensalidade do plano."}],"extraction_confidence":0.9}"#;

    #[test]
    fn explicacao_longa_da_ia_e_cortada() {
        let longa = (1..=40)
            .map(|i| format!("p{i}"))
            .collect::<Vec<_>>()
            .join(" ");
        let json = OK.replace("Mensalidade do plano.", &longa);
        let bill = parse_and_validate(&json).unwrap();
        assert!(humanobill_core::is_plain_explanation(
            &bill.items[0].plain_explanation
        ));
    }

    #[test]
    fn aceita_json_valido() {
        assert_eq!(parse_and_validate(OK).unwrap().total_cents, 9990);
    }

    #[test]
    fn aceita_json_em_bloco_de_codigo() {
        let fenced = format!("```json\n{OK}\n```");
        assert!(parse_and_validate(&fenced).is_ok());
    }

    #[test]
    fn rejeita_json_quebrado() {
        assert!(parse_and_validate("não é json").is_err());
    }

    #[test]
    fn rejeita_soma_divergente() {
        let bad = OK.replace("\"total_cents\":9990", "\"total_cents\":19990");
        let issues = parse_and_validate(&bad).unwrap_err();
        assert!(matches!(issues[..], [ValidationIssue::SumMismatch { .. }]));
    }

    #[test]
    fn mime_ida_e_volta() {
        for m in [
            InputMime::Pdf,
            InputMime::Png,
            InputMime::Jpeg,
            InputMime::Webp,
        ] {
            assert_eq!(InputMime::from_mime(m.as_str()), Some(m));
        }
        assert_eq!(InputMime::from_mime("text/html"), None);
    }
}
