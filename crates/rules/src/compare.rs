//! RF16: comparação entre duas faturas (docs/02, docs/04).
//! Cálculo puro e determinístico, sem IA: casa os itens pela descrição normalizada.

use std::cmp::Reverse;

use humanobill_core::{Cents, ComparedItem, Comparison, ExtractedBill, ItemCategory};

use crate::text::{normalized, tokens};

/// Por que duas faturas não podem ser comparadas.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompareError {
    /// Empresa (ou tipo de conta) diferente.
    DifferentProvider,
    /// Mesmo mês de referência.
    SameMonth,
}

/// Item de uma fatura, já somado quando a descrição se repete.
struct Line {
    key: String,
    description: String,
    category: ItemCategory,
    cents: Cents,
}

/// Chave de casamento: descrição sem caixa, acento e pontuação, e sem os números soltos
/// ("Parcela do aparelho 3/12" e "4/12" são o mesmo item; "324 kWh" e "198 kWh" também).
fn key(description: &str) -> String {
    let all = tokens(description);
    let words: Vec<&str> = all
        .iter()
        .map(String::as_str)
        .filter(|t| !t.chars().all(|c| c.is_ascii_digit()))
        .collect();
    if words.is_empty() {
        all.join(" ")
    } else {
        words.join(" ")
    }
}

/// Itens da fatura na ordem original; descrições repetidas viram uma linha só, com a soma.
fn lines(bill: &ExtractedBill) -> Vec<Line> {
    let mut out: Vec<Line> = Vec::new();
    for item in &bill.items {
        let k = key(&item.description);
        match out.iter_mut().find(|l| l.key == k) {
            Some(l) => l.cents = l.cents.saturating_add(item.amount_cents),
            None => out.push(Line {
                key: k,
                description: item.description.clone(),
                category: item.category,
                cents: item.amount_cents,
            }),
        }
    }
    out
}

/// Variação em %, com 1 casa, arredondada com meio para longe do zero.
/// `None` quando o valor anterior é zero. Usa inteiros para não depender de float na conta.
fn pct(delta: Cents, before: Cents) -> Option<f64> {
    if before == 0 {
        return None;
    }
    let num = i128::from(delta) * 1000;
    let den = i128::from(before).abs();
    let tenths = (num.abs() * 2 + den) / (2 * den);
    let signed = if num < 0 { -tenths } else { tenths };
    Some(signed as f64 / 10.0)
}

/// Serviço adicional que passou a custar algo é cobrança nova.
fn is_new_charge(category: ItemCategory, before: Cents, after: Cents) -> bool {
    category == ItemCategory::ServicoAdicional && before == 0 && after > 0
}

fn added(l: &Line) -> ComparedItem {
    ComparedItem {
        description: l.description.clone(),
        category: l.category,
        before_cents: None,
        after_cents: Some(l.cents),
        delta_cents: l.cents,
        delta_pct: None,
        new_charge: is_new_charge(l.category, 0, l.cents),
    }
}

fn removed(l: &Line) -> ComparedItem {
    ComparedItem {
        description: l.description.clone(),
        category: l.category,
        before_cents: Some(l.cents),
        after_cents: None,
        delta_cents: 0i64.saturating_sub(l.cents),
        delta_pct: None,
        new_charge: false,
    }
}

fn both(before: &Line, after: &Line) -> ComparedItem {
    let delta = after.cents.saturating_sub(before.cents);
    ComparedItem {
        description: after.description.clone(),
        category: after.category,
        before_cents: Some(before.cents),
        after_cents: Some(after.cents),
        delta_cents: delta,
        delta_pct: pct(delta, before.cents),
        new_charge: is_new_charge(after.category, before.cents, after.cents),
    }
}

/// Maiores variações primeiro (a ordem de leitura da fatura desempata).
fn by_impact(items: &mut [ComparedItem]) {
    items.sort_by_key(|i| Reverse(i.delta_cents.unsigned_abs()));
}

/// Compara duas faturas, em qualquer ordem: a de mês mais antigo vira o "antes".
pub fn compare(a: &ExtractedBill, b: &ExtractedBill) -> Result<Comparison, CompareError> {
    if a.kind != b.kind || normalized(&a.provider_name) != normalized(&b.provider_name) {
        return Err(CompareError::DifferentProvider);
    }
    let (ma, mb) = (a.reference_month.trim(), b.reference_month.trim());
    if ma == mb {
        return Err(CompareError::SameMonth);
    }
    // "YYYY-MM" com zero à esquerda ordena certo como texto.
    let (before, after) = if ma < mb { (a, b) } else { (b, a) };

    let old = lines(before);
    let new = lines(after);
    let (mut added_items, mut changed, mut unchanged) = (Vec::new(), Vec::new(), Vec::new());
    for n in &new {
        match old.iter().find(|o| o.key == n.key) {
            None => added_items.push(added(n)),
            Some(o) if o.cents == n.cents => unchanged.push(both(o, n)),
            Some(o) => changed.push(both(o, n)),
        }
    }
    let mut removed_items: Vec<ComparedItem> = old
        .iter()
        .filter(|o| !new.iter().any(|n| n.key == o.key))
        .map(removed)
        .collect();
    by_impact(&mut added_items);
    by_impact(&mut changed);
    by_impact(&mut removed_items);

    let total_delta = after.total_cents.saturating_sub(before.total_cents);
    Ok(Comparison {
        kind: after.kind,
        provider_name: after.provider_name.clone(),
        before_month: before.reference_month.clone(),
        after_month: after.reference_month.clone(),
        before_total_cents: before.total_cents,
        after_total_cents: after.total_cents,
        total_delta_cents: total_delta,
        total_delta_pct: pct(total_delta, before.total_cents),
        added: added_items,
        removed: removed_items,
        changed,
        unchanged,
    })
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;
    use humanobill_core::{BillItem, BillKind, Cents, ItemCategory};

    use super::*;

    fn item(id: &str, desc: &str, cents: Cents, category: ItemCategory) -> BillItem {
        BillItem {
            id: id.into(),
            description: desc.into(),
            amount_cents: cents,
            category,
            period: None,
            original_text: format!("{desc} (texto original)"),
            page: 1,
            plain_explanation: "Explicação.".into(),
        }
    }

    fn bill(provider: &str, month: &str, items: Vec<BillItem>) -> ExtractedBill {
        let total = items.iter().map(|i| i.amount_cents).sum();
        ExtractedBill {
            kind: BillKind::Telecom,
            provider_name: provider.into(),
            reference_month: month.into(),
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

    fn plano(cents: Cents) -> BillItem {
        item(
            "i1",
            "Plano Vivaz Controle 20GB",
            cents,
            ItemCategory::Plano,
        )
    }

    fn outubro() -> ExtractedBill {
        bill("Vivaz Telecom", "2026-10", vec![plano(9990)])
    }

    // Cenário: Item novo aparece
    #[test]
    fn item_novo_aparece_como_novo_com_variacao_em_reais_e_porcentagem() {
        let nov = bill(
            "Vivaz Telecom",
            "2026-11",
            vec![
                plano(9990),
                item("i2", "Seguro Tela", 990, ItemCategory::ServicoAdicional),
            ],
        );
        let c = compare(&outubro(), &nov).unwrap();

        assert_eq!(c.added.len(), 1);
        let novo = &c.added[0];
        assert_eq!(novo.description, "Seguro Tela");
        assert_eq!(novo.before_cents, None);
        assert_eq!(novo.after_cents, Some(990));
        assert_eq!(novo.delta_cents, 990);
        assert_eq!(novo.delta_pct, None);
        assert!(novo.new_charge, "serviço adicional novo é cobrança nova");

        assert_eq!(c.before_total_cents, 9990);
        assert_eq!(c.after_total_cents, 10980);
        assert_eq!(c.total_delta_cents, 990);
        assert_eq!(c.total_delta_pct, Some(9.9));
        assert_eq!(c.before_month, "2026-10");
        assert_eq!(c.after_month, "2026-11");
        assert_eq!(c.provider_name, "Vivaz Telecom");
        assert!(c.removed.is_empty() && c.changed.is_empty());
        assert_eq!(c.unchanged.len(), 1);
    }

    #[test]
    fn a_ordem_dos_argumentos_nao_importa() {
        let nov = bill(
            "Vivaz Telecom",
            "2026-11",
            vec![
                plano(9990),
                item("i2", "Seguro Tela", 990, ItemCategory::ServicoAdicional),
            ],
        );
        assert_eq!(
            compare(&outubro(), &nov).unwrap(),
            compare(&nov, &outubro()).unwrap()
        );
    }

    #[test]
    fn item_que_sumiu_vira_removido_e_o_total_cai() {
        let out = bill(
            "Vivaz Telecom",
            "2026-10",
            vec![
                plano(9990),
                item(
                    "i2",
                    "Clube Vivaz Música",
                    1290,
                    ItemCategory::ServicoAdicional,
                ),
            ],
        );
        let nov = bill("Vivaz Telecom", "2026-11", vec![plano(9990)]);
        let c = compare(&out, &nov).unwrap();

        assert_eq!(c.removed.len(), 1);
        let r = &c.removed[0];
        assert_eq!(r.description, "Clube Vivaz Música");
        assert_eq!(r.before_cents, Some(1290));
        assert_eq!(r.after_cents, None);
        assert_eq!(r.delta_cents, -1290);
        assert_eq!(r.delta_pct, None);
        assert!(!r.new_charge);
        assert_eq!(c.total_delta_cents, -1290);
        assert_eq!(c.total_delta_pct, Some(-11.4));
    }

    #[test]
    fn item_com_valor_diferente_vira_alterado_com_variacao() {
        let nov = bill("Vivaz Telecom", "2026-11", vec![plano(10990)]);
        let c = compare(&outubro(), &nov).unwrap();

        assert_eq!(c.changed.len(), 1);
        let a = &c.changed[0];
        assert_eq!(a.before_cents, Some(9990));
        assert_eq!(a.after_cents, Some(10990));
        assert_eq!(a.delta_cents, 1000);
        assert_eq!(a.delta_pct, Some(10.0));
        assert!(!a.new_charge, "reajuste do plano não é cobrança nova");
        assert!(c.added.is_empty() && c.removed.is_empty() && c.unchanged.is_empty());
    }

    #[test]
    fn item_igual_nos_dois_meses_fica_em_sem_mudanca() {
        let nov = bill("Vivaz Telecom", "2026-11", vec![plano(9990)]);
        let c = compare(&outubro(), &nov).unwrap();
        assert_eq!(c.unchanged.len(), 1);
        assert_eq!(c.unchanged[0].delta_cents, 0);
        assert_eq!(c.unchanged[0].delta_pct, Some(0.0));
        assert_eq!(c.total_delta_cents, 0);
        assert_eq!(c.total_delta_pct, Some(0.0));
    }

    // Cenário: Operadoras diferentes
    #[test]
    fn operadoras_diferentes_nao_comparam() {
        let outra = bill("Luzia Energia", "2026-11", vec![plano(9990)]);
        assert_eq!(
            compare(&outubro(), &outra),
            Err(CompareError::DifferentProvider)
        );
    }

    #[test]
    fn tipos_diferentes_nao_comparam_mesmo_com_o_mesmo_nome() {
        let mut luz = bill("Vivaz Telecom", "2026-11", vec![plano(9990)]);
        luz.kind = BillKind::Energy;
        assert_eq!(
            compare(&outubro(), &luz),
            Err(CompareError::DifferentProvider)
        );
    }

    #[test]
    fn nome_da_operadora_ignora_caixa_acento_e_espacos_extras() {
        let a = bill("Vivaz  Telecom", "2026-10", vec![plano(9990)]);
        let b = bill("VIVAZ telecom ", "2026-11", vec![plano(9990)]);
        assert!(compare(&a, &b).is_ok());
        let c = bill("Vívaz Telecom", "2026-11", vec![plano(9990)]);
        assert!(compare(&a, &c).is_ok());
    }

    // Cenário: Mesma fatura duas vezes
    #[test]
    fn mesmo_mes_de_referencia_nao_compara() {
        assert_eq!(
            compare(&outubro(), &outubro()),
            Err(CompareError::SameMonth)
        );
        let outra_do_mesmo_mes = bill("Vivaz Telecom", "2026-10", vec![plano(11000)]);
        assert_eq!(
            compare(&outubro(), &outra_do_mesmo_mes),
            Err(CompareError::SameMonth)
        );
    }

    #[test]
    fn operadora_diferente_tem_prioridade_sobre_mesmo_mes() {
        let outra = bill("Luzia Energia", "2026-10", vec![plano(9990)]);
        assert_eq!(
            compare(&outubro(), &outra),
            Err(CompareError::DifferentProvider)
        );
    }

    // Casamento dos itens
    #[test]
    fn descricao_casa_ignorando_caixa_acento_e_espacos() {
        let out = bill(
            "Vivaz Telecom",
            "2026-10",
            vec![item(
                "i1",
                "Clube Vivaz Música",
                1290,
                ItemCategory::ServicoAdicional,
            )],
        );
        let nov = bill(
            "Vivaz Telecom",
            "2026-11",
            vec![item(
                "i1",
                "CLUBE  VIVAZ MUSICA!",
                1490,
                ItemCategory::ServicoAdicional,
            )],
        );
        let c = compare(&out, &nov).unwrap();
        assert!(c.added.is_empty() && c.removed.is_empty());
        assert_eq!(c.changed.len(), 1);
        assert_eq!(c.changed[0].description, "CLUBE  VIVAZ MUSICA!");
    }

    #[test]
    fn numeros_na_descricao_nao_impedem_o_casamento() {
        // Parcela do aparelho 3/12 -> 4/12 e kWh que muda de um mês para o outro.
        let out = bill(
            "Vivaz Telecom",
            "2026-10",
            vec![
                item("i1", "Parcela do aparelho 3/12", 5000, ItemCategory::Outros),
                item("i2", "Uso da rede 324 kWh", 14904, ItemCategory::Energia),
            ],
        );
        let nov = bill(
            "Vivaz Telecom",
            "2026-11",
            vec![
                item("i1", "Parcela do aparelho 4/12", 5000, ItemCategory::Outros),
                item("i2", "Uso da rede 198 kWh", 9108, ItemCategory::Energia),
            ],
        );
        let c = compare(&out, &nov).unwrap();
        assert!(c.added.is_empty() && c.removed.is_empty());
        assert_eq!(c.unchanged.len(), 1);
        assert_eq!(c.changed.len(), 1);
        assert_eq!(c.changed[0].delta_cents, 9108 - 14904);
    }

    #[test]
    fn descricao_so_com_numeros_ainda_casa_pela_descricao_inteira() {
        let out = bill(
            "Vivaz Telecom",
            "2026-10",
            vec![item("i1", "123", 100, ItemCategory::Outros)],
        );
        let nov = bill(
            "Vivaz Telecom",
            "2026-11",
            vec![item("i1", "123", 150, ItemCategory::Outros)],
        );
        let c = compare(&out, &nov).unwrap();
        assert_eq!(c.changed.len(), 1);
    }

    #[test]
    fn itens_repetidos_na_mesma_fatura_somam_antes_de_comparar() {
        let out = bill(
            "Vivaz Telecom",
            "2026-10",
            vec![
                item("i1", "Multa por atraso", 300, ItemCategory::EncargoAtraso),
                item("i2", "Multa por atraso", 300, ItemCategory::EncargoAtraso),
            ],
        );
        let nov = bill(
            "Vivaz Telecom",
            "2026-11",
            vec![item(
                "i1",
                "Multa por atraso",
                600,
                ItemCategory::EncargoAtraso,
            )],
        );
        let c = compare(&out, &nov).unwrap();
        assert_eq!(c.unchanged.len(), 1);
        assert_eq!(c.unchanged[0].after_cents, Some(600));
    }

    // Cobrança nova
    #[test]
    fn so_servico_adicional_com_valor_vira_cobranca_nova() {
        let nov = bill(
            "Vivaz Telecom",
            "2026-11",
            vec![
                plano(9990),
                item("i2", "Taxa de instalação", 5000, ItemCategory::Outros),
                item("i3", "Multa por atraso", 200, ItemCategory::EncargoAtraso),
                item("i4", "Brinde", 0, ItemCategory::ServicoAdicional),
                item("i5", "Seguro Tela", 990, ItemCategory::ServicoAdicional),
            ],
        );
        let c = compare(&outubro(), &nov).unwrap();
        let novas: Vec<&str> = c
            .added
            .iter()
            .filter(|i| i.new_charge)
            .map(|i| i.description.as_str())
            .collect();
        assert_eq!(novas, ["Seguro Tela"]);
        assert_eq!(c.added.len(), 4);
    }

    #[test]
    fn servico_que_era_gratis_e_passou_a_cobrar_vira_cobranca_nova() {
        let out = bill(
            "Vivaz Telecom",
            "2026-10",
            vec![item(
                "i1",
                "Clube Vivaz Música",
                0,
                ItemCategory::ServicoAdicional,
            )],
        );
        let nov = bill(
            "Vivaz Telecom",
            "2026-11",
            vec![item(
                "i1",
                "Clube Vivaz Música",
                1290,
                ItemCategory::ServicoAdicional,
            )],
        );
        let c = compare(&out, &nov).unwrap();
        assert_eq!(c.changed.len(), 1);
        assert!(c.changed[0].new_charge);
        assert_eq!(c.changed[0].delta_pct, None, "não há % sobre zero");
    }

    // Ordem e arredondamento
    #[test]
    fn maiores_variacoes_vem_primeiro() {
        let out = bill(
            "Vivaz Telecom",
            "2026-10",
            vec![
                item("i1", "A", 1000, ItemCategory::Outros),
                item("i2", "B", 1000, ItemCategory::Outros),
                item("i3", "C", 1000, ItemCategory::Outros),
            ],
        );
        let nov = bill(
            "Vivaz Telecom",
            "2026-11",
            vec![
                item("i1", "A", 1100, ItemCategory::Outros),
                item("i2", "B", 400, ItemCategory::Outros),
                item("i3", "C", 1300, ItemCategory::Outros),
            ],
        );
        let c = compare(&out, &nov).unwrap();
        let ordem: Vec<&str> = c.changed.iter().map(|i| i.description.as_str()).collect();
        assert_eq!(ordem, ["B", "C", "A"]);
    }

    #[test]
    fn porcentagem_tem_uma_casa_e_arredonda_meio_para_longe_do_zero() {
        // 1/3 = 33,33...%  ->  33,3
        let out = bill("Vivaz Telecom", "2026-10", vec![plano(3000)]);
        let nov = bill("Vivaz Telecom", "2026-11", vec![plano(4000)]);
        assert_eq!(compare(&out, &nov).unwrap().total_delta_pct, Some(33.3));
        // 0,05% exato (1 centavo em R$ 20,00) -> 0,1 ; e -0,05% -> -0,1
        let a = bill("Vivaz Telecom", "2026-10", vec![plano(2000)]);
        let b = bill("Vivaz Telecom", "2026-11", vec![plano(2001)]);
        assert_eq!(compare(&a, &b).unwrap().total_delta_pct, Some(0.1));
        assert_eq!(compare(&b, &a).unwrap().total_delta_pct, Some(0.1));
        let queda = bill("Vivaz Telecom", "2026-12", vec![plano(1999)]);
        assert_eq!(compare(&a, &queda).unwrap().total_delta_pct, Some(-0.1));
    }

    #[test]
    fn total_anterior_zero_nao_tem_porcentagem() {
        let out = bill("Vivaz Telecom", "2026-10", vec![]);
        let nov = bill("Vivaz Telecom", "2026-11", vec![plano(9990)]);
        let c = compare(&out, &nov).unwrap();
        assert_eq!(c.total_delta_cents, 9990);
        assert_eq!(c.total_delta_pct, None);
    }

    #[test]
    fn valores_enormes_nao_estouram() {
        let out = bill(
            "Vivaz Telecom",
            "2026-10",
            vec![item("i1", "A", i64::MIN + 1, ItemCategory::Outros)],
        );
        let mut nov = bill(
            "Vivaz Telecom",
            "2026-11",
            vec![item("i1", "A", i64::MAX, ItemCategory::Outros)],
        );
        nov.total_cents = i64::MAX;
        let c = compare(&out, &nov).unwrap();
        assert_eq!(c.total_delta_cents, i64::MAX);
        assert_eq!(c.changed[0].delta_cents, i64::MAX);
        assert!(c.total_delta_pct.is_some_and(f64::is_finite));
    }

    #[test]
    fn resultado_nao_carrega_texto_original_nem_ids_dos_itens() {
        let nov = bill(
            "Vivaz Telecom",
            "2026-11",
            vec![item(
                "i7",
                "Seguro Tela",
                990,
                ItemCategory::ServicoAdicional,
            )],
        );
        let json = serde_json::to_string(&compare(&outubro(), &nov).unwrap()).unwrap();
        assert!(!json.contains("texto original"));
        assert!(!json.contains("i7"));
    }
}
