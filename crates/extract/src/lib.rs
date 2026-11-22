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
    let mut raw: serde_json::Value =
        serde_json::from_str(strip_code_fence(text)).map_err(|_| Vec::new())?;
    box_2d_to_bbox(&mut raw);
    remove_malformed_bboxes(&mut raw);
    let mut bill: ExtractedBill = serde_json::from_value(raw).map_err(|_| Vec::new())?;
    // RF06: garante o limite de 25 palavras mesmo se a IA passar dele.
    for item in &mut bill.items {
        item.plain_explanation = humanobill_core::tidy_explanation(&item.plain_explanation);
    }
    // #51: bbox fora de 0–1 ou sem área é descartado; a extração segue.
    let dropped = humanobill_core::drop_invalid_bboxes(&mut bill);
    if dropped > 0 {
        tracing::info!(dropped, "bbox inválido descartado");
    }
    let issues = humanobill_core::validate_bill(&bill);
    if issues.is_empty() {
        Ok(bill)
    } else {
        Err(issues)
    }
}

/// #51: o Gemini devolve a caixa no formato nativo `box_2d` =
/// `[ymin, xmin, ymax, xmax]`, inteiros de 0 a 1000. Aqui ela vira o `bbox` do
/// core (`page`, `x`, `y`, `w`, `h` de 0 a 1), na página do próprio item.
/// Formato errado (tamanho, tipo) some; caixa invertida ou fora da página cai
/// depois, em `drop_invalid_bboxes`.
fn box_2d_to_bbox(raw: &mut serde_json::Value) {
    let Some(items) = raw.get_mut("items").and_then(|v| v.as_array_mut()) else {
        return;
    };
    for item in items.iter_mut().filter_map(|i| i.as_object_mut()) {
        let Some(b) = item.remove("box_2d") else {
            continue;
        };
        if item.contains_key("bbox") {
            continue;
        }
        let nums: Option<Vec<f64>> = b
            .as_array()
            .filter(|a| a.len() == 4)
            .and_then(|a| a.iter().map(serde_json::Value::as_f64).collect());
        let page = item.get("page").and_then(serde_json::Value::as_u64);
        if let (Some(n), Some(page)) = (nums, page) {
            let [ymin, xmin, ymax, xmax] = [n[0], n[1], n[2], n[3]].map(|v| v / 1000.0);
            item.insert(
                "bbox".into(),
                serde_json::json!({
                    "page": page, "x": xmin, "y": ymin, "w": xmax - xmin, "h": ymax - ymin
                }),
            );
        }
    }
}

/// #51: um `bbox` com formato quebrado (campo faltando, página negativa, texto
/// no lugar de número) é tirado do item antes do parse, para não derrubar a
/// extração inteira. Valores fora de 0–1 são tratados depois, no core.
fn remove_malformed_bboxes(raw: &mut serde_json::Value) {
    let Some(items) = raw.get_mut("items").and_then(|v| v.as_array_mut()) else {
        return;
    };
    for item in items.iter_mut().filter_map(|i| i.as_object_mut()) {
        let malformed = item.get("bbox").is_some_and(|b| {
            !b.is_null() && serde_json::from_value::<humanobill_core::BBox>(b.clone()).is_err()
        });
        if malformed {
            item.remove("bbox");
        }
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

/// Testes de borda de `strip_code_fence` e do contrato de erro (funções privadas).
#[cfg(test)]
mod tests_bordas {
    use super::*;

    #[test]
    fn cerca_vazia_ou_so_espacos_vira_string_vazia() {
        assert_eq!(strip_code_fence(""), "");
        assert_eq!(strip_code_fence("   \n\t "), "");
        assert_eq!(strip_code_fence("```"), "");
        assert_eq!(strip_code_fence("``````"), "");
        assert_eq!(strip_code_fence("```json```"), "");
    }

    #[test]
    fn json_sem_cerca_e_devolvido_sem_espacos_nas_pontas() {
        assert_eq!(strip_code_fence("  {\"a\":1}\n"), "{\"a\":1}");
    }

    #[test]
    fn cerca_json_e_cerca_simples_sao_removidas() {
        assert_eq!(strip_code_fence("```json\n{\"a\":1}\n```"), "{\"a\":1}");
        assert_eq!(strip_code_fence("```\n{\"a\":1}\n```"), "{\"a\":1}");
        assert_eq!(strip_code_fence("```json{\"a\":1}```"), "{\"a\":1}");
    }

    #[test]
    fn cerca_so_na_abertura_ou_so_no_fechamento_e_tolerada() {
        assert_eq!(strip_code_fence("```json\n{\"a\":1}"), "{\"a\":1}");
        assert_eq!(strip_code_fence("{\"a\":1}\n```"), "{\"a\":1}");
    }

    #[test]
    fn so_uma_camada_de_cerca_e_removida() {
        assert_eq!(
            strip_code_fence("```json\n```json\n{}\n```\n```"),
            "```json\n{}\n```"
        );
    }

    #[test]
    fn rotulo_de_linguagem_em_maiuscula_nao_e_reconhecido() {
        // Só "```json" em minúsculas é tratado: com "JSON" sobra o rótulo no texto,
        // e o parse falha. Documenta o comportamento atual.
        assert_eq!(strip_code_fence("```JSON\n{}\n```"), "JSON\n{}");
        assert!(parse_and_validate("```JSON\n{}\n```").is_err());
    }

    #[test]
    fn crases_dentro_do_json_nao_sao_confundidas_com_cerca() {
        let s = r#"{"a":"```x```"}"#;
        // Não começa nem termina com crases: fica intacto.
        assert_eq!(strip_code_fence(s), s);
    }

    #[test]
    fn unicode_ao_redor_da_cerca_nao_causa_panic_de_fatiamento() {
        for s in [
            "```json\nação 日本語 🚀\n```",
            "日本語```",
            "```日本語",
            "🚀",
            "é```é",
        ] {
            let _ = strip_code_fence(s);
        }
        assert_eq!(strip_code_fence("```json\nação 🚀\n```"), "ação 🚀");
    }

    #[test]
    fn quebras_de_linha_do_windows_sao_toleradas() {
        assert_eq!(strip_code_fence("```json\r\n{}\r\n```\r\n"), "{}");
    }

    #[test]
    fn invalid_com_issues_vazias_tem_mensagem_amigavel() {
        let e = ExtractError::Invalid { issues: vec![] };
        assert_eq!(e.to_string(), "fatura ilegível ou extração inválida");
    }

    #[test]
    fn mensagens_de_erro_carregam_o_detalhe_do_provedor() {
        assert_eq!(
            ExtractError::Unavailable("sem chave".into()).to_string(),
            "extração indisponível: sem chave"
        );
        assert_eq!(
            ExtractError::Provider("HTTP 500".into()).to_string(),
            "falha ao chamar o provedor: HTTP 500"
        );
        assert_eq!(
            ExtractError::RateLimited.to_string(),
            "quota do provedor esgotada"
        );
    }

    #[test]
    fn mime_from_mime_e_exato_e_nao_aceita_prefixos() {
        for m in ["application/pd", "application/pdfx", "image/", "image", "/"] {
            assert_eq!(InputMime::from_mime(m), None, "mime {m:?}");
        }
    }
}

/// #51: a caixa do item é opcional e, se vier inválida, é descartada.
#[cfg(test)]
mod tests_bbox {
    use super::*;

    /// Fatura de um item com `campo` (`box_2d` ou `bbox`) igual a `valor` (JSON cru).
    fn com(campo: &str, valor: &str) -> String {
        format!(
            r#"{{"kind":"telecom","provider_name":"Vivaz Telecom","reference_month":"2026-10",
      "due_date":"2026-11-10","total_cents":9990,"items":[{{"id":"i1","description":"Plano",
      "amount_cents":9990,"category":"plano","original_text":"PLANO 99,90","page":1,
      "plain_explanation":"Mensalidade do plano.","{campo}":{valor}}}],"extraction_confidence":0.9}}"#
        )
    }

    #[test]
    fn box_2d_do_gemini_vira_bbox_normalizado_na_pagina_do_item() {
        let bill = parse_and_validate(&com("box_2d", "[258, 73, 272, 921]")).unwrap();
        let b = bill.items[0].bbox.expect("bbox");
        assert_eq!(b.page, 1);
        assert!((b.x - 0.073).abs() < 1e-6 && (b.y - 0.258).abs() < 1e-6);
        assert!((b.w - 0.848).abs() < 1e-6 && (b.h - 0.014).abs() < 1e-6);
    }

    #[test]
    fn box_2d_invalido_vira_none_e_a_fatura_passa() {
        for ruim in [
            "[1400, 100, 1420, 900]", // fora da página
            "[300, 100, 300, 900]",   // altura zero
            "[300, 900, 320, 100]",   // invertida
            "[-10, 100, 20, 900]",
            "[1, 2, 3]",
            "[1, 2, 3, 4, 5]",
            r#"["a", 2, 3, 4]"#,
            r#"{"ymin": 1}"#,
            r#""caixa""#,
        ] {
            let bill = parse_and_validate(&com("box_2d", ruim)).unwrap();
            assert_eq!(bill.items[0].bbox, None, "{ruim}");
        }
    }

    #[test]
    fn bbox_ja_no_formato_do_core_e_aceito_e_validado() {
        let bill = parse_and_validate(&com(
            "bbox",
            r#"{"page":1,"x":0.1,"y":0.2,"w":0.8,"h":0.03}"#,
        ))
        .unwrap();
        assert_eq!(bill.items[0].bbox.map(|b| b.w), Some(0.8));
        for ruim in [
            r#"{"page":1,"x":-0.1,"y":0.2,"w":0.8,"h":0.03}"#,
            r#"{"page":1,"x":0.1,"y":0.2,"w":0,"h":0.03}"#,
            r#"{"page":0,"x":0.1,"y":0.2,"w":0.8,"h":0.03}"#,
            r#"{"page":-2,"x":0.1,"y":0.2,"w":0.8,"h":0.03}"#,
            r#"{"x":0.1}"#,
            r#"{"page":1,"x":"0.1","y":0.2,"w":0.8,"h":0.03}"#,
            r#"[0.1,0.2,0.8,0.03]"#,
        ] {
            let bill = parse_and_validate(&com("bbox", ruim)).unwrap();
            assert_eq!(bill.items[0].bbox, None, "{ruim}");
        }
    }

    #[test]
    fn sem_caixa_ou_null_passa() {
        assert_eq!(
            parse_and_validate(&com("box_2d", "null")).unwrap().items[0].bbox,
            None
        );
        assert_eq!(
            parse_and_validate(&com("bbox", "null")).unwrap().items[0].bbox,
            None
        );
    }
}
