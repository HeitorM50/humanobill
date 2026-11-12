//! Motor de regras: `ExtractedBill` + respostas do usuário -> achados.
//! Puro e determinístico: sem I/O, sem async, sem relógio, sem aleatoriedade (RNF11).
//! Catálogo de regras em docs/03-regras-negocio.md.
//!
//! Fluxo de `evaluate`:
//! 1. cada regra da tabela [`RULES`] que vale para o tipo de fatura devolve
//!    achados com a severidade **pretendida** e as perguntas que precisa;
//! 2. o motor completa `rule_id` e `legal_basis` a partir da tabela;
//! 3. o motor aplica o **cap central**: regra `Pending` ("a validar") nunca
//!    emite `indevido`, só `suspeito` (docs/03, princípio 5);
//! 4. achados e perguntas saem em ordem determinística.
//!
//! Quando a base legal de uma regra for conferida, basta trocar o `status`
//! dela em [`RULES`]: a severidade pretendida passa a aparecer sem mexer na regra.

pub mod dispute;
pub mod legal;
mod telecom;
mod text;

use humanobill_core::{
    Answers, BillKind, Cents, ExtractedBill, Finding, Question, RuleStatus, Severity,
};

use legal::{CDC_6_III, CDC_30, CDC_39_III, CDC_42, Legal, RGC_48, RGC_55_I, RGC_78_P1, RGC_83_84};

/// Versão do conjunto de regras, gravada em cada análise.
pub const RULES_VERSION: &str = "0.3.0";

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Evaluation {
    pub findings: Vec<Finding>,
    pub questions: Vec<Question>,
}

/// Achado como a regra o produz: severidade pretendida, sem `rule_id`,
/// sem base legal e sem o cap de status (o motor completa).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Draft {
    pub severity: Severity,
    pub title: String,
    pub explanation: String,
    pub item_refs: Vec<String>,
    pub original_text: Option<String>,
    pub page: Option<u32>,
    pub amount_cents: Cents,
}

/// Saída de uma regra.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct Output {
    pub findings: Vec<Draft>,
    pub questions: Vec<Question>,
}

/// Entrada da tabela de regras (contrato `Rule` do docs/03).
#[derive(Debug, Clone, Copy)]
pub struct Rule {
    pub id: &'static str,
    /// Tipo de fatura em que a regra roda.
    pub kind: BillKind,
    /// `Pending` = "a validar": o motor rebaixa `indevido` para `suspeito`.
    pub status: RuleStatus,
    /// Base legal citada em todo achado da regra (vazia = só consistência).
    pub legal: &'static [Legal],
    eval: fn(&ExtractedBill, &Answers) -> Output,
}

impl Rule {
    /// Cópia da regra com outro status (para simular a validação em testes).
    pub fn with_status(self, status: RuleStatus) -> Rule {
        Rule { status, ..self }
    }
}

/// Tabela única de regras: id, status e base legal (docs/03).
/// Mudou o status ou a base legal de uma regra? Mude aqui e em `legal.rs`.
pub static RULES: &[Rule] = &[
    Rule {
        id: "TEL-01",
        kind: BillKind::Telecom,
        status: RuleStatus::Validated,
        legal: &[CDC_39_III, RGC_48],
        eval: telecom::tel01,
    },
    Rule {
        id: "TEL-02",
        kind: BillKind::Telecom,
        status: RuleStatus::Validated,
        legal: &[CDC_42],
        eval: telecom::tel02,
    },
    Rule {
        id: "TEL-03",
        kind: BillKind::Telecom,
        status: RuleStatus::Validated,
        legal: &[RGC_78_P1],
        eval: telecom::tel03,
    },
    Rule {
        id: "TEL-04",
        kind: BillKind::Telecom,
        status: RuleStatus::Validated,
        legal: &[RGC_78_P1],
        eval: telecom::tel04,
    },
    Rule {
        id: "TEL-05",
        kind: BillKind::Telecom,
        status: RuleStatus::Validated,
        legal: &[CDC_30],
        eval: telecom::tel05,
    },
    Rule {
        id: "TEL-06",
        kind: BillKind::Telecom,
        status: RuleStatus::Validated,
        legal: &[RGC_83_84],
        eval: telecom::tel06,
    },
    Rule {
        id: "TEL-07",
        kind: BillKind::Telecom,
        status: RuleStatus::Validated,
        legal: &[],
        eval: telecom::tel07,
    },
    Rule {
        id: "TEL-08",
        kind: BillKind::Telecom,
        // Base validada; a regra só emite `suspeito` por desenho ("clara" é subjetivo).
        status: RuleStatus::Validated,
        legal: &[CDC_6_III, RGC_55_I],
        eval: telecom::tel08,
    },
];

/// Status atual de uma regra, ou `None` se o id não existe.
pub fn status(rule_id: &str) -> Option<RuleStatus> {
    RULES.iter().find(|r| r.id == rule_id).map(|r| r.status)
}

/// Cap central (docs/03, princípio 5): regra a validar nunca emite `indevido`.
pub fn cap(severity: Severity, status: RuleStatus) -> Severity {
    match (status, severity) {
        (RuleStatus::Pending, Severity::Unlawful) => Severity::Suspicious,
        (_, s) => s,
    }
}

/// Avalia todas as regras da tabela [`RULES`] sobre a fatura.
pub fn evaluate(bill: &ExtractedBill, answers: &Answers) -> Evaluation {
    evaluate_with(RULES, bill, answers)
}

/// Igual a [`evaluate`], com uma tabela de regras explícita (útil em testes,
/// por exemplo para ver a severidade pretendida de uma regra já validada).
pub fn evaluate_with(rules: &[Rule], bill: &ExtractedBill, answers: &Answers) -> Evaluation {
    let mut findings = Vec::new();
    let mut questions = Vec::new();
    for rule in rules.iter().filter(|r| r.kind == bill.kind) {
        let out = (rule.eval)(bill, answers);
        questions.extend(out.questions);
        findings.extend(out.findings.into_iter().map(|d| Finding {
            id: finding_id(rule.id, &d.item_refs),
            rule_id: rule.id.to_owned(),
            severity: cap(d.severity, rule.status),
            title: d.title,
            explanation: d.explanation,
            item_refs: d.item_refs,
            original_text: d.original_text,
            page: d.page,
            amount_cents: d.amount_cents,
            legal_basis: rule.legal.iter().map(|l| l.to_ref()).collect(),
        }));
    }
    sort_findings(&mut findings);
    questions.sort_by(|a, b| a.id.cmp(&b.id));
    questions.dedup_by(|a, b| a.id == b.id);
    Evaluation {
        findings,
        questions,
    }
}

/// Id estável de um achado: "TEL-01:i2", "TEL-02:i1+i4" ou "TEL-07:-" (sem itens).
pub fn finding_id(rule_id: &str, item_refs: &[String]) -> String {
    if item_refs.is_empty() {
        format!("{rule_id}:-")
    } else {
        format!("{rule_id}:{}", item_refs.join("+"))
    }
}

/// Ordem: severidade (indevido, suspeito, ok), valor decrescente, regra, itens.
fn sort_findings(findings: &mut [Finding]) {
    findings.sort_by(|a, b| {
        a.severity
            .cmp(&b.severity)
            .then_with(|| b.amount_cents.cmp(&a.amount_cents))
            .then_with(|| a.rule_id.cmp(&b.rule_id))
            .then_with(|| a.item_refs.cmp(&b.item_refs))
            .then_with(|| a.title.cmp(&b.title))
    });
}

/// Valor recuperável: soma dos achados `indevido`; em dobro se já pago (CDC art. 42).
pub fn recoverable_cents(findings: &[Finding], paid: Option<bool>) -> Cents {
    let base: Cents = findings
        .iter()
        .filter(|f| f.severity == Severity::Unlawful)
        .map(|f| f.amount_cents)
        .fold(0, Cents::saturating_add);
    if paid == Some(true) {
        base.saturating_mul(2)
    } else {
        base
    }
}

/// Construtores de fatura para testes.
#[cfg(test)]
pub(crate) mod testutil {
    use chrono::NaiveDate;
    use humanobill_core::{BillItem, BillKind, ExtractedBill, ItemCategory, LateFees};

    pub fn item(id: &str, description: &str, cents: i64, category: ItemCategory) -> BillItem {
        BillItem {
            id: id.into(),
            description: description.into(),
            amount_cents: cents,
            category,
            period: None,
            original_text: format!("{} {cents}", description.to_uppercase()),
            page: 1,
            plain_explanation: String::new(),
        }
    }

    /// Fatura de telecom com total igual à soma dos itens.
    pub fn bill(items: Vec<BillItem>) -> ExtractedBill {
        let total = items.iter().map(|i| i.amount_cents).sum();
        ExtractedBill {
            kind: BillKind::Telecom,
            provider_name: "Vivaz Telecom".into(),
            reference_month: "2026-10".into(),
            due_date: NaiveDate::from_ymd_opt(2026, 11, 10).unwrap(),
            issue_date: None,
            total_cents: total,
            plan_name: None,
            items,
            late_fees: None,
            energy: None,
            extraction_confidence: 0.95,
        }
    }

    pub fn fees(base: i64, fine: i64, interest: i64, days: u32) -> Option<LateFees> {
        Some(LateFees {
            base_cents: base,
            fine_cents: fine,
            interest_cents: interest,
            days_late: days,
        })
    }

    pub fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::testutil::*;
    use super::*;
    use humanobill_core::{Answer, ItemCategory as C};

    fn finding(sev: Severity, cents: Cents) -> Finding {
        Finding {
            id: "TEL-XX:-".into(),
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

    fn validated_table() -> Vec<Rule> {
        RULES
            .iter()
            .map(|r| Rule {
                status: RuleStatus::Validated,
                ..*r
            })
            .collect()
    }

    /// Para cada regra, uma fatura (e respostas) que a viola com a severidade
    /// mais alta que ela pode emitir.
    fn violation(rule_id: &str) -> (ExtractedBill, Answers) {
        let mut answers = Answers::new();
        let plano = item("i1", "Plano Vivaz 20GB", 9_990, C::Plano);
        let bill = match rule_id {
            "TEL-01" => {
                answers.insert("TEL-01:i2".into(), Answer::No);
                bill(vec![
                    plano,
                    item("i2", "Clube Vivaz Música", 1_290, C::ServicoAdicional),
                ])
            }
            "TEL-02" => bill(vec![
                plano,
                item("i2", "Seguro Tela", 990, C::ServicoAdicional),
                item("i3", "Seguro Tela", 990, C::ServicoAdicional),
            ]),
            "TEL-03" => {
                let mut b = bill(vec![
                    plano,
                    item("i2", "Multa por atraso", 500, C::EncargoAtraso),
                ]);
                b.late_fees = fees(10_000, 500, 0, 0);
                b
            }
            "TEL-04" => {
                let mut b = bill(vec![
                    plano,
                    item("i2", "Juros de mora", 300, C::EncargoAtraso),
                ]);
                b.late_fees = fees(10_000, 0, 300, 30);
                b
            }
            "TEL-05" => {
                answers.insert("TEL-05:plano".into(), Answer::Amount(8_990));
                bill(vec![plano])
            }
            "TEL-06" => {
                answers.insert(
                    "TEL-06:cancelamento".into(),
                    Answer::Date(date(2026, 9, 15)),
                );
                bill(vec![plano])
            }
            "TEL-07" => {
                let mut b = bill(vec![plano]);
                b.total_cents += 1_000;
                b
            }
            "TEL-08" => bill(vec![plano, item("i2", "Serviços diversos", 700, C::Outros)]),
            other => panic!("sem cenário de violação para {other}: adicione um"),
        };
        (bill, answers)
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

    #[test]
    fn tabela_de_status_confere_com_doc03() {
        use RuleStatus::Validated;
        let esperado = [
            ("TEL-01", Validated),
            ("TEL-02", Validated),
            ("TEL-03", Validated),
            ("TEL-04", Validated),
            ("TEL-05", Validated),
            ("TEL-06", Validated),
            ("TEL-07", Validated),
            ("TEL-08", Validated),
        ];
        assert_eq!(RULES.len(), esperado.len());
        for (id, st) in esperado {
            assert_eq!(status(id), Some(st), "{id}");
        }
        assert_eq!(status("TEL-99"), None);
    }

    #[test]
    fn cap_rebaixa_indevido_so_quando_pendente() {
        use RuleStatus::{Pending, Validated};
        assert_eq!(cap(Severity::Unlawful, Pending), Severity::Suspicious);
        assert_eq!(cap(Severity::Suspicious, Pending), Severity::Suspicious);
        assert_eq!(cap(Severity::Ok, Pending), Severity::Ok);
        assert_eq!(cap(Severity::Unlawful, Validated), Severity::Unlawful);
    }

    /// Teste parametrizado: qualquer regra, se estiver `Pending`, emite no máximo `suspeito`.
    /// Rebaixa cada regra da tabela para `Pending` (hoje todas as TEL estão validadas).
    #[test]
    fn regra_pendente_nunca_emite_indevido() {
        let pendentes: Vec<Rule> = RULES
            .iter()
            .map(|r| r.with_status(RuleStatus::Pending))
            .collect();
        for rule in &pendentes {
            let (bill, answers) = violation(rule.id);
            let eval = evaluate_with(&pendentes, &bill, &answers);
            let meus: Vec<_> = eval
                .findings
                .iter()
                .filter(|f| f.rule_id == rule.id)
                .collect();
            assert!(!meus.is_empty(), "{}: cenário não violou a regra", rule.id);
            for f in meus {
                assert_ne!(
                    f.severity,
                    Severity::Unlawful,
                    "{} emitiu indevido",
                    rule.id
                );
            }
        }
    }

    /// Se a regra for validada, a severidade pretendida aparece sem mexer na regra.
    #[test]
    fn regra_validada_libera_severidade_pretendida() {
        let tabela = validated_table();
        for id in ["TEL-01", "TEL-03", "TEL-04", "TEL-06"] {
            let (bill, answers) = violation(id);
            let eval = evaluate_with(&tabela, &bill, &answers);
            assert!(
                eval.findings
                    .iter()
                    .any(|f| f.rule_id == id && f.severity == Severity::Unlawful),
                "{id} deveria pretender indevido"
            );
        }
    }

    #[test]
    fn toda_regra_tem_cenario_de_violacao() {
        for rule in RULES {
            let (bill, answers) = violation(rule.id);
            let eval = evaluate(&bill, &answers);
            assert!(
                eval.findings
                    .iter()
                    .any(|f| f.rule_id == rule.id && f.severity != Severity::Ok),
                "{} não disparou",
                rule.id
            );
        }
    }

    /// Todo achado: explicação simples (<= 25 palavras, sem juridiquês), título,
    /// valor e base legal com link (exceto TEL-07, regra de consistência).
    #[test]
    fn achados_completos_e_em_linguagem_simples() {
        let proibidas = [
            "supramencionado",
            "outrossim",
            "ensejar",
            "destarte",
            "mister",
        ];
        let mut cenarios: Vec<_> = RULES.iter().map(|r| violation(r.id)).collect();
        // Variantes: SVA sem resposta e confirmado, plano que confere, juros de 1 dia.
        let sva = bill(vec![
            item("i1", "Plano Vivaz 20GB", 9_990, C::Plano),
            item("i2", "Clube Vivaz Música", 1_290, C::ServicoAdicional),
            item("i3", "Seguro Tela", 990, C::ServicoAdicional),
        ]);
        let mut answers = Answers::new();
        answers.insert("TEL-01:i3".into(), Answer::Yes);
        answers.insert("TEL-05:plano".into(), Answer::Amount(9_990));
        cenarios.push((sva, answers));
        let mut juros = bill(vec![item("i1", "Juros", 300, C::EncargoAtraso)]);
        juros.late_fees = fees(10_000, 0, 300, 1);
        cenarios.push((juros, Answers::new()));
        let mut severidades = std::collections::BTreeSet::new();
        for tabela in [RULES.to_vec(), validated_table()] {
            for (bill, answers) in &cenarios {
                for f in evaluate_with(&tabela, bill, answers).findings {
                    severidades.insert(f.severity);
                    let palavras = f.explanation.split_whitespace().count();
                    assert!(
                        palavras <= 25,
                        "{}: {palavras} palavras: {}",
                        f.rule_id,
                        f.explanation
                    );
                    assert!(palavras > 0 && !f.title.is_empty(), "{}", f.rule_id);
                    let low = f.explanation.to_lowercase();
                    for p in proibidas {
                        assert!(!low.contains(p), "{}: juridiquês '{p}'", f.rule_id);
                    }
                    if f.rule_id == "TEL-07" {
                        assert!(f.legal_basis.is_empty());
                    } else {
                        assert!(!f.legal_basis.is_empty(), "{} sem base legal", f.rule_id);
                        for l in &f.legal_basis {
                            assert!(!l.label.is_empty() && l.url.starts_with("https://"));
                        }
                    }
                }
            }
        }
        assert_eq!(severidades.len(), 3, "cobrir indevido, suspeito e ok");
    }

    #[test]
    fn achados_ordenados_por_severidade_valor_e_regra() {
        let mut answers = Answers::new();
        answers.insert("TEL-01:i3".into(), Answer::Yes);
        let mut b = bill(vec![
            item("i1", "Plano Vivaz 20GB", 9_990, C::Plano),
            item("i2", "Clube Vivaz Música", 1_290, C::ServicoAdicional),
            item("i3", "Seguro Tela", 990, C::ServicoAdicional),
            item("i4", "Outros", 300, C::Outros),
            item("i5", "Multa por atraso", 500, C::EncargoAtraso),
        ]);
        b.late_fees = fees(10_000, 500, 0, 0);
        let tabela = validated_table();
        let eval = evaluate_with(&tabela, &b, &answers);
        let chave: Vec<_> = eval
            .findings
            .iter()
            .map(|f| (f.severity, f.amount_cents, f.rule_id.as_str()))
            .collect();
        assert_eq!(
            chave,
            vec![
                (Severity::Unlawful, 300, "TEL-03"),
                (Severity::Suspicious, 1_290, "TEL-01"),
                (Severity::Suspicious, 300, "TEL-08"),
                (Severity::Ok, 990, "TEL-01"),
            ]
        );
    }

    #[test]
    fn perguntas_ordenadas_por_id_e_sem_repeticao() {
        let b = bill(vec![
            item("i9", "Seguro Tela", 990, C::ServicoAdicional),
            item("i1", "Plano Vivaz 20GB", 9_990, C::Plano),
            item("i2", "Plano extra", 1_000, C::Plano),
            item("i3", "Clube Vivaz Música", 1_290, C::ServicoAdicional),
        ]);
        let ids: Vec<_> = evaluate(&b, &Answers::new())
            .questions
            .into_iter()
            .map(|q| q.id)
            .collect();
        assert_eq!(ids, vec!["TEL-01:i3", "TEL-01:i9", "TEL-05:plano"]);
    }

    #[test]
    fn fatura_de_energia_nao_roda_regras_tel() {
        let (mut b, answers) = violation("TEL-07");
        b.kind = BillKind::Energy;
        assert_eq!(evaluate(&b, &answers), Evaluation::default());
    }
}
