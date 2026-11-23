//! Contestação pronta (RF11) e roteiro "Onde reclamar" (RF12).
//!
//! Gerado por template determinístico, sem IA: o texto sempre cita o item,
//! o valor, o mês de referência e a base legal dos achados selecionados.

use humanobill_core::{
    BillKind, Cents, Dispute, DisputeCallStep, DisputeStep, ExtractedBill, Finding, Severity,
};

use crate::text::brl;

pub const DISCLAIMER: &str = "Conteúdo informativo, não é orientação jurídica.";

/// Erros de seleção da contestação.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DisputeError {
    /// Nenhum achado selecionado.
    Empty,
    /// Achado `ok` não pode ser contestado.
    NotDisputable(String),
}

/// Gera a contestação a partir dos achados selecionados (já resolvidos pelo chamador).
pub fn build(
    bill: &ExtractedBill,
    selected: &[&Finding],
    paid: Option<bool>,
    protocol: Option<&str>,
) -> Result<Dispute, DisputeError> {
    if selected.is_empty() {
        return Err(DisputeError::Empty);
    }
    if let Some(f) = selected.iter().find(|f| f.severity == Severity::Ok) {
        return Err(DisputeError::NotDisputable(f.id.clone()));
    }

    // Soma saturada: valores absurdos vindos de fora não podem dar panic (debug) nem
    // enrolar (release). Mesma política do `saturating_mul(2)` do dobro abaixo.
    let total_cents: Cents = selected
        .iter()
        .fold(0, |acc: Cents, f| acc.saturating_add(f.amount_cents));
    let month = month_pt(&bill.reference_month);
    let due = bill.due_date.format("%d/%m/%Y");

    let mut t = String::new();
    t.push_str(&match bill.kind {
        // O extrato não vence: cita o fim do período.
        BillKind::BankStatement => format!(
            "Olá. Sou cliente de {} e contesto cobranças do extrato da minha conta corrente de {} (período até {}).\n\n",
            bill.provider_name, month, due
        ),
        BillKind::CardBill => format!(
            "Olá. Sou cliente de {} e contesto cobranças da fatura do cartão de {} (vencimento em {}).\n\n",
            bill.provider_name, month, due
        ),
        BillKind::Telecom | BillKind::Energy => format!(
            "Olá. Sou cliente da {} e contesto cobranças da fatura de {} (vencimento em {}).\n\n",
            bill.provider_name, month, due
        ),
    });
    t.push_str("Itens contestados:\n");
    for f in selected {
        let item = item_label(bill, f);
        let bases: Vec<&str> = f.legal_basis.iter().map(|l| l.label.as_str()).collect();
        let base = if bases.is_empty() {
            String::new()
        } else {
            format!(" Base legal: {}.", bases.join("; "))
        };
        t.push_str(&format!(
            "- {}: {}. {}.{}\n",
            item,
            brl(f.amount_cents),
            f.title,
            base
        ));
    }
    t.push_str(&format!("\nTotal contestado: {}.\n\n", brl(total_cents)));

    let double_rule = double_rule(bill.kind);
    match paid {
        Some(true) => t.push_str(&format!(
            "Como a fatura já foi paga, peço a devolução em dobro do valor cobrado indevidamente ({}), com correção monetária e juros, conforme o art. 42 do Código de Defesa do Consumidor{double_rule}.\n",
            brl(total_cents.saturating_mul(2)),
        )),
        _ => match bill.kind {
            BillKind::Telecom => t.push_str(
                "Peço o cancelamento dessas cobranças e a emissão de uma nova fatura sem esses valores. Se a fatura já tiver sido paga, peço a devolução em dobro, conforme o art. 42 do Código de Defesa do Consumidor.\n",
            ),
            // Fatura em aberto: suspensão da parcela questionada (REN 1000 art. 413), sem dobro.
            BillKind::Energy => t.push_str(&format!(
                "Peço a suspensão imediata da parcela contestada, conforme o art. 413 da REN ANEEL 1.000/2021, o cancelamento dessas cobranças e a emissão de uma nova fatura sem esses valores. Se a fatura já tiver sido paga, peço a devolução em dobro, conforme o art. 42 do Código de Defesa do Consumidor{double_rule}.\n",
            )),
            // No extrato o valor já saiu da conta: estorno, e o dobro do CDC.
            BillKind::BankStatement => t.push_str(
                "Peço o cancelamento dessas cobranças e o estorno dos valores na minha conta. Como eles já foram debitados, peço a devolução em dobro, conforme o art. 42 do Código de Defesa do Consumidor.\n",
            ),
            BillKind::CardBill => t.push_str(
                "Peço o cancelamento dessas cobranças e o estorno dos valores na próxima fatura. Se a fatura já tiver sido paga, peço a devolução em dobro, conforme o art. 42 do Código de Defesa do Consumidor.\n",
            ),
        },
    }
    match bill.kind {
        BillKind::Telecom => t.push_str(
            "Lembro que a operadora tem até 30 dias para responder a esta contestação.\n",
        ),
        BillKind::Energy => t.push_str(
            "Lembro que a distribuidora tem até 5 dias úteis para resolver esta reclamação (10 dias úteis se precisar de visita técnica), conforme o art. 408 da REN ANEEL 1.000/2021.\n",
        ),
        BillKind::BankStatement | BillKind::CardBill => t.push_str(SAC_DEADLINE_TEXT),
    }
    match protocol.map(str::trim).filter(|p| !p.is_empty()) {
        Some(p) => t.push_str(&format!(
            "\nEste pedido complementa o atendimento de protocolo {p}. Peço um novo número de protocolo para esta contestação.\n"
        )),
        None => t.push_str("\nPeço o número de protocolo deste atendimento.\n"),
    }
    t.push_str("\nObrigado.");

    Ok(Dispute {
        text: t,
        total_cents,
        steps: steps(bill.kind),
        deadlines: deadlines(bill.kind),
        disclaimer: DISCLAIMER.into(),
    })
}

/// Banco e cartão: pede resposta em 7 dias corridos, sem citar decreto. O Decreto
/// 13.135/2026 (SAC) dá esse prazo, mas não está claro se vale para todo banco
/// (docs/03, Banco: "a validar").
const SAC_DEADLINE_TEXT: &str = "Peço uma resposta em até 7 dias corridos.\n";

/// Norma setorial do dobro: RGC art. 64 (telecom) ou REN 1000 art. 323 (energia).
/// Banco e cartão: só o CDC art. 42 (sem norma setorial do dobro).
fn double_rule(kind: BillKind) -> &'static str {
    match kind {
        BillKind::Telecom => {
            " e o art. 64 do Regulamento Geral de Direitos do Consumidor de Telecomunicações (Anatel)"
        }
        BillKind::Energy => " e o art. 323 da REN ANEEL 1.000/2021",
        BillKind::BankStatement | BillKind::CardBill => "",
    }
}

/// Tamanho máximo do protocolo anotado (os de operadora têm 13 a 20 dígitos).
pub const PROTOCOL_MAX_LEN: usize = 30;

/// Protocolo aceito: vazio (= não informado) ou só dígitos e hífen, até
/// `PROTOCOL_MAX_LEN`. Quem chama deve passar o texto já sem espaços nas pontas.
pub fn protocol_is_valid(p: &str) -> bool {
    p.len() <= PROTOCOL_MAX_LEN && p.chars().all(|c| c.is_ascii_digit() || c == '-')
}

/// Roteiro de ligação ao SAC (RF17), em texto fixo e sem IA: apresentar-se, citar o
/// mês, citar cada item e o valor, pedir o estorno e pedir o protocolo.
/// A seleção já deve ter sido validada por [`build`].
pub fn call_script(
    bill: &ExtractedBill,
    selected: &[&Finding],
    paid: Option<bool>,
) -> Vec<DisputeCallStep> {
    let step = |title: &str, say: String| DisputeCallStep {
        title: title.into(),
        say,
    };
    let total: Cents = selected
        .iter()
        .fold(0, |acc: Cents, f| acc.saturating_add(f.amount_cents));
    let month = month_pt(&bill.reference_month);
    let due = bill.due_date.format("%d/%m/%Y");

    let (cliente, onde) = match bill.kind {
        BillKind::BankStatement => ("cliente de", "no extrato da minha conta"),
        BillKind::CardBill => ("cliente de", "na fatura do meu cartão"),
        BillKind::Telecom | BillKind::Energy => ("cliente da", "na minha fatura"),
    };
    let mut roteiro = vec![
        step(
            "Apresente-se",
            format!(
                "Olá. Sou {cliente} {} e estou ligando para contestar cobranças {onde}. Quando o atendente pedir, informe seus dados para ele achar o cadastro.",
                bill.provider_name
            ),
        ),
        match bill.kind {
            BillKind::BankStatement => step(
                "Diga o mês do extrato",
                format!("Quero falar das tarifas do extrato de {month}, com período até {due}."),
            ),
            _ => step(
                "Diga o mês da fatura",
                format!("Quero falar da fatura de {month}, com vencimento em {due}."),
            ),
        },
    ];

    let n = selected.len();
    for (i, f) in selected.iter().enumerate() {
        let bases: Vec<&str> = f.legal_basis.iter().map(|l| l.label.as_str()).collect();
        let base = if bases.is_empty() {
            String::new()
        } else {
            format!(" Base legal: {}.", bases.join("; "))
        };
        roteiro.push(step(
            &format!("Cite o item {} de {n}", i + 1),
            format!(
                "Tem uma cobrança de \"{}\", no valor de {}. {}.{}",
                item_label(bill, f),
                brl(f.amount_cents),
                f.title,
                base
            ),
        ));
    }

    let estorno = match (paid, bill.kind) {
        (Some(true), kind) => format!(
            "Como a fatura já foi paga, peço a devolução em dobro do valor cobrado indevidamente ({}), conforme o art. 42 do Código de Defesa do Consumidor{}.",
            brl(total.saturating_mul(2)),
            double_rule(kind)
        ),
        (_, BillKind::Telecom) => format!(
            "O total contestado é {}. Peço o cancelamento dessas cobranças e uma nova fatura sem esses valores. Se eu já tiver pago, peço a devolução em dobro.",
            brl(total)
        ),
        (_, BillKind::Energy) => format!(
            "O total contestado é {}. Peço a suspensão imediata da parcela contestada (art. 413 da REN ANEEL 1.000/2021), o cancelamento dessas cobranças e uma nova fatura sem esses valores. Se eu já tiver pago, peço a devolução em dobro.",
            brl(total)
        ),
        (_, BillKind::BankStatement) => format!(
            "O total contestado é {}. Peço o cancelamento dessas cobranças e o estorno na minha conta. Como o valor já foi debitado, peço a devolução em dobro.",
            brl(total)
        ),
        (_, BillKind::CardBill) => format!(
            "O total contestado é {}. Peço o cancelamento dessas cobranças e o estorno na próxima fatura. Se eu já tiver pago, peço a devolução em dobro.",
            brl(total)
        ),
    };
    roteiro.push(step("Peça o estorno", estorno));
    roteiro.push(step(
        "Peça o número do protocolo",
        "Por favor, me informe o número do protocolo deste atendimento e em quanto tempo vou receber a resposta. Anote o número no campo abaixo.".into(),
    ));
    roteiro
}

fn item_label(bill: &ExtractedBill, f: &Finding) -> String {
    let names: Vec<&str> = f
        .item_refs
        .iter()
        .filter_map(|id| bill.items.iter().find(|i| &i.id == id))
        .map(|i| i.description.as_str())
        .collect();
    if names.is_empty() {
        f.title.clone()
    } else {
        names.join(" + ")
    }
}

/// Roteiro de onde reclamar, na ordem (RF12).
pub fn steps(kind: BillKind) -> Vec<DisputeStep> {
    let s = |title: &str, description: &str, url: Option<&str>| DisputeStep {
        title: title.into(),
        description: description.into(),
        url: url.map(Into::into),
    };
    match kind {
        BillKind::Telecom => vec![
            s(
                "SAC da operadora",
                "Ligue ou use o chat da operadora, envie o texto e anote o número de protocolo.",
                None,
            ),
            s(
                "Anatel",
                "Sem resposta em 30 dias, ou com resposta ruim, registre uma reclamação na Anatel com o protocolo.",
                Some("https://www.gov.br/anatel/pt-br/consumidor/reclamacoes"),
            ),
            s(
                "consumidor.gov.br",
                "Se ainda não resolver, registre no consumidor.gov.br. A empresa costuma responder em até 10 dias.",
                Some("https://www.consumidor.gov.br"),
            ),
        ],
        BillKind::Energy => vec![
            s(
                "Distribuidora",
                "Fale com a distribuidora de energia, envie o texto e anote o número de protocolo. Ela tem até 5 dias úteis para resolver.",
                None,
            ),
            s(
                "Ouvidoria",
                "Sem solução no prazo, procure a ouvidoria da distribuidora com o protocolo do primeiro contato. Ela responde em até 10 dias úteis.",
                None,
            ),
            s(
                "ANEEL",
                "Se a ouvidoria não resolver, registre uma reclamação na ANEEL: telefone 167, app ANEEL Consumidor ou site.",
                Some("https://www.gov.br/aneel/pt-br/consumidores/como-registrar-reclamacao"),
            ),
            s(
                "consumidor.gov.br",
                "Você também pode registrar no consumidor.gov.br.",
                Some("https://www.consumidor.gov.br"),
            ),
        ],
        BillKind::BankStatement | BillKind::CardBill => {
            let sac = if kind == BillKind::BankStatement {
                "SAC do banco"
            } else {
                "SAC do cartão"
            };
            vec![
                s(
                    sac,
                    "Fale pelo app, chat ou telefone, envie o texto e anote o número de protocolo. Peça resposta em até 7 dias.",
                    None,
                ),
                s(
                    "Ouvidoria",
                    "Sem solução no SAC, procure a ouvidoria da instituição com o protocolo. Ela tem até 10 dias úteis para responder.",
                    None,
                ),
                s(
                    "Banco Central",
                    "Se ainda não resolver, registre uma reclamação no Banco Central com os protocolos (precisa de conta gov.br prata ou ouro). No Registrato, também do Banco Central, você vê suas contas e dívidas.",
                    Some(BCB_RECLAMACAO_URL),
                ),
                s(
                    "consumidor.gov.br",
                    "Você também pode registrar no consumidor.gov.br. A empresa costuma responder em até 10 dias.",
                    Some("https://www.consumidor.gov.br"),
                ),
            ]
        }
    }
}

/// Página oficial do Banco Central para registrar reclamação contra banco ou cartão.
pub const BCB_RECLAMACAO_URL: &str = "https://www.bcb.gov.br/acessoinformacao/registrar_reclamacao";

/// Prazos exibidos junto do roteiro.
pub fn deadlines(kind: BillKind) -> Vec<String> {
    match kind {
        BillKind::Telecom => vec![
            "A operadora tem 30 dias para responder".into(),
            "Você pode contestar cobranças de até 3 anos".into(),
        ],
        BillKind::Energy => vec![
            "A distribuidora tem 5 dias úteis para resolver (10 com visita técnica)".into(),
            "A ouvidoria tem 10 dias úteis para responder".into(),
            "Guarde o número de protocolo de cada contato".into(),
        ],
        BillKind::BankStatement | BillKind::CardBill => vec![
            "Peça a resposta do SAC em até 7 dias corridos".into(),
            "A ouvidoria tem até 10 dias úteis para responder".into(),
            "Na reclamação ao Banco Central, a instituição tem até 10 dias úteis".into(),
            "Guarde o número de protocolo de cada contato".into(),
        ],
    }
}

const MESES: [&str; 12] = [
    "janeiro",
    "fevereiro",
    "março",
    "abril",
    "maio",
    "junho",
    "julho",
    "agosto",
    "setembro",
    "outubro",
    "novembro",
    "dezembro",
];

/// "2026-10" -> "outubro de 2026" (ou o texto original, se inválido).
fn month_pt(ym: &str) -> String {
    let parsed = ym
        .split_once('-')
        .and_then(|(y, m)| Some((y, m.parse::<usize>().ok()?)))
        .filter(|(_, m)| (1..=12).contains(m));
    match parsed {
        Some((y, m)) => format!("{} de {}", MESES[m - 1], y),
        None => ym.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evaluate;
    use humanobill_core::{Answer, Answers};

    fn fixture(name: &str) -> ExtractedBill {
        let p = format!(
            "{}/../../fixtures/extracted/{name}.json",
            env!("CARGO_MANIFEST_DIR")
        );
        serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap()
    }

    fn indevidos(bill: &ExtractedBill, answers: &Answers) -> Vec<Finding> {
        evaluate(bill, answers)
            .findings
            .into_iter()
            .filter(|f| f.severity == Severity::Unlawful)
            .collect()
    }

    // RF11 "Texto com os achados selecionados": item, valor, mês, base legal, pedido e protocolo.
    #[test]
    fn texto_cita_itens_valor_mes_base_legal_e_protocolo() {
        let bill = fixture("telecom-2");
        let fs = indevidos(&bill, &Answers::new());
        assert_eq!(fs.len(), 2);
        let refs: Vec<&Finding> = fs.iter().collect();
        let d = build(&bill, &refs, Some(false), None).unwrap();
        assert!(d.text.contains("Vivaz Fibra"));
        assert!(d.text.contains("outubro de 2026"));
        assert!(d.text.contains("Multa por atraso: R$ 3,60"));
        assert!(d.text.contains("Juros por atraso: R$ 2,40"));
        assert!(d.text.contains("RGC Anatel art. 78"));
        assert!(d.text.contains("Total contestado: R$ 6,00"));
        assert!(d.text.contains("cancelamento dessas cobranças"));
        assert!(d.text.contains("número de protocolo"));
        assert_eq!(d.total_cents, 600);
        assert_eq!(d.disclaimer, DISCLAIMER);
    }

    #[test]
    fn fatura_paga_pede_dobro_com_rgc_64() {
        let bill = fixture("telecom-1");
        let mut answers = Answers::new();
        answers.insert("TEL-01:i2".into(), Answer::No);
        let fs = indevidos(&bill, &answers);
        let refs: Vec<&Finding> = fs.iter().collect();
        let d = build(&bill, &refs, Some(true), None).unwrap();
        assert!(d.text.contains("Clube Vivaz Música: R$ 12,90"));
        assert!(
            d.text
                .contains("devolução em dobro do valor cobrado indevidamente (R$ 25,80)")
        );
        assert!(d.text.contains("art. 64"));
    }

    #[test]
    fn protocolo_informado_aparece_no_texto() {
        let bill = fixture("telecom-2");
        let fs = indevidos(&bill, &Answers::new());
        let refs: Vec<&Finding> = fs.iter().collect();
        let d = build(&bill, &refs, None, Some(" 2026111012345 ")).unwrap();
        assert!(d.text.contains("protocolo 2026111012345"));
    }

    // RF11 "Nenhum achado selecionado".
    #[test]
    fn selecao_vazia_e_erro() {
        assert_eq!(
            build(&fixture("telecom-3"), &[], None, None),
            Err(DisputeError::Empty)
        );
    }

    #[test]
    fn achado_ok_nao_pode_ser_contestado() {
        let bill = fixture("telecom-1");
        let mut answers = Answers::new();
        answers.insert("TEL-01:i2".into(), Answer::Yes);
        let ev = evaluate(&bill, &answers);
        let ok: Vec<&Finding> = ev
            .findings
            .iter()
            .filter(|f| f.severity == Severity::Ok)
            .collect();
        assert!(matches!(
            build(&bill, &ok, None, None),
            Err(DisputeError::NotDisputable(_))
        ));
    }

    // RF12: ordem dos passos e prazos.
    #[test]
    fn passos_de_telecom_na_ordem_com_prazos() {
        let titles: Vec<String> = steps(BillKind::Telecom)
            .into_iter()
            .map(|s| s.title)
            .collect();
        assert_eq!(titles, ["SAC da operadora", "Anatel", "consumidor.gov.br"]);
        let d = deadlines(BillKind::Telecom);
        assert!(d.contains(&"A operadora tem 30 dias para responder".to_string()));
        assert!(d.contains(&"Você pode contestar cobranças de até 3 anos".to_string()));
    }

    #[test]
    fn passos_de_energia_na_ordem() {
        let titles: Vec<String> = steps(BillKind::Energy)
            .into_iter()
            .map(|s| s.title)
            .collect();
        assert_eq!(
            titles,
            ["Distribuidora", "Ouvidoria", "ANEEL", "consumidor.gov.br"]
        );
    }

    #[test]
    fn links_oficiais_sao_https() {
        for kind in [
            BillKind::Telecom,
            BillKind::Energy,
            BillKind::BankStatement,
            BillKind::CardBill,
        ] {
            for s in steps(kind) {
                if let Some(u) = s.url {
                    assert!(u.starts_with("https://"), "{u}");
                }
            }
        }
    }

    // RF11 + RF12 de ponta a ponta para energia (energia-2): avalia, seleciona os
    // indevidos (multa da ENE-01 e seguro negado da ENE-07) e gera a contestação.
    fn energia_2_indevidos() -> (ExtractedBill, Vec<Finding>) {
        let bill = fixture("energia-2");
        let mut answers = Answers::new();
        answers.insert("ENE-07:i6".into(), Answer::No);
        let fs = indevidos(&bill, &answers);
        (bill, fs)
    }

    #[test]
    fn contestacao_de_energia_em_aberto_cita_a_ren() {
        let (bill, fs) = energia_2_indevidos();
        let ids: Vec<&str> = fs.iter().map(|f| f.id.as_str()).collect();
        assert_eq!(ids, ["ENE-07:i6", "ENE-01:i7"]);
        let refs: Vec<&Finding> = fs.iter().collect();
        let d = build(&bill, &refs, Some(false), None).unwrap();
        assert!(
            d.text.contains("Sou cliente da Luzia Energia"),
            "{}",
            d.text
        );
        assert!(
            d.text
                .contains("fatura de outubro de 2026 (vencimento em 10/11/2026)")
        );
        assert!(d.text.contains("Seguro Residencial Luzia: R$ 12,90"));
        assert!(d.text.contains("Multa por atraso: R$ 5,17"));
        assert!(d.text.contains("REN ANEEL 1.000/2021, art. 343"));
        assert!(d.text.contains("REN ANEEL 1.000/2021, arts. 331 e 413"));
        assert!(d.text.contains("CDC art. 39, III"));
        assert!(d.text.contains("Total contestado: R$ 18,07"));
        assert!(d.text.contains("art. 413 da REN ANEEL 1.000/2021"));
        assert!(d.text.contains("art. 323 da REN ANEEL 1.000/2021"));
        assert!(d.text.contains("art. 408 da REN ANEEL 1.000/2021"));
        assert!(!d.text.contains("Anatel"));
        assert!(
            !d.text.contains("60 ciclos"),
            "docs/03: não afirmar 60 ciclos"
        );
        assert!(d.text.contains("número de protocolo"));
        assert_eq!(d.total_cents, 1_807);
        let titles: Vec<&str> = d.steps.iter().map(|s| s.title.as_str()).collect();
        assert_eq!(
            titles,
            ["Distribuidora", "Ouvidoria", "ANEEL", "consumidor.gov.br"]
        );
        assert_eq!(d.deadlines, deadlines(BillKind::Energy));
        assert_eq!(d.disclaimer, DISCLAIMER);
    }

    #[test]
    fn contestacao_de_energia_paga_pede_dobro_com_cdc_42_e_ren_323() {
        let (bill, fs) = energia_2_indevidos();
        let refs: Vec<&Finding> = fs.iter().collect();
        let d = build(&bill, &refs, Some(true), Some("2026111099")).unwrap();
        assert!(
            d.text
                .contains("devolução em dobro do valor cobrado indevidamente (R$ 36,14)")
        );
        assert!(d.text.contains(
            "art. 42 do Código de Defesa do Consumidor e o art. 323 da REN ANEEL 1.000/2021"
        ));
        assert!(
            !d.text.contains("art. 413"),
            "fatura paga: dobro, não suspensão"
        );
        assert!(d.text.contains("protocolo 2026111099"));
    }

    #[test]
    fn contestacao_de_telecom_nao_cita_a_ren() {
        let bill = fixture("telecom-2");
        let fs = indevidos(&bill, &Answers::new());
        let refs: Vec<&Finding> = fs.iter().collect();
        for paid in [None, Some(true)] {
            let d = build(&bill, &refs, paid, None).unwrap();
            assert!(!d.text.contains("REN ANEEL"), "{}", d.text);
        }
    }

    // Regressão: a soma dos achados com valores absurdos estourava i64 (panic em debug).
    #[test]
    fn soma_de_achados_acima_de_i64_max_nao_estoura() {
        let bill = fixture("telecom-2");
        let base = indevidos(&bill, &Answers::new())
            .into_iter()
            .next()
            .expect("telecom-2 tem achado indevido");
        let a = Finding {
            amount_cents: i64::MAX,
            ..base.clone()
        };
        let b = Finding {
            amount_cents: i64::MAX,
            ..base
        };
        let refs: Vec<&Finding> = vec![&a, &b];
        for paid in [None, Some(true)] {
            let d = build(&bill, &refs, paid, None).expect("não deve dar panic nem erro");
            // Soma saturada em i64::MAX, nunca um valor "enrolado".
            assert_eq!(d.total_cents, i64::MAX);
            assert!(
                d.text
                    .contains("Total contestado: R$ 92.233.720.368.547.758,07")
            );
            assert!(d.text.contains("Itens contestados:"));
        }
    }

    // Regressão: valores negativos extremos também saturam (no piso), sem panic.
    #[test]
    fn soma_de_achados_abaixo_de_i64_min_nao_estoura() {
        let bill = fixture("telecom-2");
        let base = indevidos(&bill, &Answers::new())
            .into_iter()
            .next()
            .expect("telecom-2 tem achado indevido");
        let a = Finding {
            amount_cents: i64::MIN,
            ..base.clone()
        };
        let b = Finding {
            amount_cents: i64::MIN,
            ..base
        };
        let refs: Vec<&Finding> = vec![&a, &b];
        let d = build(&bill, &refs, Some(true), None).expect("não deve dar panic");
        assert_eq!(d.total_cents, i64::MIN);
        assert!(d.text.contains("Total contestado:"));
    }

    // RF17 "Roteiro com os achados": apresentar-se, citar o mês, citar cada item e o
    // valor, pedir o estorno e pedir o protocolo, nessa ordem e numerados.
    #[test]
    fn roteiro_de_ligacao_segue_a_ordem_do_rf17() {
        let bill = fixture("telecom-2");
        let fs = indevidos(&bill, &Answers::new());
        let refs: Vec<&Finding> = fs.iter().collect();
        let roteiro = call_script(&bill, &refs, Some(false));
        let titulos: Vec<&str> = roteiro.iter().map(|p| p.title.as_str()).collect();
        assert_eq!(
            titulos,
            [
                "Apresente-se",
                "Diga o mês da fatura",
                "Cite o item 1 de 2",
                "Cite o item 2 de 2",
                "Peça o estorno",
                "Peça o número do protocolo",
            ]
        );
        assert!(roteiro[0].say.contains("Vivaz Fibra"));
        assert!(roteiro[1].say.contains("outubro de 2026"));
        assert!(roteiro[1].say.contains("10/11/2026"));
        assert!(roteiro[2].say.contains("Multa por atraso"));
        assert!(roteiro[2].say.contains("R$ 3,60"));
        assert!(roteiro[2].say.contains("RGC Anatel art. 78"));
        assert!(roteiro[3].say.contains("Juros por atraso"));
        assert!(roteiro[3].say.contains("R$ 2,40"));
        assert!(roteiro[4].say.contains("R$ 6,00"));
        assert!(roteiro[4].say.contains("cancelamento"));
        assert!(roteiro[5].say.contains("número do protocolo"));
    }

    #[test]
    fn roteiro_de_fatura_paga_pede_dobro() {
        let bill = fixture("telecom-2");
        let fs = indevidos(&bill, &Answers::new());
        let refs: Vec<&Finding> = fs.iter().collect();
        let roteiro = call_script(&bill, &refs, Some(true));
        let estorno = &roteiro[roteiro.len() - 2].say;
        assert!(estorno.contains("devolução em dobro"), "{estorno}");
        assert!(estorno.contains("R$ 12,00"), "{estorno}");
        assert!(estorno.contains("art. 64"), "{estorno}");
    }

    #[test]
    fn roteiro_de_energia_em_aberto_pede_suspensao_da_parcela() {
        let (bill, fs) = energia_2_indevidos();
        let refs: Vec<&Finding> = fs.iter().collect();
        let roteiro = call_script(&bill, &refs, Some(false));
        assert!(roteiro[0].say.contains("Luzia Energia"));
        let estorno = &roteiro[roteiro.len() - 2].say;
        assert!(estorno.contains("suspensão"), "{estorno}");
        assert!(estorno.contains("art. 413"), "{estorno}");
        assert!(!estorno.contains("Anatel"));
    }

    // Texto fixo, sem IA: a mesma entrada dá sempre o mesmo roteiro.
    #[test]
    fn roteiro_e_deterministico() {
        let bill = fixture("telecom-2");
        let fs = indevidos(&bill, &Answers::new());
        let refs: Vec<&Finding> = fs.iter().collect();
        assert_eq!(
            call_script(&bill, &refs, None),
            call_script(&bill, &refs, None)
        );
    }

    #[test]
    fn protocolo_valido_so_aceita_digitos_e_hifen_ate_30() {
        for ok in ["", "2026111012345", "2026-1110-12345", &"9".repeat(30)] {
            assert!(protocol_is_valid(ok), "{ok:?} deveria ser válido");
        }
        for ruim in [
            "<script>",
            "ABC123",
            "2026 111",
            "2026/111",
            "12.345",
            "１２３",
            &"9".repeat(31),
        ] {
            assert!(!protocol_is_valid(ruim), "{ruim:?} deveria ser inválido");
        }
    }

    #[test]
    fn mes_por_extenso() {
        assert_eq!(month_pt("2026-10"), "outubro de 2026");
        assert_eq!(month_pt("2026-13"), "2026-13");
    }

    #[test]
    fn ids_de_achado_sao_estaveis() {
        let ev = evaluate(&fixture("telecom-1"), &Answers::new());
        let ids: Vec<&str> = ev.findings.iter().map(|f| f.id.as_str()).collect();
        assert!(ids.contains(&"TEL-01:i2"));
        assert!(ids.contains(&"TEL-01:i3"));
    }

    // ---------- extrato bancário (#54) ----------

    /// extrato-1 com a cesta e o saque contestados ("não contratei" e "não passei de 4").
    fn extrato_1_indevidos() -> (ExtractedBill, Vec<Finding>) {
        let bill = fixture("extrato-1");
        let mut answers = Answers::new();
        answers.insert("BAN-01:i1".into(), Answer::No);
        answers.insert("BAN-02:saque".into(), Answer::No);
        let fs = indevidos(&bill, &answers);
        (bill, fs)
    }

    #[test]
    fn contestacao_de_extrato_cita_o_extrato_a_norma_do_cmn_e_pede_estorno_em_dobro() {
        let (bill, fs) = extrato_1_indevidos();
        let ids: Vec<&str> = fs.iter().map(|f| f.id.as_str()).collect();
        assert_eq!(ids, ["BAN-01:i1", "BAN-02:i3"]);
        let refs: Vec<&Finding> = fs.iter().collect();
        let d = build(&bill, &refs, None, None).unwrap();
        assert!(
            d.text.contains(
                "Sou cliente de Banco Aurora e contesto cobranças do extrato da minha conta corrente de outubro de 2026 (período até 31/10/2026)"
            ),
            "{}",
            d.text
        );
        assert!(!d.text.contains("vencimento"), "extrato não vence");
        assert!(
            d.text
                .contains("Cesta Aurora Mais (pacote de tarifas): R$ 39,90")
        );
        assert!(d.text.contains("Tarifa de saque no terminal: R$ 6,50"));
        assert!(d.text.contains("Res. CMN 3.919/2010, arts. 8º e 9º, I"));
        assert!(d.text.contains("Res. CMN 3.919/2010, art. 2º, I"));
        assert!(d.text.contains("Total contestado: R$ 46,40"));
        assert!(d.text.contains("estorno dos valores na minha conta"));
        assert!(d.text.contains("devolução em dobro"));
        assert!(d.text.contains("art. 42 do Código de Defesa do Consumidor"));
        assert!(d.text.contains("Peço uma resposta em até 7 dias corridos"));
        for outro_setor in ["Anatel", "ANEEL", "REN ", "RGC"] {
            assert!(!d.text.contains(outro_setor), "{outro_setor}: {}", d.text);
        }
        // Nada de prazo legal de SAC atribuído a decreto (a aplicação a bancos está a validar).
        assert!(!d.text.contains("Decreto"), "{}", d.text);
        assert_eq!(d.total_cents, 4_640);
        let titles: Vec<&str> = d.steps.iter().map(|s| s.title.as_str()).collect();
        assert_eq!(
            titles,
            [
                "SAC do banco",
                "Ouvidoria",
                "Banco Central",
                "consumidor.gov.br"
            ]
        );
        assert_eq!(d.steps[2].url.as_deref(), Some(BCB_RECLAMACAO_URL));
        assert!(d.steps[2].description.contains("Registrato"));
        assert_eq!(d.deadlines, deadlines(BillKind::BankStatement));
    }

    #[test]
    fn contestacao_de_extrato_paga_pede_dobro_sem_norma_setorial() {
        let (bill, fs) = extrato_1_indevidos();
        let refs: Vec<&Finding> = fs.iter().collect();
        let d = build(&bill, &refs, Some(true), None).unwrap();
        assert!(
            d.text
                .contains("devolução em dobro do valor cobrado indevidamente (R$ 92,80)")
        );
        assert!(
            d.text
                .contains("conforme o art. 42 do Código de Defesa do Consumidor.\n"),
            "{}",
            d.text
        );
    }

    #[test]
    fn roteiro_de_extrato_fala_do_extrato_e_do_estorno() {
        let (bill, fs) = extrato_1_indevidos();
        let refs: Vec<&Finding> = fs.iter().collect();
        let roteiro = call_script(&bill, &refs, None);
        assert!(roteiro[0].say.contains("Sou cliente de Banco Aurora"));
        assert!(roteiro[0].say.contains("no extrato da minha conta"));
        assert_eq!(roteiro[1].title, "Diga o mês do extrato");
        assert!(roteiro[1].say.contains("outubro de 2026"));
        assert!(roteiro[1].say.contains("31/10/2026"));
        let estorno = &roteiro[roteiro.len() - 2].say;
        assert!(estorno.contains("R$ 46,40"), "{estorno}");
        assert!(estorno.contains("estorno na minha conta"), "{estorno}");
    }

    #[test]
    fn passos_de_banco_e_cartao_na_ordem_com_prazos() {
        let titulos = |k| -> Vec<String> { steps(k).into_iter().map(|s| s.title).collect() };
        assert_eq!(
            titulos(BillKind::BankStatement),
            [
                "SAC do banco",
                "Ouvidoria",
                "Banco Central",
                "consumidor.gov.br"
            ]
        );
        assert_eq!(
            titulos(BillKind::CardBill),
            [
                "SAC do cartão",
                "Ouvidoria",
                "Banco Central",
                "consumidor.gov.br"
            ]
        );
        let d = deadlines(BillKind::CardBill);
        assert!(d.contains(&"A ouvidoria tem até 10 dias úteis para responder".to_string()));
        assert!(d.iter().any(|x| x.contains("Banco Central")));
    }

    // ---------- fatura do cartão (#55) ----------

    #[test]
    fn contestacao_de_cartao_cita_a_fatura_o_cdc_52_e_pede_estorno_na_proxima() {
        let bill = fixture("cartao-1");
        let mut answers = Answers::new();
        answers.insert("CAR-01:i3".into(), Answer::No);
        let fs = indevidos(&bill, &answers);
        let ids: Vec<&str> = fs.iter().map(|f| f.id.as_str()).collect();
        assert_eq!(ids, ["CAR-04:i5", "CAR-01:i3"]);
        let refs: Vec<&Finding> = fs.iter().collect();
        let d = build(&bill, &refs, Some(false), None).unwrap();
        assert!(
            d.text.contains(
                "Sou cliente de Aurora Card e contesto cobranças da fatura do cartão de outubro de 2026 (vencimento em 10/11/2026)"
            ),
            "{}",
            d.text
        );
        assert!(d.text.contains("Multa por atraso: R$ 17,00"));
        assert!(d.text.contains("CDC art. 52, § 1º"));
        assert!(d.text.contains("Seguro Perda e Roubo: R$ 9,90"));
        assert!(d.text.contains("CDC art. 39, III"));
        assert!(d.text.contains("Total contestado: R$ 26,90"));
        assert!(d.text.contains("estorno dos valores na próxima fatura"));
        assert!(d.text.contains("Peço uma resposta em até 7 dias corridos"));
        for outro_setor in ["Anatel", "ANEEL", "REN ", "RGC", "Decreto"] {
            assert!(!d.text.contains(outro_setor), "{outro_setor}: {}", d.text);
        }
        let titles: Vec<&str> = d.steps.iter().map(|s| s.title.as_str()).collect();
        assert_eq!(
            titles,
            [
                "SAC do cartão",
                "Ouvidoria",
                "Banco Central",
                "consumidor.gov.br"
            ]
        );
        let d = build(&bill, &refs, Some(true), None).unwrap();
        assert!(
            d.text
                .contains("devolução em dobro do valor cobrado indevidamente (R$ 53,80)")
        );
        let roteiro = call_script(&bill, &refs, Some(false));
        assert!(roteiro[0].say.contains("na fatura do meu cartão"));
        assert_eq!(roteiro[1].title, "Diga o mês da fatura");
        let estorno = &roteiro[roteiro.len() - 2].say;
        assert!(estorno.contains("estorno na próxima fatura"), "{estorno}");
    }
}
