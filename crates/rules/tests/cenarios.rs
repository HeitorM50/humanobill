//! Cenários Gherkin de RF07, RF08 e RF09 (docs/02-requisitos.md) e determinismo.
//!
//! Onde o Gherkin espera `indevido` de uma regra hoje "a validar", o teste
//! confere as duas coisas: com a tabela atual sai `suspeito` (cap central) e,
//! com a regra validada, sai o `indevido` pretendido.

use std::fs;
use std::path::PathBuf;

use chrono::NaiveDate;
use humanobill_core::{
    Answer, Answers, BillItem, ExtractedBill, Finding, ItemCategory, LateFees, RuleStatus, Severity,
};
use humanobill_rules::{RULES, Rule, evaluate, evaluate_with, recoverable_cents};

fn fixture(name: &str) -> ExtractedBill {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/extracted")
        .join(format!("{name}.json"));
    serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap()
}

fn with_status(id: &str, status: RuleStatus) -> Vec<Rule> {
    RULES
        .iter()
        .map(|r| {
            if r.id == id {
                r.with_status(status)
            } else {
                *r
            }
        })
        .collect()
}

fn of<'a>(findings: &'a [Finding], rule_id: &str) -> Vec<&'a Finding> {
    findings.iter().filter(|f| f.rule_id == rule_id).collect()
}

fn fatura_com_multa_de_5_por_cento() -> ExtractedBill {
    let mut bill = fixture("telecom-3");
    bill.items.push(BillItem {
        id: "i3".into(),
        description: "Multa por atraso".into(),
        amount_cents: 500,
        category: ItemCategory::EncargoAtraso,
        period: None,
        original_text: "MULTA ATRASO 5% 5,00".into(),
        page: 1,
        plain_explanation: "Multa pelo atraso da fatura anterior.".into(),
    });
    bill.total_cents += 500;
    bill.late_fees = Some(LateFees {
        base_cents: 10_000,
        fine_cents: 500,
        interest_cents: 0,
        days_late: 0,
    });
    bill
}

// ---------- RF07 — Detectar cobranças indevidas ----------

#[test]
fn rf07_servico_adicional_nao_contratado_fica_suspeito() {
    let bill = fixture("telecom-1");
    let eval = evaluate(&bill, &Answers::new());
    let clube: Vec<_> = of(&eval.findings, "TEL-01")
        .into_iter()
        .filter(|f| f.item_refs == ["i2"])
        .collect();
    assert_eq!(clube.len(), 1);
    assert_eq!(clube[0].severity, Severity::Suspicious);
    assert_eq!(clube[0].amount_cents, 1_290);
}

#[test]
fn rf07_multa_acima_do_limite_vale_3_reais() {
    let bill = fatura_com_multa_de_5_por_cento();

    // TEL-03 validada (RGC art. 78, § 1º): "indevido" de R$ 3,00, como no Gherkin.
    let eval = evaluate(&bill, &Answers::new());
    let tel03 = of(&eval.findings, "TEL-03");
    assert_eq!(tel03.len(), 1);
    assert_eq!(tel03[0].severity, Severity::Unlawful);
    assert_eq!(tel03[0].amount_cents, 300);
    assert_eq!(tel03[0].item_refs, ["i3"]);

    // Se a regra voltasse a "a validar", no máximo suspeito.
    let pendente = evaluate_with(
        &with_status("TEL-03", RuleStatus::Pending),
        &bill,
        &Answers::new(),
    );
    assert_eq!(
        of(&pendente.findings, "TEL-03")[0].severity,
        Severity::Suspicious
    );
}

#[test]
fn rf07_regra_a_validar_nunca_acusa() {
    let mut answers = Answers::new();
    answers.insert("TEL-01:i2".into(), Answer::No);
    answers.insert("TEL-01:i3".into(), Answer::No);
    answers.insert(
        "TEL-06:cancelamento".into(),
        Answer::Date(NaiveDate::from_ymd_opt(2026, 8, 1).unwrap()),
    );
    for bill in [
        fixture("telecom-1"),
        fixture("telecom-2"),
        fatura_com_multa_de_5_por_cento(),
    ] {
        for f in evaluate(&bill, &answers).findings {
            let st = humanobill_rules::status(&f.rule_id).unwrap();
            if st == RuleStatus::Pending {
                assert_ne!(f.severity, Severity::Unlawful, "{} acusou", f.rule_id);
            }
        }
    }
}

#[test]
fn rf07_conta_sem_problemas_telecom_3() {
    let eval = evaluate(&fixture("telecom-3"), &Answers::new());
    assert!(
        eval.findings.iter().all(|f| f.severity == Severity::Ok),
        "{:?}",
        eval.findings
    );
    assert!(eval.findings.is_empty());
    assert_eq!(recoverable_cents(&eval.findings, None), 0);
}

// ---------- RF08 — Justificar achado ----------

#[test]
fn rf08_achado_tel01_tem_trecho_pagina_e_base_legal() {
    let eval = evaluate(&fixture("telecom-1"), &Answers::new());
    let f = of(&eval.findings, "TEL-01")
        .into_iter()
        .find(|f| f.item_refs == ["i2"])
        .unwrap();
    assert!(!f.explanation.is_empty());
    assert_eq!(f.original_text.as_deref(), Some("CLUBE VZ MUS 12,90"));
    assert_eq!(f.page, Some(1));
    let cdc = f
        .legal_basis
        .iter()
        .find(|l| l.label.starts_with("CDC art. 39, III"))
        .expect("base legal CDC art. 39, III");
    assert!(
        f.legal_basis
            .iter()
            .any(|l| l.label.starts_with("RGC Anatel art. 48"))
    );
    assert!(cdc.url.starts_with("https://"));
}

// ---------- RF09 — Responder perguntas de confirmação ----------

#[test]
fn rf09_pergunta_do_clube_tem_texto_do_gherkin() {
    let eval = evaluate(&fixture("telecom-1"), &Answers::new());
    let q = eval.questions.iter().find(|q| q.id == "TEL-01:i2").unwrap();
    assert_eq!(q.text, "Você contratou Clube Vivaz Música?");
}

#[test]
fn rf09_usuario_nega_a_contratacao() {
    let bill = fixture("telecom-1");
    let antes = evaluate(&bill, &Answers::new());
    let mut answers = Answers::new();
    answers.insert("TEL-01:i2".into(), Answer::No);

    // TEL-01 validada: passa a indevido e o recuperável aumenta R$ 12,90.
    let depois = evaluate(&bill, &answers);
    let f = of(&depois.findings, "TEL-01")
        .into_iter()
        .find(|f| f.item_refs == ["i2"])
        .unwrap();
    assert_eq!(f.severity, Severity::Unlawful);
    assert_eq!(
        recoverable_cents(&depois.findings, None) - recoverable_cents(&antes.findings, None),
        1_290
    );
    assert_eq!(recoverable_cents(&antes.findings, None), 0);
}

#[test]
fn rf09_usuario_confirma_a_contratacao_vira_ok() {
    let mut answers = Answers::new();
    answers.insert("TEL-01:i2".into(), Answer::Yes);
    let eval = evaluate(&fixture("telecom-1"), &answers);
    let f = of(&eval.findings, "TEL-01")
        .into_iter()
        .find(|f| f.item_refs == ["i2"])
        .unwrap();
    assert_eq!(f.severity, Severity::Ok);
    assert_eq!(recoverable_cents(&eval.findings, None), 0);
}

#[test]
fn rf09_pergunta_sem_resposta_continua_suspeito() {
    let eval = evaluate(&fixture("telecom-1"), &Answers::new());
    for f in of(&eval.findings, "TEL-01") {
        assert_eq!(f.severity, Severity::Suspicious);
    }
}

// ---------- RNF11 — determinismo ----------

#[test]
fn determinismo_mesma_entrada_100_vezes() {
    let mut answers = Answers::new();
    answers.insert("TEL-01:i3".into(), Answer::Yes);
    answers.insert("TEL-05:plano".into(), Answer::Amount(11_990));
    for name in ["telecom-1", "telecom-2", "telecom-3"] {
        let bill = fixture(name);
        let primeira = evaluate(&bill, &answers);
        for _ in 0..100 {
            assert_eq!(evaluate(&bill, &answers), primeira, "{name}");
        }
    }
}
