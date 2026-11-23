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
    /// Extrato de conta corrente (#54): só o que o banco cobrou no período
    /// (tarifas, pacote, seguros). As movimentações da pessoa não são extraídas.
    BankStatement,
    /// Fatura do cartão de crédito (#55): tarifas, serviços e encargos do emissor;
    /// as compras entram somadas num item só, sem nome de estabelecimento.
    CardBill,
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
    /// Tarifa de banco ou de cartão: pacote de serviços, saque, extrato, anuidade (#54, #55).
    Tarifa,
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
    /// Onde o item aparece na imagem da fatura (marcação visual, #51).
    /// Opcional: a IA omite quando não tem certeza. As regras não usam.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub bbox: Option<BBox>,
}

/// Caixa de um trecho da fatura, normalizada de 0 a 1 em relação à largura e
/// à altura da página, com origem no canto superior esquerdo.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct BBox {
    /// Página (começa em 1).
    pub page: u32,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl BBox {
    /// Folga para arredondamento da IA na borda da página.
    const EDGE_TOLERANCE: f32 = 0.01;

    /// Página >= 1, tudo entre 0 e 1, largura e altura positivas e a caixa
    /// dentro da página (com uma folga pequena de arredondamento).
    pub fn is_valid(&self) -> bool {
        let unit = |v: f32| v.is_finite() && (0.0..=1.0).contains(&v);
        self.page >= 1
            && [self.x, self.y, self.w, self.h].into_iter().all(unit)
            && self.w > 0.0
            && self.h > 0.0
            && self.x + self.w <= 1.0 + Self::EDGE_TOLERANCE
            && self.y + self.h <= 1.0 + Self::EDGE_TOLERANCE
    }
}

/// Descarta (vira `None`) todo bbox inválido. Um bbox ruim nunca derruba a
/// extração: o item continua, só sem a marcação. Devolve quantos caíram.
pub fn drop_invalid_bboxes(bill: &mut ExtractedBill) -> usize {
    let mut dropped = 0;
    for item in &mut bill.items {
        if item.bbox.is_some_and(|b| !b.is_valid()) {
            item.bbox = None;
            dropped += 1;
        }
    }
    dropped
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
    /// Vencimento. No extrato bancário, que não vence, é o último dia do período.
    #[ts(type = "string")]
    pub due_date: NaiveDate,
    #[serde(default)]
    #[ts(type = "string | null")]
    pub issue_date: Option<NaiveDate>,
    /// Total a pagar. No extrato bancário, a soma do que o banco cobrou no período.
    #[ts(type = "number")]
    pub total_cents: Cents,
    /// Plano (telecom), pacote de serviços (extrato) ou nome do cartão.
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

/// Um passo do roteiro de ligação ao SAC (RF17): o que dizer, em ordem.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct DisputeCallStep {
    /// Nome curto do passo ("Apresente-se", "Peça o protocolo"...).
    pub title: String,
    /// O que falar na ligação, em primeira pessoa.
    pub say: String,
}

/// Resposta de `POST /analyses/:id/dispute`: a contestação (RF11/RF12)
/// junto do roteiro de ligação (RF17). Os campos da contestação continuam no
/// mesmo nível do JSON; só `call_script` é novo.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct DisputeResult {
    #[serde(flatten)]
    #[ts(flatten)]
    pub dispute: Dispute,
    /// Roteiro numerado para quem prefere ligar.
    pub call_script: Vec<DisputeCallStep>,
}

/// Corpo de `POST /reminders` (RF18). O consentimento do lembrete é separado
/// do consentimento do envio da fatura.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ReminderRequest {
    pub analysis_id: String,
    pub email: String,
    pub consent: bool,
}

// Debug manual: o e-mail nunca pode aparecer em log.
impl std::fmt::Debug for ReminderRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ReminderRequest")
            .field("analysis_id", &self.analysis_id)
            .field("email", &"<oculto>")
            .field("consent", &self.consent)
            .finish()
    }
}

/// Resposta de `POST /reminders`: quando o aviso será enviado (RFC 3339).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ReminderCreated {
    pub due_at: String,
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

/// Tamanho mínimo de um grupo no painel coletivo (k-anonimato): grupos menores não aparecem.
pub const STATS_MIN_GROUP: u32 = 5;

/// Total de análises enviadas de um tipo de fatura. `total` é `null` com poucos casos.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct KindStats {
    pub kind: BillKind,
    /// `None` quando o grupo tem menos de `STATS_MIN_GROUP` análises ("poucos casos ainda").
    pub total: Option<u32>,
    /// Algum achado deste tipo ficou de fora por ter poucos casos.
    pub has_hidden_findings: bool,
}

/// Quantas análises tiveram um achado de uma regra. Sem empresa, sem valor, sem ids.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct RuleStats {
    pub rule_id: String,
    /// Título curto da regra, em português claro.
    pub title: String,
    pub kind: BillKind,
    pub severity: Severity,
    /// Análises com esse achado (sempre >= `STATS_MIN_GROUP`).
    pub count: u32,
    /// Parte das análises do mesmo tipo que teve o achado (0 a 100).
    pub percent: u8,
}

/// Painel coletivo anonimizado (`GET /stats`): só faturas enviadas, não expiradas.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Stats {
    /// Tamanho mínimo de grupo que aparece (k-anonimato).
    pub min_group: u32,
    pub kinds: Vec<KindStats>,
    /// Do mais frequente para o menos frequente.
    pub rules: Vec<RuleStats>,
}

/// Um item na comparação entre duas faturas (RF16).
/// Só descrição, categoria e valores: nada de trecho original nem dado pessoal.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ComparedItem {
    /// Descrição como aparece na fatura mais recente (na antiga, se o item sumiu).
    pub description: String,
    pub category: ItemCategory,
    /// Valor na fatura anterior. `null` quando o item é novo.
    #[ts(type = "number | null")]
    pub before_cents: Option<Cents>,
    /// Valor na fatura mais recente. `null` quando o item sumiu.
    #[ts(type = "number | null")]
    pub after_cents: Option<Cents>,
    /// `after - before` (um item novo conta como subida; um que sumiu, como queda).
    #[ts(type = "number")]
    pub delta_cents: Cents,
    /// Variação em %, com 1 casa. `null` quando não há valor anterior para comparar.
    pub delta_pct: Option<f64>,
    /// Virou cobrança nova (ex.: seguro ou assinatura que não existia antes).
    pub new_charge: bool,
}

/// Resultado de comparar duas faturas do mesmo tipo e da mesma empresa (RF16).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Comparison {
    pub kind: BillKind,
    pub provider_name: String,
    /// Mês de referência da fatura mais antiga ("YYYY-MM").
    pub before_month: String,
    /// Mês de referência da fatura mais recente ("YYYY-MM").
    pub after_month: String,
    #[ts(type = "number")]
    pub before_total_cents: Cents,
    #[ts(type = "number")]
    pub after_total_cents: Cents,
    #[ts(type = "number")]
    pub total_delta_cents: Cents,
    /// Variação do total em %, com 1 casa. `null` se o total anterior era zero.
    pub total_delta_pct: Option<f64>,
    /// Itens novos este mês.
    pub added: Vec<ComparedItem>,
    /// Itens que não aparecem mais.
    pub removed: Vec<ComparedItem>,
    /// Itens nos dois meses com valor diferente.
    pub changed: Vec<ComparedItem>,
    /// Itens nos dois meses com o mesmo valor.
    pub unchanged: Vec<ComparedItem>,
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
                    bbox: None,
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
                    bbox: None,
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
                    bbox: None,
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
    fn tipos_novos_serializam_em_snake_case() {
        assert_eq!(
            serde_json::to_string(&BillKind::BankStatement).unwrap(),
            "\"bank_statement\""
        );
        assert_eq!(
            serde_json::to_string(&BillKind::CardBill).unwrap(),
            "\"card_bill\""
        );
        assert_eq!(
            serde_json::to_string(&ItemCategory::Tarifa).unwrap(),
            "\"tarifa\""
        );
        let k: BillKind = serde_json::from_str("\"bank_statement\"").unwrap();
        assert_eq!(k, BillKind::BankStatement);
        assert!(serde_json::from_str::<BillKind>("\"bank\"").is_err());
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
            // Banco e cartão (#54, #55): nada de agência, conta ou número do cartão.
            "agencia",
            "account",
            "card_number",
            "last_digits",
            "ultimos_digitos",
        ] {
            assert!(
                !json.contains(proibido),
                "campo proibido no schema: {proibido}"
            );
        }
    }

    fn caixa(x: f32, y: f32, w: f32, h: f32) -> BBox {
        BBox {
            page: 1,
            x,
            y,
            w,
            h,
        }
    }

    #[test]
    fn bbox_valido_e_bordas() {
        assert!(caixa(0.07, 0.25, 0.85, 0.02).is_valid());
        assert!(caixa(0.0, 0.0, 1.0, 1.0).is_valid());
        // Folga de arredondamento na borda.
        assert!(caixa(0.5, 0.5, 0.505, 0.5).is_valid());
    }

    #[test]
    fn bbox_invalido_e_reconhecido() {
        for b in [
            caixa(-0.1, 0.2, 0.3, 0.1),
            caixa(0.1, 1.2, 0.3, 0.1),
            caixa(0.1, 0.2, 0.0, 0.1),
            caixa(0.1, 0.2, 0.3, -0.1),
            caixa(0.8, 0.2, 0.5, 0.1),
            caixa(0.1, 0.95, 0.3, 0.2),
            caixa(f32::NAN, 0.2, 0.3, 0.1),
            caixa(0.1, 0.2, f32::INFINITY, 0.1),
            BBox {
                page: 0,
                ..caixa(0.1, 0.2, 0.3, 0.1)
            },
        ] {
            assert!(!b.is_valid(), "{b:?} deveria ser inválido");
        }
    }

    #[test]
    fn bbox_invalido_e_descartado_sem_mexer_no_resto() {
        let mut bill = sample_bill();
        bill.items[0].bbox = Some(caixa(0.07, 0.25, 0.85, 0.02));
        bill.items[1].bbox = Some(caixa(0.07, 0.25, 0.0, 0.02));
        bill.items[2].bbox = Some(caixa(0.07, 1.5, 0.85, 0.02));
        assert_eq!(drop_invalid_bboxes(&mut bill), 2);
        assert!(bill.items[0].bbox.is_some());
        assert_eq!(bill.items[1].bbox, None);
        assert_eq!(bill.items[2].bbox, None);
        assert_eq!(bill.items[1].amount_cents, 1_290);
    }

    #[test]
    fn bbox_ausente_nao_aparece_no_json_e_null_e_aceito() {
        let json = serde_json::to_string(&sample_bill()).unwrap();
        assert!(!json.contains("bbox"));
        let mut v = serde_json::to_value(sample_bill()).unwrap();
        v["items"][0]["bbox"] = serde_json::Value::Null;
        v["items"][1]["bbox"] =
            serde_json::json!({"page": 1, "x": 0.1, "y": 0.2, "w": 0.3, "h": 0.04});
        let bill: ExtractedBill = serde_json::from_value(v).unwrap();
        assert_eq!(bill.items[0].bbox, None);
        assert_eq!(bill.items[1].bbox, Some(caixa(0.1, 0.2, 0.3, 0.04)));
    }

    #[test]
    fn rejeita_campo_desconhecido_de_valor_invalido() {
        let mut v = serde_json::to_value(sample_bill()).unwrap();
        v["total_cents"] = serde_json::json!("cem reais");
        assert!(serde_json::from_value::<ExtractedBill>(v).is_err());
    }
}
