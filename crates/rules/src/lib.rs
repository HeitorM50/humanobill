//! Motor de regras: `ExtractedBill` + respostas do usuário -> achados.
//! Puro e determinístico: sem I/O, sem async, sem aleatoriedade (RNF11).
//! Catálogo de regras em docs/03-regras-negocio.md.
//!
//! S1: só a estrutura. As regras TEL entram na issue #15 e as ENE na #26.

use humanobill_core::{Answers, Cents, ExtractedBill, Finding, Question, Severity};

/// Versão do conjunto de regras, gravada em cada análise.
pub const RULES_VERSION: &str = "0.1.0";

/// Contexto informado pelo usuário que não está na fatura.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Context {
    /// Valor do plano contratado, se informado (TEL-05).
    pub plan_price_cents: Option<Cents>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Evaluation {
    pub findings: Vec<Finding>,
    pub questions: Vec<Question>,
}

/// Avalia todas as regras sobre a fatura.
pub fn evaluate(_bill: &ExtractedBill, _answers: &Answers, _ctx: &Context) -> Evaluation {
    Evaluation::default()
}

/// Valor recuperável: soma dos achados `indevido`; em dobro se já pago (CDC art. 42).
pub fn recoverable_cents(findings: &[Finding], paid: Option<bool>) -> Cents {
    let base: Cents = findings
        .iter()
        .filter(|f| f.severity == Severity::Unlawful)
        .map(|f| f.amount_cents)
        .sum();
    if paid == Some(true) { base * 2 } else { base }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn finding(sev: Severity, cents: Cents) -> Finding {
        Finding {
            rule_id: "TEL-XX".into(),
            severity: sev,
            title: String::new(),
            explanation: String::new(),
            item_refs: vec![],
            original_text: None,
            page: None,
            amount_cents: cents,
            legal_basis: vec![],
        }
    }

    #[test]
    fn recuperavel_soma_so_indevidos() {
        let f = [
            finding(Severity::Unlawful, 1290),
            finding(Severity::Suspicious, 990),
            finding(Severity::Unlawful, 990),
        ];
        assert_eq!(recoverable_cents(&f, None), 2280);
        assert_eq!(recoverable_cents(&f, Some(false)), 2280);
    }

    #[test]
    fn recuperavel_em_dobro_se_pago() {
        let f = [finding(Severity::Unlawful, 2280)];
        assert_eq!(recoverable_cents(&f, Some(true)), 4560);
    }
}
