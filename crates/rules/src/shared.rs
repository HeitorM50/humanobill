//! Utilidades compartilhadas pelas regras de telecom e de energia.
//! Puras: só montam achados e fazem contas em centavos inteiros.

use humanobill_core::{
    BillItem, Cents, ExtractedBill, ItemCategory, SUM_TOLERANCE_CENTS, Severity,
};

use crate::text::{brl, has_word, tokens};
use crate::{Draft, Output};

/// Achado ligado a itens; trecho original e página vêm do primeiro item.
pub(crate) fn draft(
    severity: Severity,
    title: String,
    explanation: String,
    items: &[&BillItem],
    amount_cents: Cents,
) -> Draft {
    let main = items.first();
    Draft {
        severity,
        title,
        explanation,
        item_refs: items.iter().map(|i| i.id.clone()).collect(),
        original_text: main.map(|i| i.original_text.clone()),
        page: main.map(|i| i.page),
        amount_cents,
    }
}

pub(crate) fn sum(items: &[&BillItem]) -> Cents {
    items
        .iter()
        .map(|i| i.amount_cents)
        .fold(0, Cents::saturating_add)
}

/// `round(value * num / den)` com meio para cima, para `value >= 0`.
pub(crate) fn pct_round(value: Cents, num: Cents, den: Cents) -> Cents {
    value.saturating_mul(num).saturating_add(den / 2) / den
}

/// Itens de uma categoria, na ordem da fatura.
pub(crate) fn items_of(bill: &ExtractedBill, category: ItemCategory) -> Vec<&BillItem> {
    bill.items
        .iter()
        .filter(|i| i.category == category)
        .collect()
}

/// Encargos de atraso cujo texto contém uma das palavras.
pub(crate) fn late_fee_items<'a>(bill: &'a ExtractedBill, words: &[&str]) -> Vec<&'a BillItem> {
    bill.items
        .iter()
        .filter(|i| i.category == ItemCategory::EncargoAtraso)
        .filter(|i| {
            let mut t = tokens(&i.description);
            t.extend(tokens(&i.original_text));
            words.iter().any(|w| has_word(&t, w))
        })
        .collect()
}

/// "YYYY-MM" -> (ano, mês).
pub(crate) fn parse_month(s: &str) -> Option<(i32, u32)> {
    let (y, m) = s.split_once('-')?;
    if y.len() != 4 || m.len() != 2 {
        return None;
    }
    let year = y.parse().ok()?;
    let month = m.parse().ok().filter(|m| (1..=12).contains(m))?;
    Some((year, month))
}

/// "1 dia", "30 dias".
pub(crate) fn dias(n: i64) -> String {
    if n == 1 {
        "1 dia".to_owned()
    } else {
        format!("{n} dias")
    }
}

/// TEL-07 / ENE-08: soma dos itens diferente do total, além da tolerância do core.
pub(crate) fn sum_mismatch(bill: &ExtractedBill) -> Output {
    let mut out = Output::default();
    let items: Vec<&BillItem> = bill.items.iter().collect();
    let total_items = sum(&items);
    let diff = total_items
        .saturating_sub(bill.total_cents)
        .saturating_abs();
    if diff > SUM_TOLERANCE_CENTS {
        out.findings.push(draft(
            Severity::Suspicious,
            "Soma dos itens não bate com o total".into(),
            format!(
                "Os itens somam {}, mas o total da fatura é {}. Peça a conta detalhada à \
                 empresa.",
                brl(total_items),
                brl(bill.total_cents)
            ),
            &[],
            diff,
        ));
    }
    out
}
