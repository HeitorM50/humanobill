//! Validação de consistência de uma fatura extraída (depois da IA).

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{BillKind, ExtractedBill, SUM_TOLERANCE_CENTS};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "code", rename_all = "snake_case")]
#[ts(export)]
pub enum ValidationIssue {
    /// A soma dos itens difere do total além da tolerância.
    SumMismatch {
        #[ts(type = "number")]
        items_cents: i64,
        #[ts(type = "number")]
        total_cents: i64,
    },
    NoItems,
    NegativeTotal,
    BadReferenceMonth,
    DuplicateItemId {
        id: String,
    },
    /// Conta de energia sem o bloco `energy`.
    MissingEnergyData,
    /// Leitura atual anterior à leitura anterior.
    ReadingDatesInverted,
    ConfidenceOutOfRange,
}

/// Lista todos os problemas de consistência. Vazio = fatura consistente.
pub fn validate_bill(bill: &ExtractedBill) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();

    if bill.items.is_empty() {
        issues.push(ValidationIssue::NoItems);
    }
    if bill.total_cents < 0 {
        issues.push(ValidationIssue::NegativeTotal);
    }
    if !is_year_month(&bill.reference_month) {
        issues.push(ValidationIssue::BadReferenceMonth);
    }
    if !(0.0..=1.0).contains(&bill.extraction_confidence) {
        issues.push(ValidationIssue::ConfidenceOutOfRange);
    }

    // Valores absurdos vindos da IA não podem derrubar a validação: se a soma
    // ou a diferença estourar i64, a fatura é inconsistente (SumMismatch).
    let items_sum = bill
        .items
        .iter()
        .try_fold(0i64, |acc, i| acc.checked_add(i.amount_cents));
    let items_cents = items_sum.unwrap_or_else(|| {
        bill.items
            .iter()
            .fold(0i64, |acc, i| acc.saturating_add(i.amount_cents))
    });
    let sum_diverges = items_sum
        .and_then(|s| s.checked_sub(bill.total_cents))
        .is_none_or(|d| d.unsigned_abs() > SUM_TOLERANCE_CENTS.unsigned_abs());
    if !bill.items.is_empty() && sum_diverges {
        issues.push(ValidationIssue::SumMismatch {
            items_cents,
            total_cents: bill.total_cents,
        });
    }

    let mut seen = std::collections::BTreeSet::new();
    for item in &bill.items {
        if !seen.insert(item.id.as_str()) {
            issues.push(ValidationIssue::DuplicateItemId {
                id: item.id.clone(),
            });
        }
    }

    match (&bill.kind, &bill.energy) {
        (BillKind::Energy, None) => issues.push(ValidationIssue::MissingEnergyData),
        (_, Some(e)) if e.reading_curr_date < e.reading_prev_date => {
            issues.push(ValidationIssue::ReadingDatesInverted)
        }
        _ => {}
    }

    issues
}

fn is_year_month(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 7
        && b[4] == b'-'
        && b[..4].iter().all(u8::is_ascii_digit)
        && b[5..].iter().all(u8::is_ascii_digit)
        && matches!(s[5..].parse::<u8>(), Ok(1..=12))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::sample_bill;

    #[test]
    fn fatura_consistente_nao_tem_problemas() {
        assert!(validate_bill(&sample_bill()).is_empty());
    }

    #[test]
    fn soma_dentro_da_tolerancia_passa() {
        let mut b = sample_bill();
        b.total_cents += SUM_TOLERANCE_CENTS;
        assert!(validate_bill(&b).is_empty());
    }

    #[test]
    fn soma_divergente_e_apontada() {
        let mut b = sample_bill();
        b.total_cents += 100;
        assert!(matches!(
            validate_bill(&b)[..],
            [ValidationIssue::SumMismatch { .. }]
        ));
    }

    #[test]
    fn mes_invalido_e_apontado() {
        let mut b = sample_bill();
        b.reference_month = "2026-13".into();
        assert_eq!(validate_bill(&b), vec![ValidationIssue::BadReferenceMonth]);
    }

    #[test]
    fn id_duplicado_e_apontado() {
        let mut b = sample_bill();
        b.items[1].id = "i1".into();
        assert!(validate_bill(&b).contains(&ValidationIssue::DuplicateItemId { id: "i1".into() }));
    }

    #[test]
    fn energia_sem_bloco_energy_e_apontada() {
        let mut b = sample_bill();
        b.kind = BillKind::Energy;
        assert!(validate_bill(&b).contains(&ValidationIssue::MissingEnergyData));
    }
}
