//! Regras de telecom TEL-01 a TEL-08 (docs/03, seção Telecom).
//!
//! Cada função devolve a severidade **pretendida** pela regra. O rebaixamento
//! de regras "a validar" é feito pelo motor (`crate::cap`), não aqui.

//!
//! Arredondamento: limites percentuais são calculados em centavos inteiros com
//! arredondamento "meio para cima" (`(x + d/2) / d`), e só acusamos quando o
//! valor cobrado passa do limite por mais de [`ROUNDING_TOLERANCE_CENTS`].

use std::collections::BTreeMap;

use chrono::{Datelike, NaiveDate};
use humanobill_core::{
    Answer, Answers, BillItem, Cents, ExtractedBill, ItemCategory, Question, QuestionKind, Severity,
};

use crate::Output;
use crate::shared::{dias, draft, late_fee_items, parse_month, pct_round, sum, sum_mismatch};
use crate::text::{brl, has_word, normalized, tokens};

/// Folga para arredondamentos nos limites de multa, juros e plano.
pub(crate) const ROUNDING_TOLERANCE_CENTS: Cents = 1;

/// Palavras (radicais no singular, sem acento) que indicam serviço adicional (TEL-01).
const SVA_KEYWORDS: &[&str] = &[
    "seguro",
    "clube",
    "conteudo",
    "banca",
    "musica",
    "jogo",
    "antivirus",
    "backup",
    "cloud",
    "assistencia",
    "protecao",
];

/// Palavras que, sozinhas, não dizem o que é a cobrança (TEL-08).
const GENERIC_WORDS: &[&str] = &["outro", "outra", "diverso", "diversa", "servico", "ajuste"];

/// Palavras de ligação ignoradas na TEL-08.
const STOPWORDS: &[&str] = &["de", "da", "do", "das", "dos", "e"];

pub(crate) const Q_TEL05_PLAN: &str = "TEL-05:plano";
pub(crate) const Q_TEL06_CANCEL: &str = "TEL-06:cancelamento";

fn q_tel01(item_id: &str) -> String {
    format!("TEL-01:{item_id}")
}

/// TEL-01: serviço adicional (SVA). Sem resposta: suspeito. "Não contratei":
/// indevido (pretendido). "Contratei": ok.
pub(crate) fn tel01(bill: &ExtractedBill, answers: &Answers) -> Output {
    let mut out = Output::default();
    for item in bill.items.iter().filter(|i| is_sva(i)) {
        let qid = q_tel01(&item.id);
        out.questions.push(Question {
            id: qid.clone(),
            rule_id: "TEL-01".into(),
            item_id: Some(item.id.clone()),
            text: format!("Você contratou {}?", item.description),
            kind: QuestionKind::YesNo,
        });
        let (severity, title, explanation) = match answers.get(&qid) {
            Some(Answer::No) => (
                Severity::Unlawful,
                format!("Serviço não contratado: {}", item.description),
                "Você disse que não contratou este serviço. Cobrança sem pedido pode ser \
                 contestada e o valor, devolvido."
                    .to_owned(),
            ),
            Some(Answer::Yes) => (
                Severity::Ok,
                format!("Serviço confirmado: {}", item.description),
                "Você confirmou que contratou este serviço. Nada a contestar aqui.".to_owned(),
            ),
            _ => (
                Severity::Suspicious,
                format!("Serviço extra: {}", item.description),
                "Serviço cobrado à parte do plano. Se você não pediu, pode contestar e pedir \
                 o dinheiro de volta."
                    .to_owned(),
            ),
        };
        out.findings.push(draft(
            severity,
            title,
            explanation,
            &[item],
            item.amount_cents,
        ));
    }
    out
}

fn is_sva(item: &BillItem) -> bool {
    match item.category {
        ItemCategory::Plano | ItemCategory::Imposto | ItemCategory::EncargoAtraso => false,
        ItemCategory::ServicoAdicional => true,
        _ => {
            let t = tokens(&item.description);
            SVA_KEYWORDS.iter().any(|k| has_word(&t, k))
        }
    }
}

/// TEL-02: itens com mesma descrição normalizada, mesmo valor e mesmo período.
pub(crate) fn tel02(bill: &ExtractedBill, _answers: &Answers) -> Output {
    type Key = (String, Cents, Option<(NaiveDate, NaiveDate)>);
    let mut groups: BTreeMap<Key, Vec<&BillItem>> = BTreeMap::new();
    // Itens sem valor positivo (descontos, zerados) não geram cobrança em dobro.
    for item in bill.items.iter().filter(|i| i.amount_cents > 0) {
        let period = item.period.as_ref().map(|p| (p.start, p.end));
        groups
            .entry((normalized(&item.description), item.amount_cents, period))
            .or_default()
            .push(item);
    }
    let mut out = Output::default();
    for ((_, amount, _), items) in groups.into_iter().filter(|(_, v)| v.len() > 1) {
        let n = items.len();
        let extra = Cents::try_from(n - 1).unwrap_or(Cents::MAX);
        out.findings.push(draft(
            Severity::Suspicious,
            format!("Cobrança repetida: {}", items[0].description),
            format!(
                "Este item aparece {n} vezes com o mesmo valor. Confira se não foi cobrado \
                 mais de uma vez."
            ),
            &items,
            amount.saturating_mul(extra),
        ));
    }
    out
}

/// TEL-03: multa por atraso acima de 2% do valor em atraso.
pub(crate) fn tel03(bill: &ExtractedBill, _answers: &Answers) -> Output {
    let mut out = Output::default();
    let Some(fees) = bill.late_fees.as_ref().filter(|f| f.base_cents > 0) else {
        return out;
    };
    let allowed = pct_round(fees.base_cents, 2, 100);
    if fees.fine_cents > allowed.saturating_add(ROUNDING_TOLERANCE_CENTS) {
        out.findings.push(draft(
            Severity::Unlawful,
            "Multa por atraso acima de 2%".into(),
            format!(
                "A multa foi {}, mais que 2% do valor em atraso ({}). Acima de 2% pode ser \
                 abusivo.",
                brl(fees.fine_cents),
                brl(allowed)
            ),
            &late_fee_items(bill, &["multa"]),
            fees.fine_cents - allowed,
        ));
    }
    out
}

/// TEL-04: juros de mora acima de 1% ao mês, pro rata dia (1%/30 por dia).
pub(crate) fn tel04(bill: &ExtractedBill, _answers: &Answers) -> Output {
    let mut out = Output::default();
    let Some(fees) = bill.late_fees.as_ref().filter(|f| f.base_cents > 0) else {
        return out;
    };
    let days = Cents::from(fees.days_late);
    let allowed = pct_round(fees.base_cents, days, 3_000);
    if fees.interest_cents > allowed.saturating_add(ROUNDING_TOLERANCE_CENTS) {
        let dias = dias(days);
        out.findings.push(draft(
            Severity::Unlawful,
            "Juros de atraso acima de 1% ao mês".into(),
            format!(
                "Os juros foram {}. Para {dias} de atraso, 1% ao mês daria no máximo {}.",
                brl(fees.interest_cents),
                brl(allowed)
            ),
            &late_fee_items(bill, &["juro", "mora"]),
            fees.interest_cents - allowed,
        ));
    }
    out
}

/// TEL-05: soma dos itens de plano acima do valor do plano informado.
pub(crate) fn tel05(bill: &ExtractedBill, answers: &Answers) -> Output {
    let mut out = Output::default();
    let plans: Vec<&BillItem> = bill
        .items
        .iter()
        .filter(|i| i.category == ItemCategory::Plano)
        .collect();
    if plans.is_empty() {
        return out;
    }
    out.questions.push(Question {
        id: Q_TEL05_PLAN.into(),
        rule_id: "TEL-05".into(),
        item_id: None,
        text: "Quanto custa o seu plano contratado?".into(),
        kind: QuestionKind::Amount,
    });
    let Some(&Answer::Amount(price)) = answers.get(Q_TEL05_PLAN) else {
        return out;
    };
    if price < 0 {
        return out;
    }
    let charged = sum(&plans);
    let finding = if charged > price.saturating_add(ROUNDING_TOLERANCE_CENTS) {
        draft(
            Severity::Suspicious,
            "Mensalidade acima do plano".into(),
            format!(
                "A mensalidade cobrada é {}, mas você disse que o plano custa {}. A oferta deve \
                 ser cumprida.",
                brl(charged),
                brl(price)
            ),
            &plans,
            charged - price,
        )
    } else {
        draft(
            Severity::Ok,
            "Mensalidade confere com o plano".into(),
            "A mensalidade cobrada bate com o valor do plano que você informou.".into(),
            &plans,
            0,
        )
    };
    out.findings.push(finding);
    out
}

/// TEL-06: mês de referência posterior ao mês do cancelamento informado.
pub(crate) fn tel06(bill: &ExtractedBill, answers: &Answers) -> Output {
    let mut out = Output::default();
    let Some(&Answer::Date(cancel)) = answers.get(Q_TEL06_CANCEL) else {
        return out;
    };
    let Some((year, month)) = parse_month(&bill.reference_month) else {
        return out;
    };
    if (year, month) <= (cancel.year(), cancel.month()) {
        return out;
    }
    let charged: Vec<&BillItem> = bill
        .items
        .iter()
        .filter(|i| {
            matches!(
                i.category,
                ItemCategory::Plano | ItemCategory::ServicoAdicional
            )
        })
        .collect();
    if charged.is_empty() {
        return out;
    }
    out.findings.push(draft(
        Severity::Unlawful,
        "Cobrança depois do cancelamento".into(),
        format!(
            "Esta fatura é de {month:02}/{year}, depois do cancelamento que você informou. \
             Plano e serviços não deveriam ser cobrados."
        ),
        &charged,
        sum(&charged),
    ));
    out
}

/// TEL-07: soma dos itens diferente do total, além da tolerância do core.
pub(crate) fn tel07(bill: &ExtractedBill, _answers: &Answers) -> Output {
    sum_mismatch(bill)
}

/// TEL-08: descrição genérica ("outros", "serviços diversos", "ajuste") ou só código.
pub(crate) fn tel08(bill: &ExtractedBill, _answers: &Answers) -> Output {
    let mut out = Output::default();
    for item in bill.items.iter().filter(|i| is_generic(&i.description)) {
        let title = if normalized(&item.description).is_empty() {
            "Item sem descrição clara".to_owned()
        } else {
            format!("Item sem descrição clara: {}", item.description.trim())
        };
        out.findings.push(draft(
            Severity::Suspicious,
            title,
            "Não dá para saber o que é esta cobrança. A empresa deve explicar cada item com \
             clareza."
                .into(),
            &[item],
            item.amount_cents,
        ));
    }
    out
}

/// Genérica = toda palavra é genérica, de ligação ou código (tem dígito).
/// Descrição vazia ou só com símbolos também conta.
fn is_generic(description: &str) -> bool {
    tokens(description).iter().all(|t| {
        let one = std::slice::from_ref(t);
        STOPWORDS.contains(&t.as_str())
            || t.chars().any(|c| c.is_ascii_digit())
            || GENERIC_WORDS.iter().any(|g| has_word(one, g))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::*;
    use humanobill_core::{Answer, ItemCategory as C, Period, QuestionKind, Severity};

    fn ans(pairs: &[(&str, Answer)]) -> Answers {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_owned(), v.clone()))
            .collect()
    }

    fn sev_refs(out: &Output) -> Vec<(Severity, Vec<String>, i64)> {
        out.findings
            .iter()
            .map(|d| (d.severity, d.item_refs.clone(), d.amount_cents))
            .collect()
    }

    fn refs(ids: &[&str]) -> Vec<String> {
        ids.iter().map(|s| (*s).to_owned()).collect()
    }

    fn plano() -> humanobill_core::BillItem {
        item("i1", "Plano Vivaz Controle 20GB", 9_990, C::Plano)
    }

    // ---------- TEL-01: serviço adicional ----------

    #[test]
    fn tel01_sva_por_categoria_sem_resposta_e_suspeito() {
        let b = bill(vec![
            plano(),
            item("i2", "Clube Vivaz Música", 1_290, C::ServicoAdicional),
        ]);
        let out = tel01(&b, &Answers::new());
        assert_eq!(
            sev_refs(&out),
            vec![(Severity::Suspicious, refs(&["i2"]), 1_290)]
        );
        let d = &out.findings[0];
        assert_eq!(d.original_text.as_deref(), Some("CLUBE VIVAZ MÚSICA 1290"));
        assert_eq!(d.page, Some(1));
    }

    #[test]
    fn tel01_sva_por_palavra_chave_mesmo_em_outros() {
        for desc in [
            "Proteção de Tela",
            "PACOTE JOGOS",
            "Antivírus Mobile",
            "Banca Digital",
            "Backup na nuvem",
            "Vivaz Cloud 50GB",
            "Assistência 24h",
            "Seguros Diversos",
            "Conteúdo Kids",
        ] {
            let b = bill(vec![plano(), item("i2", desc, 500, C::Outros)]);
            let out = tel01(&b, &Answers::new());
            assert_eq!(
                sev_refs(&out),
                vec![(Severity::Suspicious, refs(&["i2"]), 500)],
                "{desc}"
            );
        }
    }

    #[test]
    fn tel01_nao_marca_item_comum() {
        let b = bill(vec![
            plano(),
            item("i2", "Parcela do aparelho 3/12", 5_000, C::Outros),
            item("i3", "Tarifa bancária", 200, C::Outros),
            item("i4", "Segurança da rede", 100, C::Outros),
        ]);
        let out = tel01(&b, &Answers::new());
        assert!(out.findings.is_empty(), "{:?}", out.findings);
        assert!(out.questions.is_empty());
    }

    #[test]
    fn tel01_ignora_plano_imposto_e_encargo() {
        let b = bill(vec![
            item("i1", "Plano Vivaz com Clube Música", 9_990, C::Plano),
            item("i2", "Seguro obrigatório (imposto)", 100, C::Imposto),
            item("i3", "Multa seguro atraso", 100, C::EncargoAtraso),
        ]);
        let out = tel01(&b, &Answers::new());
        assert!(out.findings.is_empty());
        assert!(out.questions.is_empty());
    }

    #[test]
    fn tel01_gera_pergunta_por_item() {
        let b = bill(vec![
            plano(),
            item("i2", "Clube Vivaz Música", 1_290, C::ServicoAdicional),
        ]);
        for answers in [Answers::new(), ans(&[("TEL-01:i2", Answer::No)])] {
            let out = tel01(&b, &answers);
            assert_eq!(out.questions.len(), 1);
            let q = &out.questions[0];
            assert_eq!(q.id, "TEL-01:i2");
            assert_eq!(q.rule_id, "TEL-01");
            assert_eq!(q.item_id.as_deref(), Some("i2"));
            assert_eq!(q.text, "Você contratou Clube Vivaz Música?");
            assert_eq!(q.kind, QuestionKind::YesNo);
        }
    }

    #[test]
    fn tel01_resposta_nao_pretende_indevido() {
        let b = bill(vec![
            plano(),
            item("i2", "Clube Vivaz Música", 1_290, C::ServicoAdicional),
        ]);
        let out = tel01(&b, &ans(&[("TEL-01:i2", Answer::No)]));
        assert_eq!(
            sev_refs(&out),
            vec![(Severity::Unlawful, refs(&["i2"]), 1_290)]
        );
    }

    #[test]
    fn tel01_resposta_sim_e_ok() {
        let b = bill(vec![
            plano(),
            item("i2", "Clube Vivaz Música", 1_290, C::ServicoAdicional),
        ]);
        let out = tel01(&b, &ans(&[("TEL-01:i2", Answer::Yes)]));
        assert_eq!(sev_refs(&out), vec![(Severity::Ok, refs(&["i2"]), 1_290)]);
    }

    #[test]
    fn tel01_resposta_de_tipo_errado_conta_como_sem_resposta() {
        let b = bill(vec![
            plano(),
            item("i2", "Clube Vivaz Música", 1_290, C::ServicoAdicional),
        ]);
        let out = tel01(&b, &ans(&[("TEL-01:i2", Answer::Amount(1))]));
        assert_eq!(out.findings[0].severity, Severity::Suspicious);
    }

    // ---------- TEL-02: cobrança duplicada ----------

    #[test]
    fn tel02_duplicado_mesmo_valor_e_suspeito() {
        let b = bill(vec![
            plano(),
            item("i2", "Seguro Tela", 990, C::ServicoAdicional),
            item("i3", "Seguro Tela", 990, C::ServicoAdicional),
        ]);
        let out = tel02(&b, &Answers::new());
        assert_eq!(
            sev_refs(&out),
            vec![(Severity::Suspicious, refs(&["i2", "i3"]), 990)]
        );
    }

    #[test]
    fn tel02_triplicado_soma_so_as_copias_excedentes() {
        let b = bill(vec![
            item("i1", "Seguro Tela", 990, C::ServicoAdicional),
            item("i2", "Seguro Tela", 990, C::ServicoAdicional),
            item("i3", "Seguro Tela", 990, C::ServicoAdicional),
        ]);
        let out = tel02(&b, &Answers::new());
        assert_eq!(
            sev_refs(&out),
            vec![(Severity::Suspicious, refs(&["i1", "i2", "i3"]), 1_980)]
        );
    }

    #[test]
    fn tel02_compara_descricao_normalizada() {
        let b = bill(vec![
            item("i1", "Seguro Tela", 990, C::ServicoAdicional),
            item("i2", "SEGURO  TELA.", 990, C::ServicoAdicional),
        ]);
        let out = tel02(&b, &Answers::new());
        assert_eq!(out.findings.len(), 1);
    }

    #[test]
    fn tel02_valor_diferente_nao_e_duplicado() {
        let b = bill(vec![
            item("i1", "Seguro Tela", 990, C::ServicoAdicional),
            item("i2", "Seguro Tela", 991, C::ServicoAdicional),
        ]);
        assert!(tel02(&b, &Answers::new()).findings.is_empty());
    }

    #[test]
    fn tel02_periodos_diferentes_nao_sao_duplicados() {
        let mut a = item("i1", "Seguro Tela", 990, C::ServicoAdicional);
        let mut c = item("i2", "Seguro Tela", 990, C::ServicoAdicional);
        a.period = Some(Period {
            start: date(2026, 9, 1),
            end: date(2026, 9, 30),
        });
        c.period = Some(Period {
            start: date(2026, 10, 1),
            end: date(2026, 10, 31),
        });
        assert!(
            tel02(&bill(vec![a, c]), &Answers::new())
                .findings
                .is_empty()
        );
    }

    #[test]
    fn tel02_ignora_itens_sem_valor_positivo() {
        let b = bill(vec![
            item("i1", "Desconto fidelidade", -500, C::Outros),
            item("i2", "Desconto fidelidade", -500, C::Outros),
        ]);
        assert!(tel02(&b, &Answers::new()).findings.is_empty());
    }

    // ---------- TEL-03: multa acima de 2% ----------

    fn com_encargos(fine: i64, interest: i64, base: i64, days: u32) -> ExtractedBill {
        let mut b = bill(vec![
            plano(),
            item("i2", "Multa por atraso", fine, C::EncargoAtraso),
            item("i3", "Juros de mora", interest, C::EncargoAtraso),
        ]);
        b.late_fees = fees(base, fine, interest, days);
        b
    }

    #[test]
    fn tel03_multa_de_5_por_cento_sobre_100_reais_excede_3_reais() {
        let out = tel03(&com_encargos(500, 0, 10_000, 0), &Answers::new());
        assert_eq!(
            sev_refs(&out),
            vec![(Severity::Unlawful, refs(&["i2"]), 300)]
        );
        assert_eq!(
            out.findings[0].original_text.as_deref(),
            Some("MULTA POR ATRASO 500")
        );
    }

    #[test]
    fn tel03_multa_de_exatamente_2_por_cento_nao_acusa() {
        assert!(
            tel03(&com_encargos(200, 0, 10_000, 0), &Answers::new())
                .findings
                .is_empty()
        );
    }

    #[test]
    fn tel03_tolerancia_de_1_centavo_no_arredondamento() {
        // 2% de R$ 119,90 = 239,8 centavos -> arredonda para 240.
        assert!(
            tel03(&com_encargos(241, 0, 11_990, 0), &Answers::new())
                .findings
                .is_empty()
        );
        let out = tel03(&com_encargos(242, 0, 11_990, 0), &Answers::new());
        assert_eq!(sev_refs(&out), vec![(Severity::Unlawful, refs(&["i2"]), 2)]);
    }

    #[test]
    fn tel03_sem_encargos_nao_avalia() {
        let b = bill(vec![
            plano(),
            item("i2", "Multa por atraso", 900, C::EncargoAtraso),
        ]);
        assert!(tel03(&b, &Answers::new()).findings.is_empty());
    }

    #[test]
    fn tel03_sem_item_de_multa_acusa_sem_referencia() {
        let mut b = bill(vec![plano(), item("i2", "Encargos", 900, C::EncargoAtraso)]);
        b.late_fees = fees(10_000, 900, 0, 0);
        let out = tel03(&b, &Answers::new());
        assert_eq!(sev_refs(&out), vec![(Severity::Unlawful, vec![], 700)]);
        assert_eq!(out.findings[0].original_text, None);
    }

    // ---------- TEL-04: juros acima de 1% ao mês ----------

    #[test]
    fn tel04_juros_de_3_por_cento_em_30_dias() {
        // Base R$ 119,90, 30 dias: permitido 1% = 119,9 -> 120 centavos.
        let out = tel04(&com_encargos(0, 360, 11_990, 30), &Answers::new());
        assert_eq!(
            sev_refs(&out),
            vec![(Severity::Unlawful, refs(&["i3"]), 240)]
        );
    }

    #[test]
    fn tel04_juros_pro_rata_dentro_do_limite_nao_acusa() {
        // R$ 100,00 por 15 dias: limite R$ 0,50.
        assert!(
            tel04(&com_encargos(0, 50, 10_000, 15), &Answers::new())
                .findings
                .is_empty()
        );
        assert!(
            tel04(&com_encargos(0, 51, 10_000, 15), &Answers::new())
                .findings
                .is_empty()
        );
        let out = tel04(&com_encargos(0, 52, 10_000, 15), &Answers::new());
        assert_eq!(sev_refs(&out), vec![(Severity::Unlawful, refs(&["i3"]), 2)]);
    }

    #[test]
    fn tel04_sem_encargos_nao_avalia() {
        let b = bill(vec![
            plano(),
            item("i3", "Juros de mora", 900, C::EncargoAtraso),
        ]);
        assert!(tel04(&b, &Answers::new()).findings.is_empty());
    }

    // ---------- TEL-05: mensalidade acima do plano ----------

    #[test]
    fn tel05_sem_resposta_so_pergunta() {
        let out = tel05(&bill(vec![plano()]), &Answers::new());
        assert!(out.findings.is_empty());
        assert_eq!(out.questions.len(), 1);
        let q = &out.questions[0];
        assert_eq!(q.id, "TEL-05:plano");
        assert_eq!(q.text, "Quanto custa o seu plano contratado?");
        assert_eq!(q.kind, QuestionKind::Amount);
        assert_eq!(q.item_id, None);
    }

    #[test]
    fn tel05_mensalidade_acima_do_plano_e_suspeito() {
        let out = tel05(
            &bill(vec![plano()]),
            &ans(&[("TEL-05:plano", Answer::Amount(8_990))]),
        );
        assert_eq!(
            sev_refs(&out),
            vec![(Severity::Suspicious, refs(&["i1"]), 1_000)]
        );
        assert_eq!(
            out.questions.len(),
            1,
            "pergunta continua disponível para mudar a resposta"
        );
    }

    #[test]
    fn tel05_mensalidade_igual_ao_plano_e_ok() {
        let out = tel05(
            &bill(vec![plano()]),
            &ans(&[("TEL-05:plano", Answer::Amount(9_990))]),
        );
        assert_eq!(sev_refs(&out), vec![(Severity::Ok, refs(&["i1"]), 0)]);
    }

    #[test]
    fn tel05_tolerancia_de_1_centavo() {
        let out = tel05(
            &bill(vec![plano()]),
            &ans(&[("TEL-05:plano", Answer::Amount(9_989))]),
        );
        assert_eq!(out.findings[0].severity, Severity::Ok);
        let out = tel05(
            &bill(vec![plano()]),
            &ans(&[("TEL-05:plano", Answer::Amount(9_988))]),
        );
        assert_eq!(
            sev_refs(&out),
            vec![(Severity::Suspicious, refs(&["i1"]), 2)]
        );
    }

    #[test]
    fn tel05_soma_todos_os_itens_de_plano() {
        let b = bill(vec![
            plano(),
            item("i2", "Plano adicional TV", 2_000, C::Plano),
        ]);
        let out = tel05(&b, &ans(&[("TEL-05:plano", Answer::Amount(9_990))]));
        assert_eq!(
            sev_refs(&out),
            vec![(Severity::Suspicious, refs(&["i1", "i2"]), 2_000)]
        );
    }

    #[test]
    fn tel05_sem_item_de_plano_nao_pergunta() {
        let b = bill(vec![item("i1", "Parcela do aparelho", 5_000, C::Outros)]);
        let out = tel05(&b, &ans(&[("TEL-05:plano", Answer::Amount(1))]));
        assert_eq!(out, Output::default());
    }

    // ---------- TEL-06: cobrança após cancelamento ----------

    #[test]
    fn tel06_fatura_depois_do_cancelamento_pretende_indevido() {
        let b = bill(vec![
            plano(),
            item("i2", "Clube Vivaz Música", 1_290, C::ServicoAdicional),
            item("i3", "Parcela do aparelho", 5_000, C::Outros),
        ]);
        let out = tel06(
            &b,
            &ans(&[("TEL-06:cancelamento", Answer::Date(date(2026, 9, 15)))]),
        );
        assert_eq!(
            sev_refs(&out),
            vec![(Severity::Unlawful, refs(&["i1", "i2"]), 11_280)]
        );
    }

    #[test]
    fn tel06_cancelamento_no_mesmo_mes_nao_acusa() {
        let b = bill(vec![plano()]);
        let out = tel06(
            &b,
            &ans(&[("TEL-06:cancelamento", Answer::Date(date(2026, 10, 5)))]),
        );
        assert!(out.findings.is_empty());
        let out = tel06(
            &b,
            &ans(&[("TEL-06:cancelamento", Answer::Date(date(2026, 11, 1)))]),
        );
        assert!(out.findings.is_empty());
    }

    #[test]
    fn tel06_virada_de_ano() {
        let mut b = bill(vec![plano()]);
        b.reference_month = "2027-01".into();
        let out = tel06(
            &b,
            &ans(&[("TEL-06:cancelamento", Answer::Date(date(2026, 12, 20)))]),
        );
        assert_eq!(out.findings.len(), 1);
    }

    #[test]
    fn tel06_sem_resposta_nao_avalia_nem_pergunta() {
        assert_eq!(
            tel06(&bill(vec![plano()]), &Answers::new()),
            Output::default()
        );
    }

    #[test]
    fn tel06_mes_de_referencia_invalido_nao_acusa() {
        let mut b = bill(vec![plano()]);
        b.reference_month = "outubro".into();
        let out = tel06(
            &b,
            &ans(&[("TEL-06:cancelamento", Answer::Date(date(2026, 1, 1)))]),
        );
        assert!(out.findings.is_empty());
    }

    // ---------- TEL-07: soma não bate ----------

    #[test]
    fn tel07_soma_diferente_do_total_e_suspeito() {
        let mut b = bill(vec![plano()]);
        b.total_cents = 10_090;
        let out = tel07(&b, &Answers::new());
        assert_eq!(sev_refs(&out), vec![(Severity::Suspicious, vec![], 100)]);
    }

    #[test]
    fn tel07_total_menor_que_a_soma_usa_diferenca_absoluta() {
        let mut b = bill(vec![plano()]);
        b.total_cents = 9_000;
        assert_eq!(tel07(&b, &Answers::new()).findings[0].amount_cents, 990);
    }

    #[test]
    fn tel07_soma_igual_ao_total_nao_marca() {
        let mut b = bill(vec![plano(), item("i2", "ICMS", 100, C::Imposto)]);
        b.total_cents = b.items.iter().map(|i| i.amount_cents).sum();
        assert!(tel07(&b, &Answers::new()).findings.is_empty());
    }

    #[test]
    fn tel07_tolerancia_de_5_centavos() {
        let mut b = bill(vec![plano()]);
        b.total_cents = 9_995;
        assert!(tel07(&b, &Answers::new()).findings.is_empty());
        b.total_cents = 9_996;
        assert_eq!(tel07(&b, &Answers::new()).findings.len(), 1);
    }

    // ---------- TEL-08: item sem descrição clara ----------

    #[test]
    fn tel08_descricao_generica_e_suspeito() {
        for desc in [
            "Outros",
            "DIVERSOS",
            "Serviços diversos",
            "Ajuste",
            "Ajustes 09/26",
            "123-456",
            "",
            "  -  ",
        ] {
            let b = bill(vec![plano(), item("i2", desc, 700, C::Outros)]);
            let out = tel08(&b, &Answers::new());
            assert_eq!(
                sev_refs(&out),
                vec![(Severity::Suspicious, refs(&["i2"]), 700)],
                "{desc:?}"
            );
        }
    }

    #[test]
    fn tel08_descricao_clara_nao_marca() {
        let b = bill(vec![
            plano(),
            item("i2", "Parcela do aparelho 3/12", 5_000, C::Outros),
            item("i3", "ICMS", 100, C::Imposto),
            item("i4", "Outros serviços de roaming", 100, C::Outros),
        ]);
        assert!(tel08(&b, &Answers::new()).findings.is_empty());
    }

    /// Borda: só palavras genéricas + números/conectivos ainda é genérico;
    /// uma palavra específica já basta para a descrição ser clara.
    #[test]
    fn tel08_borda_generico_com_numeros_e_conectivos() {
        for desc in ["Outros 123", "Serviços diversos de 10/2026", "Ajustes"] {
            let b = bill(vec![plano(), item("i2", desc, 100, C::Outros)]);
            assert_eq!(tel08(&b, &Answers::new()).findings.len(), 1, "{desc}");
        }
        for desc in ["Serviço de roaming", "Ajuste de franquia"] {
            let b = bill(vec![plano(), item("i2", desc, 100, C::Outros)]);
            assert!(tel08(&b, &Answers::new()).findings.is_empty(), "{desc}");
        }
    }
}
