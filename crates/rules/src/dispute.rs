//! Contestação pronta (RF11) e roteiro "Onde reclamar" (RF12).
//!
//! Gerado por template determinístico, sem IA: o texto sempre cita o item,
//! o valor, o mês de referência e a base legal dos achados selecionados.

use humanobill_core::{BillKind, Cents, Dispute, DisputeStep, ExtractedBill, Finding, Severity};

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

    let total_cents: Cents = selected.iter().map(|f| f.amount_cents).sum();
    let month = month_pt(&bill.reference_month);
    let due = bill.due_date.format("%d/%m/%Y");

    let mut t = String::new();
    t.push_str(&format!(
        "Olá. Sou cliente da {} e contesto cobranças da fatura de {} (vencimento em {}).\n\n",
        bill.provider_name, month, due
    ));
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

    // Norma setorial do dobro: RGC art. 64 (telecom) ou REN 1000 art. 323 (energia).
    let double_rule = match bill.kind {
        BillKind::Telecom => {
            " e o art. 64 do Regulamento Geral de Direitos do Consumidor de Telecomunicações (Anatel)"
        }
        BillKind::Energy => " e o art. 323 da REN ANEEL 1.000/2021",
    };
    match paid {
        Some(true) => t.push_str(&format!(
            "Como a fatura já foi paga, peço a devolução em dobro do valor cobrado indevidamente ({}), com correção monetária e juros, conforme o art. 42 do Código de Defesa do Consumidor{double_rule}.\n",
            brl(total_cents * 2),
        )),
        _ => match bill.kind {
            BillKind::Telecom => t.push_str(
                "Peço o cancelamento dessas cobranças e a emissão de uma nova fatura sem esses valores. Se a fatura já tiver sido paga, peço a devolução em dobro, conforme o art. 42 do Código de Defesa do Consumidor.\n",
            ),
            // Fatura em aberto: suspensão da parcela questionada (REN 1000 art. 413), sem dobro.
            BillKind::Energy => t.push_str(&format!(
                "Peço a suspensão imediata da parcela contestada, conforme o art. 413 da REN ANEEL 1.000/2021, o cancelamento dessas cobranças e a emissão de uma nova fatura sem esses valores. Se a fatura já tiver sido paga, peço a devolução em dobro, conforme o art. 42 do Código de Defesa do Consumidor{double_rule}.\n",
            )),
        },
    }
    match bill.kind {
        BillKind::Telecom => t.push_str(
            "Lembro que a operadora tem até 30 dias para responder a esta contestação.\n",
        ),
        BillKind::Energy => t.push_str(
            "Lembro que a distribuidora tem até 5 dias úteis para resolver esta reclamação (10 dias úteis se precisar de visita técnica), conforme o art. 408 da REN ANEEL 1.000/2021.\n",
        ),
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
    }
}

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
        for kind in [BillKind::Telecom, BillKind::Energy] {
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
}
