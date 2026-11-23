//! Golden tests: `fixtures/extracted/<bill>.json` + respostas -> achados esperados
//! em `fixtures/findings/<bill>.json`. Compara (rule_id, severidade, item_refs).

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use humanobill_core::{Answer, Answers, ExtractedBill, Severity};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Golden {
    bill: String,
    #[serde(default)]
    answers: BTreeMap<String, Answer>,
    /// Atalho para a resposta "TEL-05:plano".
    #[serde(default)]
    plan_price_cents: Option<i64>,
    expected: Vec<Expected>,
}

#[derive(Debug, Deserialize, PartialEq, Eq, PartialOrd, Ord, Clone)]
struct Expected {
    rule_id: String,
    severity: Severity,
    item_refs: Vec<String>,
}

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
}

fn normalize(mut v: Vec<Expected>) -> Vec<Expected> {
    for e in &mut v {
        e.item_refs.sort();
    }
    v.sort();
    v
}

/// Roda todos os goldens cujo arquivo começa com `prefix` (ex.: "telecom-").
fn run_goldens(prefix: &str, min: usize) {
    let dir = fixtures().join("findings");
    let mut paths: Vec<_> = fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| {
            p.extension().and_then(|e| e.to_str()) == Some("json")
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.starts_with(prefix))
        })
        .collect();
    paths.sort();
    assert!(
        paths.len() >= min,
        "esperava ao menos {min} goldens {prefix}*"
    );

    for path in paths {
        let golden: Golden = serde_json::from_str(&fs::read_to_string(&path).unwrap())
            .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let bill_path = fixtures()
            .join("extracted")
            .join(format!("{}.json", golden.bill));
        let bill: ExtractedBill = serde_json::from_str(&fs::read_to_string(&bill_path).unwrap())
            .unwrap_or_else(|e| panic!("{}: {e}", bill_path.display()));

        let mut answers: Answers = golden.answers.clone();
        if let Some(p) = golden.plan_price_cents {
            answers.insert("TEL-05:plano".into(), Answer::Amount(p));
        }

        let eval = humanobill_rules::evaluate(&bill, &answers);
        let got = normalize(
            eval.findings
                .iter()
                .map(|f| Expected {
                    rule_id: f.rule_id.clone(),
                    severity: f.severity,
                    item_refs: f.item_refs.clone(),
                })
                .collect(),
        );
        let want = normalize(golden.expected.clone());
        assert_eq!(got, want, "golden {}", path.display());
    }
}

#[test]
fn golden_telecom() {
    run_goldens("telecom-", 3);
}

#[test]
fn golden_energia() {
    run_goldens("energia-", 3);
}

#[test]
fn golden_extrato() {
    run_goldens("extrato-", 2);
}

#[test]
fn golden_cartao() {
    run_goldens("cartao-", 2);
}
