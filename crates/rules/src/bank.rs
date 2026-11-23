//! Regras do extrato de conta corrente BAN-01 a BAN-03 (docs/03, seção Banco).
//!
//! O extrato extraído só traz o que o banco cobrou no período (tarifas, pacote,
//! seguros e outros produtos): as movimentações da pessoa não são extraídas
//! (prompt do `crates/extract`). Cada função devolve a severidade **pretendida**;
//! o rebaixamento de regras "a validar" é feito pelo motor (`crate::cap`).
//!
//! Cada item cai em no máximo uma regra, nesta ordem: pacote (BAN-01), produto
//! (BAN-03) e tarifa de serviço essencial (BAN-02).

use humanobill_core::{
    Answer, Answers, BillItem, ExtractedBill, ItemCategory, Question, QuestionKind, Severity,
};

use crate::Output;
use crate::shared::{Asked, ask_per_item, draft, has_any_word};
use crate::text::fold;

/// Palavras de pacote ou cesta de serviços (BAN-01).
const PACKAGE_KEYWORDS: &[&str] = &["pacote", "cesta"];

/// Palavras de produto que o banco vende e debita na conta (BAN-03).
const PRODUCT_KEYWORDS: &[&str] = &[
    "seguro",
    "capitalizacao",
    "clube",
    "assistencia",
    "protecao",
];

/// Canal presencial: por opção do cliente, pode ser cobrado desde o primeiro
/// evento (Res. CMN 3.919, art. 2º, § 1º, I). Inclui as siglas da Tabela I
/// ("SAQUEpessoal", "EXTRATOmês(P)").
const IN_PERSON_MARKERS: &[&str] = &["pessoal", "guiche", "presencial", "(p)"];

/// Saque em rede de outra empresa: a norma fala só em "terminal de
/// autoatendimento", sem ressalva. Sem fonte oficial, fica no máximo suspeito.
const SHARED_NETWORK_WORDS: &[&str] = &["rede", "24h", "24horas", "banco24horas"];

fn is_shared_network(item: &BillItem) -> bool {
    has_any_word(item, SHARED_NETWORK_WORDS) || folded(item).contains("24 horas")
}

/// Serviço essencial com cota grátis por mês (Res. CMN 3.919, art. 2º, I).
struct Essential {
    /// Sufixo do id da pergunta ("BAN-02:saque").
    key: &'static str,
    /// Trechos (sem acento, minúsculos) que identificam o serviço na descrição.
    markers: &'static [&'static str],
    question: &'static str,
    /// Explicação sem resposta.
    open: &'static str,
    /// Explicação quando a pessoa não passou da cota.
    free: &'static str,
    /// Explicação quando a pessoa passou da cota.
    paid: &'static str,
}

const ESSENTIALS: &[Essential] = &[
    Essential {
        key: "extrato",
        markers: &["extrato"],
        question: "Neste mês, você pediu mais de 2 extratos?",
        open: "Até 2 extratos por mês são grátis. Se você não passou disso, esta tarifa não \
               deveria ser cobrada.",
        free: "Você não passou de 2 extratos no mês. Eles são grátis pela regra dos serviços \
               essenciais da conta.",
        paid: "Você passou de 2 extratos no mês. A partir do terceiro, o banco pode cobrar.",
    },
    Essential {
        key: "cheque",
        markers: &["folha", "talao", "talonario"],
        question: "Neste mês, você pediu mais de 10 folhas de cheque?",
        open: "Até 10 folhas de cheque por mês são grátis. Se você não passou disso, esta \
               tarifa não deveria ser cobrada.",
        free: "Você não passou de 10 folhas de cheque no mês. Elas são grátis pela regra dos \
               serviços essenciais da conta.",
        paid: "Você passou de 10 folhas de cheque no mês. A partir daí, o banco pode cobrar.",
    },
    Essential {
        key: "saque",
        markers: &["saque"],
        question: "Neste mês, você fez mais de 4 saques nos caixas eletrônicos do banco?",
        open: "Até 4 saques por mês nos caixas eletrônicos são grátis. Se você não passou \
               disso, esta tarifa não deveria ser cobrada.",
        free: "Você não passou de 4 saques no mês. Eles são grátis pela regra dos serviços \
               essenciais da conta.",
        paid: "Você passou de 4 saques no mês. A partir do quinto, o banco pode cobrar.",
    },
];

/// Texto do item (descrição + trecho original) sem acento e em minúsculas.
fn folded(item: &BillItem) -> String {
    fold(&format!("{} {}", item.description, item.original_text))
}

/// Itens que o banco cobra e que as regras podem acusar: valor positivo e
/// categoria de cobrança (impostos e encargos de limite ficam de fora).
fn is_charge(item: &BillItem) -> bool {
    item.amount_cents > 0
        && matches!(
            item.category,
            ItemCategory::Tarifa
                | ItemCategory::ServicoAdicional
                | ItemCategory::Plano
                | ItemCategory::Outros
        )
}

fn is_package(item: &BillItem) -> bool {
    is_charge(item)
        && item.category != ItemCategory::ServicoAdicional
        && has_any_word(item, PACKAGE_KEYWORDS)
}

fn is_product(item: &BillItem) -> bool {
    is_charge(item)
        && !is_package(item)
        && (item.category == ItemCategory::ServicoAdicional || has_any_word(item, PRODUCT_KEYWORDS))
}

/// Serviço essencial cobrado por este item (tarifa avulsa, fora do canal presencial).
fn essential_of(item: &BillItem) -> Option<&'static Essential> {
    if !is_charge(item) || is_package(item) || is_product(item) {
        return None;
    }
    let text = folded(item);
    if IN_PERSON_MARKERS.iter().any(|m| text.contains(m)) {
        return None;
    }
    ESSENTIALS
        .iter()
        .find(|e| e.markers.iter().any(|m| text.contains(m)))
}

/// BAN-01: pacote ou cesta de serviços cobrado. Sem resposta: suspeito.
/// "Não contratei": indevido (pretendido). "Contratei": ok.
pub(crate) fn ban01(bill: &ExtractedBill, answers: &Answers) -> Output {
    let mut out = Output::default();
    for item in bill.items.iter().filter(|i| is_package(i)) {
        let d = &item.description;
        ask_per_item(
            &mut out,
            item,
            answers,
            Asked {
                rule_id: "BAN-01",
                question: format!("Você assinou contrato para {d}?"),
                open: (
                    format!("Pacote de tarifas: {d}"),
                    "O banco só cobra pacote com contrato. Sem pacote, você paga só o que usar, \
                     e os serviços essenciais são grátis.",
                ),
                no: (
                    format!("Pacote não contratado: {d}"),
                    "Você disse que não contratou este pacote. Cobrança sem contrato pode ser \
                     contestada e o valor, devolvido.",
                ),
                yes: (
                    format!("Pacote confirmado: {d}"),
                    "Você confirmou que contratou este pacote. Nada a contestar aqui.",
                ),
            },
        );
    }
    out
}

/// BAN-02: tarifa de serviço essencial com cota grátis no mês (extrato, folhas de
/// cheque, saque). Uma pergunta por serviço; um achado por item.
/// "Não passei da cota": indevido (pretendido). "Passei": ok. Saque em rede de
/// outra empresa fica no máximo suspeito.
pub(crate) fn ban02(bill: &ExtractedBill, answers: &Answers) -> Output {
    let mut out = Output::default();
    for e in ESSENTIALS {
        let items: Vec<&BillItem> = bill
            .items
            .iter()
            .filter(|i| essential_of(i).is_some_and(|x| x.key == e.key))
            .collect();
        if items.is_empty() {
            continue;
        }
        let qid = format!("BAN-02:{}", e.key);
        out.questions.push(Question {
            id: qid.clone(),
            rule_id: "BAN-02".into(),
            item_id: None,
            text: e.question.into(),
            kind: QuestionKind::YesNo,
        });
        let answer = answers.get(&qid);
        for item in items {
            let d = &item.description;
            let shared = e.key == "saque" && is_shared_network(item);
            let (severity, title, explanation) = match answer {
                Some(Answer::Yes) => (
                    Severity::Ok,
                    format!("Tarifa acima da cota grátis: {d}"),
                    e.paid,
                ),
                Some(Answer::No) if shared => (
                    Severity::Suspicious,
                    format!("Tarifa de saque em outra rede: {d}"),
                    "Saque em caixa eletrônico de outra rede pode ter regra diferente. Pergunte \
                     ao banco se este saque entra na cota grátis.",
                ),
                Some(Answer::No) => (
                    Severity::Unlawful,
                    format!("Tarifa de serviço grátis: {d}"),
                    e.free,
                ),
                _ => (
                    Severity::Suspicious,
                    format!("Tarifa que pode ser grátis: {d}"),
                    e.open,
                ),
            };
            out.findings.push(draft(
                severity,
                title,
                explanation.to_owned(),
                &[item],
                item.amount_cents,
            ));
        }
    }
    out
}

/// BAN-03: seguro, título de capitalização ou outro produto debitado na conta.
/// Sem resposta: suspeito. "Não contratei": indevido (pretendido). "Contratei": ok.
pub(crate) fn ban03(bill: &ExtractedBill, answers: &Answers) -> Output {
    let mut out = Output::default();
    for item in bill.items.iter().filter(|i| is_product(i)) {
        let d = &item.description;
        ask_per_item(
            &mut out,
            item,
            answers,
            Asked {
                rule_id: "BAN-03",
                question: format!("Você contratou {d}?"),
                open: (
                    format!("Produto debitado na conta: {d}"),
                    "Seguro ou outro produto só pode ser cobrado se você pediu. Se não pediu, \
                     pode contestar e pedir o dinheiro de volta.",
                ),
                no: (
                    format!("Produto não contratado: {d}"),
                    "Você disse que não contratou este produto. Cobrança sem pedido não precisa \
                     ser paga e o valor pode ser devolvido.",
                ),
                yes: (
                    format!("Produto confirmado: {d}"),
                    "Você confirmou que contratou este produto. Nada a contestar aqui.",
                ),
            },
        );
    }
    out
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

    fn cesta() -> BillItem {
        item("i1", "Cesta Aurora Mais", 3_990, C::Tarifa)
    }

    fn saque() -> BillItem {
        item("i3", "Tarifa saque terminal", 650, C::Tarifa)
    }

    fn seguro() -> BillItem {
        item("i2", "Seguro Aurora Proteção", 1_490, C::ServicoAdicional)
    }

    // ---------- BAN-01: pacote de serviços ----------

    #[test]
    fn ban01_cesta_sem_resposta_e_suspeita_com_pergunta() {
        let b = bank_bill(vec![cesta(), saque()]);
        let out = ban01(&b, &Answers::new());
        assert_eq!(
            sev_refs(&out),
            vec![(Severity::Suspicious, refs(&["i1"]), 3_990)]
        );
        assert_eq!(out.questions.len(), 1);
        let q = &out.questions[0];
        assert_eq!(q.id, "BAN-01:i1");
        assert_eq!(q.rule_id, "BAN-01");
        assert_eq!(q.item_id.as_deref(), Some("i1"));
        assert_eq!(q.text, "Você assinou contrato para Cesta Aurora Mais?");
        assert_eq!(q.kind, QuestionKind::YesNo);
        let d = &out.findings[0];
        assert_eq!(d.original_text.as_deref(), Some("CESTA AURORA MAIS 3990"));
        assert_eq!(d.page, Some(1));
    }

    #[test]
    fn ban01_nao_contratei_pretende_indevido_e_contratei_e_ok() {
        let b = bank_bill(vec![cesta()]);
        let out = ban01(&b, &ans(&[("BAN-01:i1", Answer::No)]));
        assert_eq!(
            sev_refs(&out),
            vec![(Severity::Unlawful, refs(&["i1"]), 3_990)]
        );
        let out = ban01(&b, &ans(&[("BAN-01:i1", Answer::Yes)]));
        assert_eq!(sev_refs(&out), vec![(Severity::Ok, refs(&["i1"]), 3_990)]);
        // Resposta de tipo errado conta como sem resposta.
        let out = ban01(&b, &ans(&[("BAN-01:i1", Answer::Amount(1))]));
        assert_eq!(out.findings[0].severity, Severity::Suspicious);
    }

    #[test]
    fn ban01_reconhece_pacote_por_palavra_em_qualquer_categoria_de_tarifa() {
        for (desc, cat) in [
            ("PACOTE PADRONIZADO I", C::Tarifa),
            ("Pacotes de serviços", C::Outros),
            ("TAR CESTA SERVICOS", C::Tarifa),
            ("Cesta Exclusiva", C::Plano),
        ] {
            let b = bank_bill(vec![item("i1", desc, 2_500, cat)]);
            let out = ban01(&b, &Answers::new());
            assert_eq!(
                sev_refs(&out),
                vec![(Severity::Suspicious, refs(&["i1"]), 2_500)],
                "{desc}"
            );
        }
    }

    #[test]
    fn ban01_ignora_o_que_nao_e_pacote_e_valor_zero_ou_negativo() {
        let b = bank_bill(vec![
            saque(),
            seguro(),
            item("i4", "Estorno cesta de serviços", -3_990, C::Tarifa),
            item("i5", "Nenhuma tarifa no período", 0, C::Tarifa),
            item("i6", "IOF cesta", 100, C::Imposto),
        ]);
        let out = ban01(&b, &Answers::new());
        assert!(out.findings.is_empty(), "{:?}", out.findings);
        assert!(out.questions.is_empty());
    }

    // ---------- BAN-02: tarifa de serviço essencial ----------

    #[test]
    fn ban02_tarifa_de_saque_sem_resposta_e_suspeita_com_pergunta_por_servico() {
        let b = bank_bill(vec![cesta(), saque()]);
        let out = ban02(&b, &Answers::new());
        assert_eq!(
            sev_refs(&out),
            vec![(Severity::Suspicious, refs(&["i3"]), 650)]
        );
        assert_eq!(out.questions.len(), 1);
        let q = &out.questions[0];
        assert_eq!(q.id, "BAN-02:saque");
        assert_eq!(q.rule_id, "BAN-02");
        assert_eq!(q.item_id, None);
        assert_eq!(
            q.text,
            "Neste mês, você fez mais de 4 saques nos caixas eletrônicos do banco?"
        );
        assert_eq!(q.kind, QuestionKind::YesNo);
    }

    #[test]
    fn ban02_ate_4_saques_pretende_indevido_e_mais_de_4_e_ok() {
        let b = bank_bill(vec![saque()]);
        let out = ban02(&b, &ans(&[("BAN-02:saque", Answer::No)]));
        assert_eq!(
            sev_refs(&out),
            vec![(Severity::Unlawful, refs(&["i3"]), 650)]
        );
        let out = ban02(&b, &ans(&[("BAN-02:saque", Answer::Yes)]));
        assert_eq!(sev_refs(&out), vec![(Severity::Ok, refs(&["i3"]), 650)]);
    }

    #[test]
    fn ban02_cada_servico_tem_sua_cota_e_sua_pergunta() {
        let b = bank_bill(vec![
            item("i1", "Tarifa extrato mensal", 300, C::Tarifa),
            item("i2", "TAR FORNEC FOLHAS CHEQUE", 1_000, C::Tarifa),
            item("i3", "SAQUEterminal", 650, C::Tarifa),
            item("i4", "Tarifa saque terminal", 650, C::Tarifa),
        ]);
        let out = ban02(&b, &ans(&[("BAN-02:extrato", Answer::No)]));
        let perguntas: Vec<(&str, &str)> = out
            .questions
            .iter()
            .map(|q| (q.id.as_str(), q.text.as_str()))
            .collect();
        assert_eq!(
            perguntas,
            vec![
                (
                    "BAN-02:extrato",
                    "Neste mês, você pediu mais de 2 extratos?"
                ),
                (
                    "BAN-02:cheque",
                    "Neste mês, você pediu mais de 10 folhas de cheque?"
                ),
                (
                    "BAN-02:saque",
                    "Neste mês, você fez mais de 4 saques nos caixas eletrônicos do banco?"
                ),
            ]
        );
        // Um achado por item; a resposta vale para todos os itens do mesmo serviço.
        assert_eq!(
            sev_refs(&out),
            vec![
                (Severity::Unlawful, refs(&["i1"]), 300),
                (Severity::Suspicious, refs(&["i2"]), 1_000),
                (Severity::Suspicious, refs(&["i3"]), 650),
                (Severity::Suspicious, refs(&["i4"]), 650),
            ]
        );
    }

    /// Res. CMN 3.919, art. 2º, § 1º, I: no caixa presencial, por opção do cliente,
    /// o banco pode cobrar desde o primeiro saque ou extrato.
    #[test]
    fn ban02_canal_presencial_nao_entra_na_cota_gratis() {
        let b = bank_bill(vec![
            item("i1", "SAQUEpessoal", 900, C::Tarifa),
            item("i2", "Tarifa saque no guichê", 900, C::Tarifa),
            item("i3", "EXTRATOmês(P)", 400, C::Tarifa),
            item("i4", "Extrato presencial", 400, C::Tarifa),
        ]);
        let out = ban02(&b, &ans(&[("BAN-02:saque", Answer::No)]));
        assert!(out.findings.is_empty(), "{:?}", out.findings);
        assert!(out.questions.is_empty());
        // O extrato pelo terminal ("(E)") continua na cota.
        let b = bank_bill(vec![item("i1", "EXTRATOmês(E)", 400, C::Tarifa)]);
        assert_eq!(ban02(&b, &Answers::new()).findings.len(), 1);
    }

    /// A norma não diz se o saque em rede de outra empresa (24 horas) entra na
    /// cota: mesmo com "não passei de 4", fica suspeito.
    #[test]
    fn ban02_saque_em_rede_compartilhada_fica_no_maximo_suspeito() {
        for desc in [
            "Tarifa saque 24h",
            "TARIFA SAQUE REDE BANCO24HORAS",
            "Saque Banco 24 Horas",
        ] {
            let b = bank_bill(vec![item("i1", desc, 650, C::Tarifa)]);
            let out = ban02(&b, &ans(&[("BAN-02:saque", Answer::No)]));
            assert_eq!(
                sev_refs(&out),
                vec![(Severity::Suspicious, refs(&["i1"]), 650)],
                "{desc}"
            );
            let out = ban02(&b, &ans(&[("BAN-02:saque", Answer::Yes)]));
            assert_eq!(out.findings[0].severity, Severity::Ok, "{desc}");
        }
    }

    #[test]
    fn ban02_ignora_pacote_produto_encargo_e_o_que_nao_e_essencial() {
        let b = bank_bill(vec![
            item("i1", "Cesta com saques ilimitados", 3_990, C::Tarifa),
            item("i2", "Seguro saque protegido", 500, C::ServicoAdicional),
            item("i3", "Tarifa TED outra instituição", 1_200, C::Tarifa),
            item("i4", "Juros saque cheque especial", 800, C::EncargoAtraso),
            item("i5", "IOF", 50, C::Imposto),
            item("i6", "Nenhuma tarifa no período", 0, C::Tarifa),
            item("i7", "Estorno tarifa saque", -650, C::Tarifa),
        ]);
        let out = ban02(&b, &Answers::new());
        assert!(out.findings.is_empty(), "{:?}", out.findings);
        assert!(out.questions.is_empty());
    }

    // ---------- BAN-03: produto debitado na conta ----------

    #[test]
    fn ban03_seguro_sem_resposta_e_suspeito_com_pergunta() {
        let b = bank_bill(vec![cesta(), seguro(), saque()]);
        let out = ban03(&b, &Answers::new());
        assert_eq!(
            sev_refs(&out),
            vec![(Severity::Suspicious, refs(&["i2"]), 1_490)]
        );
        let q = &out.questions[0];
        assert_eq!(q.id, "BAN-03:i2");
        assert_eq!(q.item_id.as_deref(), Some("i2"));
        assert_eq!(q.text, "Você contratou Seguro Aurora Proteção?");
    }

    #[test]
    fn ban03_nao_contratei_pretende_indevido_e_contratei_e_ok() {
        let b = bank_bill(vec![seguro()]);
        let out = ban03(&b, &ans(&[("BAN-03:i2", Answer::No)]));
        assert_eq!(
            sev_refs(&out),
            vec![(Severity::Unlawful, refs(&["i2"]), 1_490)]
        );
        let out = ban03(&b, &ans(&[("BAN-03:i2", Answer::Yes)]));
        assert_eq!(sev_refs(&out), vec![(Severity::Ok, refs(&["i2"]), 1_490)]);
    }

    #[test]
    fn ban03_reconhece_produto_por_palavra_mesmo_como_tarifa_ou_outros() {
        for desc in [
            "Título de capitalização Aurora",
            "CAPITALIZACAO MENSAL",
            "Clube de vantagens",
            "Assistência residencial",
            "Proteção cartão de débito",
            "SEGUROS DIVERSOS",
        ] {
            let b = bank_bill(vec![item("i1", desc, 990, C::Outros)]);
            assert_eq!(
                sev_refs(&ban03(&b, &Answers::new())),
                vec![(Severity::Suspicious, refs(&["i1"]), 990)],
                "{desc}"
            );
            let b = bank_bill(vec![item("i1", desc, 990, C::Tarifa)]);
            assert_eq!(ban03(&b, &Answers::new()).findings.len(), 1, "{desc}");
        }
    }

    #[test]
    fn ban03_ignora_tarifa_comum_pacote_encargo_e_valor_nao_positivo() {
        let b = bank_bill(vec![
            cesta(),
            saque(),
            item("i4", "Juros cheque especial", 800, C::EncargoAtraso),
            item("i5", "IOF seguro", 30, C::Imposto),
            item("i6", "Estorno seguro", -1_490, C::ServicoAdicional),
            item("i7", "Nenhuma tarifa no período", 0, C::Tarifa),
        ]);
        let out = ban03(&b, &Answers::new());
        assert!(out.findings.is_empty(), "{:?}", out.findings);
        assert!(out.questions.is_empty());
    }

    // ---------- as três juntas: um item, uma regra ----------

    #[test]
    fn cada_item_cai_em_uma_regra_so() {
        let b = bank_bill(vec![
            item("i1", "Cesta com seguro e saques", 3_990, C::Tarifa),
            item("i2", "Seguro saque protegido", 500, C::ServicoAdicional),
            item("i3", "Tarifa saque", 650, C::Tarifa),
        ]);
        let n = Answers::new();
        let ids = |o: Output| -> Vec<String> {
            o.findings.into_iter().flat_map(|d| d.item_refs).collect()
        };
        assert_eq!(ids(ban01(&b, &n)), ["i1"]);
        assert_eq!(ids(ban03(&b, &n)), ["i2"]);
        assert_eq!(ids(ban02(&b, &n)), ["i3"]);
    }
}
