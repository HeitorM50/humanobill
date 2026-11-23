//! Regras da fatura do cartão de crédito CAR-01 a CAR-05 (docs/03, seção Cartão).
//!
//! A fatura extraída traz as tarifas, serviços e encargos do emissor um a um; as
//! compras entram somadas num item só, sem nome de estabelecimento (prompt do
//! `crates/extract`). Cada função devolve a severidade **pretendida**; o
//! rebaixamento de regras "a validar" é feito pelo motor (`crate::cap`).

use humanobill_core::{
    Answer, Answers, BillItem, ExtractedBill, ItemCategory, Question, QuestionKind, Severity,
};

use crate::Output;
use crate::shared::{
    Asked, ask_per_item, draft, has_any_word, late_fee_items, pct_round, sum, sum_mismatch,
};
use crate::text::{brl, fold};

/// Folga de arredondamento no teto da multa (igual à TEL-03).
const ROUNDING_TOLERANCE_CENTS: i64 = 1;

/// Palavras de seguro ou serviço vendido junto com o cartão (CAR-01).
const SERVICE_KEYWORDS: &[&str] = &[
    "seguro",
    "protecao",
    "assistencia",
    "sms",
    "mensagem",
    "clube",
    "capitalizacao",
];

/// Trechos (sem acento, minúsculos) das tarifas previstas para o cartão de crédito
/// (Res. CMN 3.919/2010, art. 3º, V e Tabela I, item 5; art. 5º, XVIII e XIX).
const ALLOWED_FEE_MARKERS: &[&str] = &[
    "anuidade",
    "2a via",
    "2ª via",
    "segunda via",
    "saque",
    "retirada",
    "pagamento de conta",
    "pagamentocontas",
    "emerg",
    "plastico",
];

pub(crate) const Q_CAR03_ANNUAL_FEE: &str = "CAR-03:anuidade";

fn is_annual_fee(item: &BillItem) -> bool {
    item.amount_cents > 0 && has_any_word(item, &["anuidade"])
}

/// Seguro ou serviço cobrado (não a anuidade, nem encargo ou imposto).
fn is_service(item: &BillItem) -> bool {
    item.amount_cents > 0
        && !is_annual_fee(item)
        && match item.category {
            ItemCategory::ServicoAdicional => true,
            ItemCategory::Tarifa | ItemCategory::Outros | ItemCategory::Plano => {
                has_any_word(item, SERVICE_KEYWORDS)
            }
            _ => false,
        }
}

/// CAR-01: seguro ou serviço cobrado no cartão (seguro, proteção, SMS...).
/// Sem resposta: suspeito. "Não contratei": indevido (pretendido). "Contratei": ok.
pub(crate) fn car01(bill: &ExtractedBill, answers: &Answers) -> Output {
    let mut out = Output::default();
    for item in bill.items.iter().filter(|i| is_service(i)) {
        let d = &item.description;
        ask_per_item(
            &mut out,
            item,
            answers,
            Asked {
                rule_id: "CAR-01",
                question: format!("Você contratou {d}?"),
                open: (
                    format!("Serviço extra no cartão: {d}"),
                    "Seguro ou serviço no cartão só pode ser cobrado se você pediu. Se não pediu, \
                     pode contestar e pedir o dinheiro de volta.",
                ),
                no: (
                    format!("Serviço não contratado: {d}"),
                    "Você disse que não contratou este serviço. Cobrança sem pedido não precisa \
                     ser paga e o valor pode ser devolvido.",
                ),
                yes: (
                    format!("Serviço confirmado: {d}"),
                    "Você confirmou que contratou este serviço. Nada a contestar aqui.",
                ),
            },
        );
    }
    out
}

/// CAR-02: tarifa fora da lista de tarifas do cartão de crédito. Sempre suspeito:
/// a norma lista o que pode, sem dizer "vedadas as demais", e o nome da tarifa
/// na fatura pode não deixar claro qual é.
pub(crate) fn car02(bill: &ExtractedBill, _answers: &Answers) -> Output {
    let mut out = Output::default();
    let unexpected = bill.items.iter().filter(|i| {
        i.category == ItemCategory::Tarifa && i.amount_cents > 0 && !is_service(i) && {
            let text = fold(&format!("{} {}", i.description, i.original_text));
            !ALLOWED_FEE_MARKERS.iter().any(|m| text.contains(m))
        }
    });
    for item in unexpected {
        out.findings.push(draft(
            Severity::Suspicious,
            format!("Tarifa fora da lista do cartão: {}", item.description),
            "No cartão de crédito, as tarifas previstas são anuidade, 2ª via, saque, pagamento \
             de contas e avaliação emergencial de crédito."
                .into(),
            &[item],
            item.amount_cents,
        ));
    }
    out
}

/// CAR-03: anuidade cobrada em cartão oferecido sem anuidade (a oferta vincula).
/// Sem resposta: só a pergunta. "Foi oferecido sem anuidade": suspeito. "Não": ok.
pub(crate) fn car03(bill: &ExtractedBill, answers: &Answers) -> Output {
    let mut out = Output::default();
    let fees: Vec<&BillItem> = bill.items.iter().filter(|i| is_annual_fee(i)).collect();
    if fees.is_empty() {
        return out;
    }
    out.questions.push(Question {
        id: Q_CAR03_ANNUAL_FEE.into(),
        rule_id: "CAR-03".into(),
        item_id: None,
        text: "O cartão foi oferecido a você sem anuidade?".into(),
        kind: QuestionKind::YesNo,
    });
    let finding = match answers.get(Q_CAR03_ANNUAL_FEE) {
        Some(Answer::Yes) => draft(
            Severity::Suspicious,
            "Anuidade em cartão oferecido sem anuidade".into(),
            "Você disse que o cartão veio sem anuidade. A oferta deve ser cumprida: guarde \
             o anúncio ou a mensagem da oferta."
                .into(),
            &fees,
            sum(&fees),
        ),
        Some(Answer::No) => draft(
            Severity::Ok,
            "Anuidade prevista na oferta".into(),
            "Você disse que o cartão não foi oferecido sem anuidade. Nada a contestar aqui.".into(),
            &fees,
            sum(&fees),
        ),
        _ => return out,
    };
    out.findings.push(finding);
    out
}

/// CAR-04: multa por atraso acima de 2% do valor em atraso (CDC art. 52, § 1º).
pub(crate) fn car04(bill: &ExtractedBill, _answers: &Answers) -> Output {
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
                "A multa foi {}, mais que 2% do valor em atraso ({}). No cartão, o limite da \
                 multa é 2%.",
                brl(fees.fine_cents),
                brl(allowed)
            ),
            &late_fee_items(bill, &["multa"]),
            fees.fine_cents - allowed,
        ));
    }
    out
}

/// CAR-05: soma dos itens diferente do total (igual à TEL-07).
pub(crate) fn car05(bill: &ExtractedBill, _answers: &Answers) -> Output {
    sum_mismatch(bill)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::*;
    use humanobill_core::{Answer, BillItem, ItemCategory as C, QuestionKind, Severity};

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

    fn compras() -> BillItem {
        item("i1", "Compras e lançamentos do período", 118_040, C::Outros)
    }

    fn anuidade() -> BillItem {
        item("i2", "Anuidade Aurora Card 03/12", 3_200, C::Tarifa)
    }

    fn seguro() -> BillItem {
        item("i3", "Seguro Perda e Roubo", 990, C::ServicoAdicional)
    }

    fn manutencao() -> BillItem {
        item("i4", "Tarifa manutenção cartão", 890, C::Tarifa)
    }

    // ---------- CAR-01: seguro ou serviço no cartão ----------

    #[test]
    fn car01_seguro_sem_resposta_e_suspeito_com_pergunta() {
        let b = card_bill(vec![compras(), anuidade(), seguro()]);
        let out = car01(&b, &Answers::new());
        assert_eq!(
            sev_refs(&out),
            vec![(Severity::Suspicious, refs(&["i3"]), 990)]
        );
        let q = &out.questions[0];
        assert_eq!(q.id, "CAR-01:i3");
        assert_eq!(q.rule_id, "CAR-01");
        assert_eq!(q.item_id.as_deref(), Some("i3"));
        assert_eq!(q.text, "Você contratou Seguro Perda e Roubo?");
        assert_eq!(q.kind, QuestionKind::YesNo);
        assert_eq!(
            out.findings[0].original_text.as_deref(),
            Some("SEGURO PERDA E ROUBO 990")
        );
    }

    #[test]
    fn car01_nao_contratei_pretende_indevido_e_contratei_e_ok() {
        let b = card_bill(vec![compras(), seguro()]);
        let out = car01(&b, &ans(&[("CAR-01:i3", Answer::No)]));
        assert_eq!(
            sev_refs(&out),
            vec![(Severity::Unlawful, refs(&["i3"]), 990)]
        );
        let out = car01(&b, &ans(&[("CAR-01:i3", Answer::Yes)]));
        assert_eq!(sev_refs(&out), vec![(Severity::Ok, refs(&["i3"]), 990)]);
        let out = car01(&b, &ans(&[("CAR-01:i3", Answer::Date(date(2026, 1, 1)))]));
        assert_eq!(out.findings[0].severity, Severity::Suspicious);
    }

    #[test]
    fn car01_reconhece_servico_por_palavra_mesmo_como_tarifa_ou_outros() {
        for desc in [
            "Proteção Premiada",
            "AVISO SMS",
            "Mensagem de compra por SMS",
            "Assistência viagem",
            "Clube Aurora Pontos",
            "SEGURO FATURA",
            "Título de capitalização",
        ] {
            for cat in [C::Outros, C::Tarifa] {
                let b = card_bill(vec![compras(), item("i5", desc, 590, cat)]);
                assert_eq!(
                    sev_refs(&car01(&b, &Answers::new())),
                    vec![(Severity::Suspicious, refs(&["i5"]), 590)],
                    "{desc} {cat:?}"
                );
            }
        }
    }

    #[test]
    fn car01_ignora_compras_anuidade_tarifa_encargo_imposto_e_estorno() {
        let b = card_bill(vec![
            compras(),
            anuidade(),
            manutencao(),
            item("i5", "Multa atraso seguro", 300, C::EncargoAtraso),
            item("i6", "IOF seguro viagem", 30, C::Imposto),
            item("i7", "Estorno seguro", -990, C::ServicoAdicional),
            item("i8", "Anuidade com proteção", 3_200, C::Tarifa),
        ]);
        let out = car01(&b, &Answers::new());
        assert!(out.findings.is_empty(), "{:?}", out.findings);
        assert!(out.questions.is_empty());
    }

    // ---------- CAR-02: tarifa fora da lista ----------

    #[test]
    fn car02_tarifa_fora_da_lista_e_suspeita_por_desenho() {
        let b = card_bill(vec![compras(), anuidade(), manutencao()]);
        for answers in [Answers::new(), ans(&[("CAR-02:i4", Answer::No)])] {
            let out = car02(&b, &answers);
            assert_eq!(
                sev_refs(&out),
                vec![(Severity::Suspicious, refs(&["i4"]), 890)]
            );
            assert!(out.questions.is_empty());
        }
    }

    #[test]
    fn car02_aceita_as_tarifas_previstas_para_o_cartao() {
        let b = card_bill(vec![
            compras(),
            anuidade(),
            item("i3", "ANUIDADE DIFERENCIADA", 4_000, C::Tarifa),
            item("i4", "2ª via do cartão", 1_500, C::Tarifa),
            item("i5", "Segunda via cartão", 1_500, C::Tarifa),
            item("i6", "Tarifa saque crédito", 1_200, C::Tarifa),
            item("i7", "RETIRADA-País", 1_200, C::Tarifa),
            item("i8", "Pagamento de contas", 500, C::Tarifa),
            item("i9", "AVAL.EMERG.CRÉDITO", 2_000, C::Tarifa),
            item("i10", "Avaliação emergencial de crédito", 2_000, C::Tarifa),
            item("i11", "Plástico personalizado", 1_000, C::Tarifa),
        ]);
        let out = car02(&b, &Answers::new());
        assert!(out.findings.is_empty(), "{:?}", out.findings);
    }

    #[test]
    fn car02_ignora_servico_da_car01_outros_itens_e_valor_nao_positivo() {
        let b = card_bill(vec![
            compras(),
            seguro(),
            item("i5", "Tarifa aviso SMS", 590, C::Tarifa),
            item("i6", "Estorno tarifa manutenção", -890, C::Tarifa),
            item("i7", "Juros rotativo", 5_000, C::EncargoAtraso),
            item("i8", "IOF", 120, C::Imposto),
        ]);
        let out = car02(&b, &Answers::new());
        assert!(out.findings.is_empty(), "{:?}", out.findings);
    }

    // ---------- CAR-03: anuidade x oferta ----------

    #[test]
    fn car03_sem_resposta_so_pergunta() {
        let b = card_bill(vec![compras(), anuidade()]);
        let out = car03(&b, &Answers::new());
        assert!(out.findings.is_empty());
        assert_eq!(out.questions.len(), 1);
        let q = &out.questions[0];
        assert_eq!(q.id, "CAR-03:anuidade");
        assert_eq!(q.rule_id, "CAR-03");
        assert_eq!(q.item_id, None);
        assert_eq!(q.text, "O cartão foi oferecido a você sem anuidade?");
        assert_eq!(q.kind, QuestionKind::YesNo);
    }

    #[test]
    fn car03_oferta_sem_anuidade_e_suspeita_e_sem_promessa_e_ok() {
        let b = card_bill(vec![
            compras(),
            anuidade(),
            item("i5", "Anuidade adicional", 1_000, C::Tarifa),
        ]);
        let out = car03(&b, &ans(&[("CAR-03:anuidade", Answer::Yes)]));
        assert_eq!(
            sev_refs(&out),
            vec![(Severity::Suspicious, refs(&["i2", "i5"]), 4_200)]
        );
        let out = car03(&b, &ans(&[("CAR-03:anuidade", Answer::No)]));
        assert_eq!(
            sev_refs(&out),
            vec![(Severity::Ok, refs(&["i2", "i5"]), 4_200)]
        );
    }

    #[test]
    fn car03_sem_anuidade_nao_pergunta() {
        let b = card_bill(vec![
            compras(),
            seguro(),
            item("i5", "Estorno anuidade", -3_200, C::Tarifa),
        ]);
        let out = car03(&b, &ans(&[("CAR-03:anuidade", Answer::Yes)]));
        assert!(out.findings.is_empty());
        assert!(out.questions.is_empty());
    }

    // ---------- CAR-04: multa acima de 2% ----------

    fn com_multa(fine: i64, base: i64) -> ExtractedBill {
        let mut b = card_bill(vec![
            compras(),
            item("i5", "Multa por atraso", fine, C::EncargoAtraso),
            item("i6", "Juros de mora", 283, C::EncargoAtraso),
        ]);
        b.late_fees = fees(base, fine, 283, 10);
        b
    }

    #[test]
    fn car04_multa_de_4_por_cento_excede_o_teto_de_2() {
        let out = car04(&com_multa(3_400, 85_000), &Answers::new());
        assert_eq!(
            sev_refs(&out),
            vec![(Severity::Unlawful, refs(&["i5"]), 1_700)]
        );
    }

    #[test]
    fn car04_multa_de_2_por_cento_ou_com_folga_de_arredondamento_passa() {
        assert!(
            car04(&com_multa(1_700, 85_000), &Answers::new())
                .findings
                .is_empty()
        );
        assert!(
            car04(&com_multa(1_701, 85_000), &Answers::new())
                .findings
                .is_empty()
        );
        assert_eq!(
            car04(&com_multa(1_702, 85_000), &Answers::new())
                .findings
                .len(),
            1
        );
    }

    #[test]
    fn car04_sem_encargos_ou_base_zero_nao_acusa() {
        let b = card_bill(vec![compras()]);
        assert!(car04(&b, &Answers::new()).findings.is_empty());
        assert!(
            car04(&com_multa(3_400, 0), &Answers::new())
                .findings
                .is_empty()
        );
    }

    // ---------- CAR-05: soma ----------

    #[test]
    fn car05_soma_diferente_do_total_e_suspeita() {
        let mut b = card_bill(vec![compras(), seguro()]);
        assert!(car05(&b, &Answers::new()).findings.is_empty());
        b.total_cents += 1_000;
        assert_eq!(
            sev_refs(&car05(&b, &Answers::new())),
            vec![(Severity::Suspicious, vec![], 1_000)]
        );
    }
}
