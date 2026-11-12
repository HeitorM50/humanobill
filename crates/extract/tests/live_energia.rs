//! Medição live de acurácia (gasta quota do Gemini). Rode com:
//! `set -a && . ./.env && set +a && EXTRACT_CACHE_DIR=target/extract-cache \
//!  cargo test -p humanobill-extract --test live_energia -- --ignored --nocapture`
//! Imprime só nomes de campo, ok/erro e valores numéricos (RNF13).

use humanobill_core::ExtractedBill;
use humanobill_extract::{BillExtractor, ExtractError, GeminiConfig, GeminiExtractor, InputMime};
use std::collections::BTreeMap;

fn root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn norm(s: &str) -> String {
    s.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_uppercase()
}

type Tally = BTreeMap<String, (u32, u32)>;

fn rec(t: &mut Tally, field: &str, ok: bool, id: &str, errs: &mut Vec<String>) {
    let e = t.entry(field.to_string()).or_default();
    e.1 += 1;
    if ok {
        e.0 += 1;
    } else {
        errs.push(format!("{id}: {field}"));
    }
}

fn compare(
    id: &str,
    got: &ExtractedBill,
    exp: &ExtractedBill,
    t: &mut Tally,
    errs: &mut Vec<String>,
) {
    rec(t, "kind", got.kind == exp.kind, id, errs);
    rec(
        t,
        "provider_name",
        got.provider_name == exp.provider_name,
        id,
        errs,
    );
    rec(
        t,
        "reference_month",
        got.reference_month == exp.reference_month,
        id,
        errs,
    );
    rec(t, "due_date", got.due_date == exp.due_date, id, errs);
    rec(
        t,
        "total_cents",
        got.total_cents == exp.total_cents,
        id,
        errs,
    );
    rec(t, "items.len", got.items.len() == exp.items.len(), id, errs);
    for (i, e) in exp.items.iter().enumerate() {
        let g = got.items.get(i);
        rec(
            t,
            "item.amount_cents",
            g.is_some_and(|g| g.amount_cents == e.amount_cents),
            id,
            errs,
        );
        rec(
            t,
            "item.category",
            g.is_some_and(|g| g.category == e.category),
            id,
            errs,
        );
        rec(
            t,
            "item.original_text",
            g.is_some_and(|g| norm(&g.original_text) == norm(&e.original_text)),
            id,
            errs,
        );
    }
    if let Some(ee) = &exp.energy {
        let ge = got.energy.as_ref();
        rec(
            t,
            "energy.consumption_kwh",
            ge.is_some_and(|g| g.consumption_kwh == ee.consumption_kwh),
            id,
            errs,
        );
        rec(
            t,
            "energy.reading_type",
            ge.is_some_and(|g| g.reading_type == ee.reading_type),
            id,
            errs,
        );
        rec(
            t,
            "energy.reading_prev_date",
            ge.is_some_and(|g| g.reading_prev_date == ee.reading_prev_date),
            id,
            errs,
        );
        rec(
            t,
            "energy.reading_curr_date",
            ge.is_some_and(|g| g.reading_curr_date == ee.reading_curr_date),
            id,
            errs,
        );
        rec(
            t,
            "energy.tariff_flag",
            ge.is_some_and(|g| g.tariff_flag == ee.tariff_flag),
            id,
            errs,
        );
        rec(
            t,
            "energy.history.len",
            ge.is_some_and(|g| g.history.len() == 12),
            id,
            errs,
        );
        let kwh_ok = ge.is_some_and(|g| {
            g.history.len() == ee.history.len()
                && g.history
                    .iter()
                    .zip(&ee.history)
                    .all(|(a, b)| a.kwh == b.kwh)
        });
        rec(t, "energy.history.kwh", kwh_ok, id, errs);
        let mon_ok = ge.is_some_and(|g| {
            g.history.len() == ee.history.len()
                && g.history
                    .iter()
                    .zip(&ee.history)
                    .all(|(a, b)| a.month == b.month)
        });
        rec(t, "energy.history.month", mon_ok, id, errs);
    } else {
        rec(t, "energy.ausente", got.energy.is_none(), id, errs);
    }
}

#[tokio::test]
#[ignore]
async fn live_acuracia_fixtures() {
    let key = std::env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY");
    let model =
        std::env::var("GEMINI_MODEL").unwrap_or_else(|_| GeminiConfig::DEFAULT_MODEL.into());
    let ex = GeminiExtractor::new(GeminiConfig {
        api_key: key,
        model,
        fallback_model: Some(GeminiConfig::DEFAULT_FALLBACK_MODEL.into()),
        base_url: GeminiConfig::DEFAULT_BASE_URL.into(),
    });
    let mut tally = Tally::new();
    let mut errs = Vec::new();
    let max_calls: u32 = std::env::var("EXTRACT_MAX_CALLS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(4);
    for id in ["telecom-1", "energia-1", "energia-2", "energia-3"] {
        if ex.http_calls() >= max_calls {
            println!("orçamento de chamadas atingido ({max_calls}); parando antes de {id}");
            errs.push(format!("{id}: nao medido"));
            continue;
        }
        let pdf = std::fs::read(root().join(format!("fixtures/bills/{id}.pdf"))).unwrap();
        let exp: ExtractedBill = serde_json::from_slice(
            &std::fs::read(root().join(format!("fixtures/extracted/{id}.json"))).unwrap(),
        )
        .unwrap();
        match ex.extract(&pdf, InputMime::Pdf).await {
            Ok(got) => compare(id, &got, &exp, &mut tally, &mut errs),
            Err(ExtractError::RateLimited) => {
                println!(
                    "QUOTA (429) em {id}: parando. chamadas HTTP: {}",
                    ex.http_calls()
                );
                panic!("quota esgotada");
            }
            Err(e) => {
                println!("{id}: extração falhou: {e}");
                errs.push(format!("{id}: extract"));
            }
        }
    }
    println!("\ncampo | acertos/total");
    for (f, (ok, n)) in &tally {
        println!("{f} | {ok}/{n}");
    }
    println!("erros: {errs:?}");
    println!("chamadas HTTP ao Gemini: {}", ex.http_calls());
    assert!(errs.is_empty(), "divergências: {errs:?}");
}
