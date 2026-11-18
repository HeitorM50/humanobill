//! Tipos de domínio do HumanoBill: fatura extraída, itens, achados e análise.
//!
//! - Valores monetários sempre em centavos (`Cents = i64`), nunca float.
//! - Nenhum tipo carrega nome, CPF, endereço ou nº de instalação/linha (RNF04).
//! - Os tipos são exportados para TypeScript (`apps/web/src/types`) via `ts-rs`
//!   ao rodar `cargo test -p humanobill-core`.

use std::collections::BTreeMap;

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

mod validate;
pub use validate::{ValidationIssue, validate_bill};

/// Valor monetário em centavos de real.
pub type Cents = i64;

/// Tolerância de arredondamento ao comparar somas (R$ 0,05).
pub const SUM_TOLERANCE_CENTS: Cents = 5;

/// RF06: explicações têm no máximo 25 palavras.
pub const MAX_EXPLANATION_WORDS: usize = 25;
/// RF06: termos de juridiquês que nunca devem aparecer nas explicações.
pub const FORBIDDEN_TERMS: [&str; 5] = [
    "supramencionad",
    "outrossim",
    "ensejar",
    "doravante",
    "destarte",
];

/// A explicação respeita o limite de palavras e não tem juridiquês.
pub fn is_plain_explanation(s: &str) -> bool {
    let lower = s.to_lowercase();
    s.split_whitespace().count() <= MAX_EXPLANATION_WORDS
        && !FORBIDDEN_TERMS.iter().any(|t| lower.contains(t))
}

/// Corta a explicação em 25 palavras (com reticências) se a IA passar do limite.
pub fn tidy_explanation(s: &str) -> String {
    let words: Vec<&str> = s.split_whitespace().collect();
    if words.len() <= MAX_EXPLANATION_WORDS {
        return words.join(" ");
    }
    let mut out = words[..MAX_EXPLANATION_WORDS].join(" ");
    // Tira pontuação e espaços do fim: se a 25ª palavra era só pontuação, não
    // pode sobrar espaço antes das reticências.
    out = out
        .trim_end_matches(|c: char| matches!(c, ',' | ';' | ':' | '.') || c.is_whitespace())
        .to_string();
    out.push('…');
    out
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum BillKind {
    Telecom,
    Energy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum ItemCategory {
    Plano,
    ServicoAdicional,
    EncargoAtraso,
    Imposto,
    Energia,
    Bandeira,
    IluminacaoPublica,
    Outros,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Period {
    #[ts(type = "string")]
    pub start: NaiveDate,
    #[ts(type = "string")]
    pub end: NaiveDate,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct BillItem {
    /// Identificador estável dentro da fatura (ex.: "i1").
    pub id: String,
    pub description: String,
    #[ts(type = "number")]
    pub amount_cents: Cents,
    pub category: ItemCategory,
    #[serde(default)]
    pub period: Option<Period>,
    /// Trecho exatamente como aparece na fatura (RF08).
    pub original_text: String,
    /// Página da fatura onde o item aparece (1-based).
    pub page: u32,
    /// Explicação em linguagem simples (RF06).
    pub plain_explanation: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct LateFees {
    /// Valor em atraso sobre o qual incidem multa e juros.
    #[ts(type = "number")]
    pub base_cents: Cents,
    #[ts(type = "number")]
    pub fine_cents: Cents,
    #[ts(type = "number")]
    pub interest_cents: Cents,
    pub days_late: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum ReadingType {
    Real,
    Estimada,
    Media,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum TariffFlag {
    #[serde(rename = "verde")]
    Verde,
    #[serde(rename = "amarela")]
    Amarela,
    #[serde(rename = "vermelha_1")]
    Vermelha1,
    #[serde(rename = "vermelha_2")]
    Vermelha2,
    #[serde(rename = "escassez")]
    Escassez,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ConsumptionPoint {
    /// Mês no formato "YYYY-MM".
    pub month: String,
    pub kwh: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct EnergyData {
    pub consumption_kwh: u32,
    pub reading_type: ReadingType,
    #[ts(type = "string")]
    pub reading_prev_date: NaiveDate,
    #[ts(type = "string")]
    pub reading_curr_date: NaiveDate,
    #[serde(default)]
    pub tariff_flag: Option<TariffFlag>,
    /// Histórico impresso na fatura (até 12 meses).
    #[serde(default)]
    pub history: Vec<ConsumptionPoint>,
}

/// Fatura extraída pela IA. Não contém dados pessoais do titular (RNF04).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ExtractedBill {
    pub kind: BillKind,
    pub provider_name: String,
    /// Mês de referência no formato "YYYY-MM".
    pub reference_month: String,
    #[ts(type = "string")]
    pub due_date: NaiveDate,
    #[serde(default)]
    #[ts(type = "string | null")]
    pub issue_date: Option<NaiveDate>,
    #[ts(type = "number")]
    pub total_cents: Cents,
    #[serde(default)]
    pub plan_name: Option<String>,
    pub items: Vec<BillItem>,
    #[serde(default)]
    pub late_fees: Option<LateFees>,
    #[serde(default)]
    pub energy: Option<EnergyData>,
    /// Confiança declarada pela extração (0 a 1).
    pub extraction_confidence: f32,
}

/// Severidade de um achado. Serializa em português para a UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum Severity {
    /// Regra objetiva violada.
    #[serde(rename = "indevido")]
    Unlawful,
    /// Indício que depende de confirmação.
    #[serde(rename = "suspeito")]
    Suspicious,
    #[serde(rename = "ok")]
    Ok,
}

/// Status de validação da base legal de uma regra (docs/03).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum RuleStatus {
    Validated,
    /// "A validar": nunca pode emitir `Severity::Unlawful`.
    Pending,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct LegalRef {
    pub label: String,
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Finding {
    /// Id estável dentro da análise: "{rule_id}:{item_refs unidos por +}" (ex.: "TEL-01:i2").
    #[serde(default)]
    pub id: String,
    pub rule_id: String,
    pub severity: Severity,
    pub title: String,
    pub explanation: String,
    pub item_refs: Vec<String>,
    #[serde(default)]
    pub original_text: Option<String>,
    #[serde(default)]
    pub page: Option<u32>,
    #[ts(type = "number")]
    pub amount_cents: Cents,
    pub legal_basis: Vec<LegalRef>,
}

/// Um passo do roteiro "Onde reclamar" (RF12).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct DisputeStep {
    pub title: String,
    pub description: String,
    #[serde(default)]
    pub url: Option<String>,
}

/// Contestação pronta (RF11) + roteiro de onde reclamar (RF12).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Dispute {
    pub text: String,
    /// Soma dos achados incluídos.
    #[ts(type = "number")]
    pub total_cents: Cents,
    pub steps: Vec<DisputeStep>,
    /// Prazos importantes, em frases curtas.
    pub deadlines: Vec<String>,
    pub disclaimer: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum QuestionKind {
    YesNo,
    Amount,
    Date,
}

/// Pergunta de confirmação gerada por uma regra (RF09).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Question {
    /// Id estável, usado como chave em `Answers` (ex.: "TEL-01:i3").
    pub id: String,
    pub rule_id: String,
    #[serde(default)]
    pub item_id: Option<String>,
    pub text: String,
    pub kind: QuestionKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
#[ts(export)]
pub enum Answer {
    Yes,
    No,
    Amount(#[ts(type = "number")] Cents),
    Date(#[ts(type = "string")] NaiveDate),
}

/// Respostas do usuário, indexadas por `Question::id`.
pub type Answers = BTreeMap<String, Answer>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum AnalysisSource {
    Upload,
    Demo,
}

/// Resultado de uma análise, como a API devolve ao front.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Analysis {
    pub id: String,
    /// RFC 3339.
    pub created_at: String,
    /// RFC 3339.
    pub expires_at: String,
    pub source: AnalysisSource,
    pub bill: ExtractedBill,
    pub answers: Answers,
    #[serde(default)]
    pub paid: Option<bool>,
    pub findings: Vec<Finding>,
    pub questions: Vec<Question>,
    #[ts(type = "number")]
    pub recoverable_cents: Cents,
    pub rules_version: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(crate) fn sample_bill() -> ExtractedBill {
        ExtractedBill {
            kind: BillKind::Telecom,
            provider_name: "Vivaz Telecom".into(),
            reference_month: "2026-10".into(),
            due_date: NaiveDate::from_ymd_opt(2026, 11, 10).unwrap(),
            issue_date: None,
            total_cents: 12_280,
            plan_name: Some("Vivaz Controle 20GB".into()),
            items: vec![
                BillItem {
                    id: "i1".into(),
                    description: "Plano Vivaz Controle 20GB".into(),
                    amount_cents: 9_990,
                    category: ItemCategory::Plano,
                    period: None,
                    original_text: "PLANO VZ CONTROLE 20GB 99,90".into(),
                    page: 1,
                    plain_explanation: "Mensalidade do seu plano de celular.".into(),
                },
                BillItem {
                    id: "i2".into(),
                    description: "Clube Vivaz Música".into(),
                    amount_cents: 1_290,
                    category: ItemCategory::ServicoAdicional,
                    period: None,
                    original_text: "CLUBE VZ MUS 12,90".into(),
                    page: 1,
                    plain_explanation: "Serviço extra de música cobrado à parte.".into(),
                },
                BillItem {
                    id: "i3".into(),
                    description: "Seguro Tela".into(),
                    amount_cents: 1_000,
                    category: ItemCategory::ServicoAdicional,
                    period: None,
                    original_text: "SEG TELA 10,00".into(),
                    page: 1,
                    plain_explanation: "Seguro para a tela do aparelho.".into(),
                },
            ],
            late_fees: None,
            energy: None,
            extraction_confidence: 0.93,
        }
    }

    #[test]
    fn explicacao_longa_e_cortada_em_25_palavras() {
        let longa = (1..=40)
            .map(|i| format!("p{i}"))
            .collect::<Vec<_>>()
            .join(" ");
        let t = tidy_explanation(&longa);
        assert_eq!(t.split_whitespace().count(), MAX_EXPLANATION_WORDS);
        assert!(t.ends_with('…'));
        assert!(is_plain_explanation(&t));
        assert_eq!(
            tidy_explanation("  Mensalidade   do plano. "),
            "Mensalidade do plano."
        );
    }

    #[test]
    fn juridiques_e_rejeitado() {
        assert!(!is_plain_explanation("Valor supramencionado na fatura."));
        assert!(is_plain_explanation(
            "Imposto estadual cobrado sobre a energia."
        ));
    }

    #[test]
    fn bill_roundtrip_json() {
        let bill = sample_bill();
        let json = serde_json::to_string(&bill).unwrap();
        let back: ExtractedBill = serde_json::from_str(&json).unwrap();
        assert_eq!(bill, back);
        assert!(json.contains("\"servico_adicional\""));
    }

    #[test]
    fn severity_serializa_em_portugues() {
        assert_eq!(
            serde_json::to_string(&Severity::Unlawful).unwrap(),
            "\"indevido\""
        );
        assert_eq!(
            serde_json::to_string(&Severity::Suspicious).unwrap(),
            "\"suspeito\""
        );
    }

    #[test]
    fn tariff_flag_vermelha() {
        assert_eq!(
            serde_json::to_string(&TariffFlag::Vermelha1).unwrap(),
            "\"vermelha_1\""
        );
    }

    #[test]
    fn answer_formato_tagged() {
        let a = Answer::Amount(9_990);
        assert_eq!(
            serde_json::to_string(&a).unwrap(),
            r#"{"type":"amount","value":9990}"#
        );
        assert_eq!(
            serde_json::to_string(&Answer::No).unwrap(),
            r#"{"type":"no"}"#
        );
    }

    /// RNF04: o schema não pode ter campos de dado pessoal.
    #[test]
    fn schema_sem_campos_pessoais() {
        let json = serde_json::to_string(&sample_bill())
            .unwrap()
            .to_lowercase();
        for proibido in [
            "\"name\"",
            "cpf",
            "cnpj",
            "address",
            "endereco",
            "installation",
            "instalacao",
            "phone_number",
            "holder",
        ] {
            assert!(
                !json.contains(proibido),
                "campo proibido no schema: {proibido}"
            );
        }
    }

    #[test]
    fn rejeita_campo_desconhecido_de_valor_invalido() {
        let mut v = serde_json::to_value(sample_bill()).unwrap();
        v["total_cents"] = serde_json::json!("cem reais");
        assert!(serde_json::from_value::<ExtractedBill>(v).is_err());
    }
}
