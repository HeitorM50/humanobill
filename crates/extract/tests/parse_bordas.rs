//! Testes de borda de `parse_and_validate`: JSON malformado, campos ausentes,
//! limites numéricos, datas, Unicode e acúmulo de problemas de validação.
//! Só dados sintéticos (Vivaz Telecom / Luzia Energia). Sem rede.

use humanobill_core::{BillKind, ValidationIssue};
use humanobill_extract::parse_and_validate;
use serde_json::{Value, json};

/// Fatura telecom mínima e consistente (1 item, total = item).
fn base() -> Value {
    json!({
        "kind": "telecom",
        "provider_name": "Vivaz Telecom",
        "reference_month": "2026-10",
        "due_date": "2026-11-10",
        "total_cents": 9990,
        "items": [{
            "id": "i1",
            "description": "Plano",
            "amount_cents": 9990,
            "category": "plano",
            "original_text": "PLANO 99,90",
            "page": 1,
            "plain_explanation": "Mensalidade do plano."
        }],
        "extraction_confidence": 0.9
    })
}

fn item(id: &str, cents: i64) -> Value {
    json!({
        "id": id, "description": "Item", "amount_cents": cents, "category": "outros",
        "original_text": "ITEM", "page": 1, "plain_explanation": "Item qualquer."
    })
}

fn energia() -> Value {
    let mut v = base();
    v["kind"] = json!("energy");
    v["provider_name"] = json!("Luzia Energia");
    v["energy"] = json!({
        "consumption_kwh": 324,
        "reading_type": "real",
        "reading_prev_date": "2026-09-28",
        "reading_curr_date": "2026-10-28",
        "tariff_flag": "verde",
        "history": []
    });
    v
}

fn parse(v: &Value) -> Result<humanobill_core::ExtractedBill, Vec<ValidationIssue>> {
    parse_and_validate(&v.to_string())
}

// ---------- entradas vazias / não-JSON / formato errado ----------

#[test]
fn entrada_vazia_ou_so_espacos_e_rejeitada_sem_issues() {
    for txt in ["", "   ", "\n\t\r\n", "\u{a0}"] {
        assert_eq!(
            parse_and_validate(txt).unwrap_err(),
            vec![],
            "texto {txt:?}"
        );
    }
}

#[test]
fn json_de_tipo_errado_na_raiz_e_rejeitado() {
    for txt in ["null", "[]", "[{}]", "42", "\"texto\"", "true", "{}"] {
        assert_eq!(
            parse_and_validate(txt).unwrap_err(),
            vec![],
            "texto {txt:?}"
        );
    }
}

#[test]
fn cerca_de_codigo_vazia_ou_sem_json_e_rejeitada() {
    for txt in [
        "```",
        "``````",
        "```json```",
        "```json\n```",
        "```json\n\n```",
    ] {
        assert_eq!(
            parse_and_validate(txt).unwrap_err(),
            vec![],
            "texto {txt:?}"
        );
    }
}

#[test]
fn json_truncado_no_meio_e_rejeitado() {
    let s = base().to_string();
    for corte in [1, s.len() / 2, s.len() - 1] {
        assert!(parse_and_validate(&s[..corte]).is_err(), "corte em {corte}");
    }
}

#[test]
fn texto_extra_antes_ou_depois_do_json_e_rejeitado() {
    let s = base().to_string();
    assert!(parse_and_validate(&format!("Aqui está: {s}")).is_err());
    assert!(parse_and_validate(&format!("{s} fim")).is_err());
    assert!(parse_and_validate(&format!("{s}{s}")).is_err());
}

#[test]
fn cerca_com_json_depois_do_fechamento_e_rejeitada() {
    let s = base().to_string();
    assert!(parse_and_validate(&format!("```json\n{s}\n```\nobrigado")).is_err());
}

#[test]
fn cerca_sem_fechamento_ainda_e_aceita() {
    // O modelo às vezes corta o fechamento; o conteúdo continua sendo JSON válido.
    let s = base().to_string();
    assert!(parse_and_validate(&format!("```json\n{s}")).is_ok());
    assert!(parse_and_validate(&format!("```\n{s}")).is_ok());
}

#[test]
fn cerca_com_espacos_e_quebras_ao_redor_e_aceita() {
    let s = base().to_string();
    assert!(parse_and_validate(&format!("\n\n  ```json\r\n{s}\r\n```  \n")).is_ok());
}

#[test]
fn json_sem_cerca_mas_com_espacos_e_aceito() {
    let s = base().to_string();
    assert!(parse_and_validate(&format!("  \n\t{s}\n  ")).is_ok());
}

#[test]
fn chave_duplicada_no_json_e_rejeitada() {
    let s = r#"{"kind":"telecom","kind":"energy"}"#;
    assert!(parse_and_validate(s).is_err());
}

#[test]
fn json_com_aninhamento_absurdo_nao_estoura_a_pilha() {
    let fundo = format!("{}{}", "[".repeat(50_000), "]".repeat(50_000));
    assert!(parse_and_validate(&fundo).is_err());
    let objs = format!("{}{}", "{\"a\":".repeat(50_000), "1}".repeat(50_000));
    assert!(parse_and_validate(&objs).is_err());
}

// ---------- campos ausentes / tipos errados ----------

#[test]
fn cada_campo_obrigatorio_ausente_e_rejeitado() {
    for campo in [
        "kind",
        "provider_name",
        "reference_month",
        "due_date",
        "total_cents",
        "items",
        "extraction_confidence",
    ] {
        let mut v = base();
        v.as_object_mut().unwrap().remove(campo);
        assert!(parse(&v).is_err(), "sem {campo} deveria falhar");
    }
}

#[test]
fn cada_campo_obrigatorio_do_item_ausente_e_rejeitado() {
    for campo in [
        "id",
        "description",
        "amount_cents",
        "category",
        "original_text",
        "page",
        "plain_explanation",
    ] {
        let mut v = base();
        v["items"][0].as_object_mut().unwrap().remove(campo);
        assert!(parse(&v).is_err(), "item sem {campo} deveria falhar");
    }
}

#[test]
fn campos_opcionais_ausentes_ou_nulos_sao_aceitos() {
    let mut v = base();
    v["issue_date"] = Value::Null;
    v["plan_name"] = Value::Null;
    v["late_fees"] = Value::Null;
    v["energy"] = Value::Null;
    v["items"][0]["period"] = Value::Null;
    let bill = parse(&v).unwrap();
    assert!(bill.issue_date.is_none() && bill.plan_name.is_none());
    assert!(bill.late_fees.is_none() && bill.energy.is_none());
    assert!(bill.items[0].period.is_none());
}

#[test]
fn campos_obrigatorios_nulos_sao_rejeitados() {
    for campo in ["provider_name", "total_cents", "due_date", "items"] {
        let mut v = base();
        v[campo] = Value::Null;
        assert!(parse(&v).is_err(), "{campo}=null deveria falhar");
    }
}

#[test]
fn tipos_errados_sao_rejeitados() {
    let casos = [
        ("total_cents", json!("9990")),
        ("total_cents", json!(99.9)),
        ("total_cents", json!(true)),
        ("provider_name", json!(123)),
        ("items", json!({})),
        ("extraction_confidence", json!("alta")),
        ("kind", json!("Telecom")),
        ("kind", json!("outro")),
        ("due_date", json!("10/11/2026")),
        ("due_date", json!("2026-11-10T00:00:00")),
        ("due_date", json!(20261110)),
    ];
    for (campo, valor) in casos {
        let mut v = base();
        v[campo] = valor.clone();
        assert!(parse(&v).is_err(), "{campo}={valor} deveria falhar");
    }
}

#[test]
fn categoria_inexistente_ou_com_caixa_errada_e_rejeitada() {
    for cat in ["Plano", "PLANO", "plano ", "multa", "", "servico adicional"] {
        let mut v = base();
        v["items"][0]["category"] = json!(cat);
        assert!(parse(&v).is_err(), "categoria {cat:?} deveria falhar");
    }
}

#[test]
fn campos_pessoais_extras_sao_descartados_e_nao_aparecem_no_resultado() {
    // RNF04: mesmo que a IA devolva dados pessoais por conta própria,
    // eles não podem sobreviver ao parse.
    let mut v = base();
    v["holder_name"] = json!("Fulano de Tal");
    v["cpf"] = json!("000.000.000-00");
    v["address"] = json!("Rua Fictícia, 1");
    v["items"][0]["customer_phone"] = json!("(00) 90000-0000");
    let bill = parse(&v).unwrap();
    let saida = serde_json::to_string(&bill).unwrap();
    for proibido in [
        "Fulano",
        "000.000.000-00",
        "Rua Fictícia",
        "90000-0000",
        "cpf",
    ] {
        assert!(!saida.contains(proibido), "vazou {proibido}");
    }
}

// ---------- itens vazios, zeros, negativos ----------

#[test]
fn lista_de_itens_vazia_gera_no_items() {
    let mut v = base();
    v["items"] = json!([]);
    v["total_cents"] = json!(0);
    assert_eq!(parse(&v).unwrap_err(), vec![ValidationIssue::NoItems]);
}

#[test]
fn sem_itens_e_com_total_positivo_nao_gera_soma_divergente() {
    // Sem itens só aparece NoItems; SumMismatch exige ao menos um item.
    let mut v = base();
    v["items"] = json!([]);
    assert_eq!(parse(&v).unwrap_err(), vec![ValidationIssue::NoItems]);
}

#[test]
fn total_zero_com_itens_que_se_anulam_e_aceito() {
    let mut v = base();
    v["total_cents"] = json!(0);
    v["items"] = json!([item("i1", 5000), item("i2", -5000)]);
    assert_eq!(parse(&v).unwrap().total_cents, 0);
}

#[test]
fn item_com_valor_zero_e_aceito() {
    let mut v = base();
    v["items"] = json!([item("i1", 9990), item("i2", 0)]);
    assert_eq!(parse(&v).unwrap().items.len(), 2);
}

#[test]
fn desconto_negativo_entra_na_soma() {
    let mut v = base();
    v["total_cents"] = json!(8990);
    v["items"] = json!([item("i1", 9990), item("i2", -1000)]);
    assert!(parse(&v).is_ok());
}

#[test]
fn total_negativo_gera_negative_total() {
    let mut v = base();
    v["total_cents"] = json!(-100);
    v["items"] = json!([item("i1", -100)]);
    assert_eq!(parse(&v).unwrap_err(), vec![ValidationIssue::NegativeTotal]);
}

#[test]
fn total_negativo_com_soma_divergente_acumula_os_dois_problemas() {
    let mut v = base();
    v["total_cents"] = json!(-100);
    let issues = parse(&v).unwrap_err();
    assert!(issues.contains(&ValidationIssue::NegativeTotal));
    assert!(issues.contains(&ValidationIssue::SumMismatch {
        items_cents: 9990,
        total_cents: -100
    }));
}

// ---------- tolerância de arredondamento (R$ 0,05) ----------

#[test]
fn soma_exatamente_na_tolerancia_passa_nos_dois_sentidos() {
    for delta in [5, -5] {
        let mut v = base();
        v["total_cents"] = json!(9990 + delta);
        assert!(parse(&v).is_ok(), "delta {delta}");
    }
}

#[test]
fn soma_um_centavo_alem_da_tolerancia_falha_nos_dois_sentidos() {
    for delta in [6, -6] {
        let mut v = base();
        v["total_cents"] = json!(9990 + delta);
        let issues = parse(&v).unwrap_err();
        assert_eq!(
            issues,
            vec![ValidationIssue::SumMismatch {
                items_cents: 9990,
                total_cents: 9990 + delta
            }],
            "delta {delta}"
        );
    }
}

#[test]
fn centavos_de_varios_itens_somam_sem_perder_arredondamento() {
    // 3 x 33,33 = 99,99; total impresso 100,00 -> diferença de 1 centavo, dentro da tolerância.
    let mut v = base();
    v["total_cents"] = json!(10000);
    v["items"] = json!([item("i1", 3333), item("i2", 3333), item("i3", 3333)]);
    assert!(parse(&v).is_ok());
}

// ---------- limites numéricos ----------

#[test]
fn valores_grandes_porem_somaveis_sao_aceitos() {
    let mut v = base();
    let grande = i64::MAX / 2;
    v["total_cents"] = json!(grande);
    v["items"] = json!([item("i1", grande)]);
    assert_eq!(parse(&v).unwrap().total_cents, grande);
}

#[test]
fn i64_max_em_item_unico_e_total_e_aceito() {
    let mut v = base();
    v["total_cents"] = json!(i64::MAX);
    v["items"] = json!([item("i1", i64::MAX)]);
    assert_eq!(parse(&v).unwrap().total_cents, i64::MAX);
}

#[test]
fn inteiro_fora_do_intervalo_de_i64_e_rejeitado() {
    // 2^63: não cabe em i64 e não pode virar float em silêncio.
    let txt = base().to_string().replace(
        "\"total_cents\":9990",
        "\"total_cents\":9223372036854775808",
    );
    assert!(parse_and_validate(&txt).is_err());
    let txt = base().to_string().replace(
        "\"total_cents\":9990",
        "\"total_cents\":-9223372036854775809",
    );
    assert!(parse_and_validate(&txt).is_err());
}

#[test]
fn page_negativa_ou_decimal_e_rejeitada() {
    for p in [json!(-1), json!(1.5), json!("1"), Value::Null] {
        let mut v = base();
        v["items"][0]["page"] = p.clone();
        assert!(parse(&v).is_err(), "page {p}");
    }
}

#[test]
fn consumo_kwh_negativo_ou_gigante_e_rejeitado() {
    for kwh in [json!(-1), json!(u64::from(u32::MAX) + 1), json!(1.5)] {
        let mut v = energia();
        v["energy"]["consumption_kwh"] = kwh.clone();
        assert!(parse(&v).is_err(), "kwh {kwh}");
    }
}

#[test]
fn consumo_kwh_zero_e_maximo_de_u32_sao_aceitos() {
    for kwh in [0u64, u64::from(u32::MAX)] {
        let mut v = energia();
        v["energy"]["consumption_kwh"] = json!(kwh);
        assert_eq!(
            parse(&v).unwrap().energy.unwrap().consumption_kwh as u64,
            kwh
        );
    }
}

#[test]
fn dias_de_atraso_negativos_sao_rejeitados() {
    let mut v = base();
    v["late_fees"] =
        json!({"base_cents": 100, "fine_cents": 2, "interest_cents": 1, "days_late": -1});
    assert!(parse(&v).is_err());
}

#[test]
fn late_fees_com_zeros_e_aceito() {
    let mut v = base();
    v["late_fees"] = json!({"base_cents": 0, "fine_cents": 0, "interest_cents": 0, "days_late": 0});
    let bill = parse(&v).unwrap();
    assert_eq!(bill.late_fees.unwrap().days_late, 0);
}

#[test]
fn late_fees_incompleto_e_rejeitado() {
    let mut v = base();
    v["late_fees"] = json!({"base_cents": 100});
    assert!(parse(&v).is_err());
}

// ---------- confiança ----------

#[test]
fn confianca_nos_limites_zero_e_um_e_aceita() {
    for c in [json!(0), json!(0.0), json!(1), json!(1.0)] {
        let mut v = base();
        v["extraction_confidence"] = c.clone();
        assert!(parse(&v).is_ok(), "confiança {c}");
    }
}

#[test]
fn confianca_fora_do_intervalo_gera_issue() {
    for c in [json!(-0.01), json!(1.01), json!(2), json!(-1), json!(100)] {
        let mut v = base();
        v["extraction_confidence"] = c.clone();
        assert_eq!(
            parse(&v).unwrap_err(),
            vec![ValidationIssue::ConfidenceOutOfRange],
            "confiança {c}"
        );
    }
}

#[test]
fn confianca_um_mais_epsilon_de_f32_gera_issue() {
    // 1.0000001 é o próximo f32 acima de 1.0.
    let txt = base().to_string().replace(
        "\"extraction_confidence\":0.9",
        "\"extraction_confidence\":1.0000001",
    );
    assert_eq!(
        parse_and_validate(&txt).unwrap_err(),
        vec![ValidationIssue::ConfidenceOutOfRange]
    );
}

#[test]
fn confianca_enorme_nunca_e_aceita() {
    // 1e39 estoura f32 (vira infinito); 1e999 estoura até f64. Qualquer um dos
    // dois desfechos (erro de parse ou issue) é aceitável; aceitar não é.
    for c in ["1e39", "-1e39", "1e999", "-1e999"] {
        let txt = base().to_string().replace(
            "\"extraction_confidence\":0.9",
            &format!("\"extraction_confidence\":{c}"),
        );
        assert!(parse_and_validate(&txt).is_err(), "confiança {c}");
    }
}

// ---------- mês de referência ----------

#[test]
fn mes_de_referencia_invalido_gera_issue() {
    let invalidos = [
        "",
        "2026",
        "2026-1",
        "2026-00",
        "2026-13",
        "2026-99",
        "26-10",
        "2026/10",
        "2026-10-01",
        "2026-1a",
        "abcd-ef",
        " 2026-1",
        "2026-10 ",
        "２０２６-10",
        "2026-１０",
        "2026-é",
        "-2026-1",
        "2026--1",
    ];
    for m in invalidos {
        let mut v = base();
        v["reference_month"] = json!(m);
        assert_eq!(
            parse(&v).unwrap_err(),
            vec![ValidationIssue::BadReferenceMonth],
            "mês {m:?}"
        );
    }
}

#[test]
fn mes_de_referencia_nos_limites_e_aceito() {
    for m in ["2026-01", "2026-12", "0001-01", "9999-12", "2024-02"] {
        let mut v = base();
        v["reference_month"] = json!(m);
        assert!(parse(&v).is_ok(), "mês {m}");
    }
}

// ---------- datas ----------

#[test]
fn datas_de_calendario_impossiveis_sao_rejeitadas() {
    for d in [
        "2026-02-29",
        "2026-02-30",
        "2026-04-31",
        "2026-13-01",
        "2026-00-10",
        "2026-11-00",
    ] {
        let mut v = base();
        v["due_date"] = json!(d);
        assert!(parse(&v).is_err(), "data {d}");
    }
}

#[test]
fn dia_bissexto_valido_e_aceito() {
    let mut v = base();
    v["due_date"] = json!("2028-02-29");
    let bill = parse(&v).unwrap();
    assert_eq!(bill.due_date.to_string(), "2028-02-29");
}

#[test]
fn datas_extremas_validas_sao_aceitas() {
    for d in ["0001-01-01", "9999-12-31", "2026-12-31", "2027-01-01"] {
        let mut v = base();
        v["due_date"] = json!(d);
        v["issue_date"] = json!(d);
        assert!(parse(&v).is_ok(), "data {d}");
    }
}

#[test]
fn periodo_de_item_incompleto_ou_invalido_e_rejeitado() {
    for p in [
        json!({"start": "2026-10-01"}),
        json!({"end": "2026-10-31"}),
        json!({"start": "2026-10-01", "end": "31/10/2026"}),
        json!({"start": "2026-10-01", "end": "2026-02-30"}),
    ] {
        let mut v = base();
        v["items"][0]["period"] = p.clone();
        assert!(parse(&v).is_err(), "período {p}");
    }
}

// ---------- ids ----------

#[test]
fn ids_duplicados_sao_apontados_uma_vez_por_repeticao() {
    let mut v = base();
    v["total_cents"] = json!(300);
    v["items"] = json!([item("i1", 100), item("i1", 100), item("i1", 100)]);
    let issues = parse(&v).unwrap_err();
    let dups: Vec<_> = issues
        .iter()
        .filter(|i| matches!(i, ValidationIssue::DuplicateItemId { .. }))
        .collect();
    assert_eq!(dups.len(), 2, "3 itens com o mesmo id = 2 repetições");
}

#[test]
fn ids_vazios_repetidos_contam_como_duplicados() {
    let mut v = base();
    v["total_cents"] = json!(200);
    v["items"] = json!([item("", 100), item("", 100)]);
    assert!(
        parse(&v)
            .unwrap_err()
            .contains(&ValidationIssue::DuplicateItemId { id: String::new() })
    );
}

#[test]
fn ids_diferentes_so_na_caixa_ou_espacos_nao_sao_duplicados() {
    let mut v = base();
    v["total_cents"] = json!(300);
    v["items"] = json!([item("i1", 100), item("I1", 100), item("i1 ", 100)]);
    assert!(parse(&v).is_ok());
}

// ---------- energia ----------

#[test]
fn energia_sem_bloco_energy_e_rejeitada() {
    let mut v = energia();
    v["energy"] = Value::Null;
    assert_eq!(
        parse(&v).unwrap_err(),
        vec![ValidationIssue::MissingEnergyData]
    );
    v.as_object_mut().unwrap().remove("energy");
    assert_eq!(
        parse(&v).unwrap_err(),
        vec![ValidationIssue::MissingEnergyData]
    );
}

#[test]
fn telecom_com_bloco_energy_nao_e_problema() {
    let mut v = energia();
    v["kind"] = json!("telecom");
    assert_eq!(parse(&v).unwrap().kind, BillKind::Telecom);
}

#[test]
fn leituras_no_mesmo_dia_sao_aceitas() {
    let mut v = energia();
    v["energy"]["reading_prev_date"] = json!("2026-10-28");
    v["energy"]["reading_curr_date"] = json!("2026-10-28");
    assert!(parse(&v).is_ok());
}

#[test]
fn leituras_invertidas_por_um_dia_sao_apontadas() {
    let mut v = energia();
    v["energy"]["reading_prev_date"] = json!("2026-10-29");
    v["energy"]["reading_curr_date"] = json!("2026-10-28");
    assert_eq!(
        parse(&v).unwrap_err(),
        vec![ValidationIssue::ReadingDatesInverted]
    );
}

#[test]
fn leituras_invertidas_tambem_em_fatura_telecom_sao_apontadas() {
    let mut v = energia();
    v["kind"] = json!("telecom");
    v["energy"]["reading_prev_date"] = json!("2026-10-29");
    v["energy"]["reading_curr_date"] = json!("2026-10-28");
    assert_eq!(
        parse(&v).unwrap_err(),
        vec![ValidationIssue::ReadingDatesInverted]
    );
}

#[test]
fn leituras_atravessando_virada_de_ano_e_bissexto_sao_aceitas() {
    let mut v = energia();
    v["energy"]["reading_prev_date"] = json!("2027-12-30");
    v["energy"]["reading_curr_date"] = json!("2028-02-29");
    assert!(parse(&v).is_ok());
}

#[test]
fn tipo_de_leitura_e_bandeira_invalidos_sao_rejeitados() {
    for (campo, valor) in [
        ("reading_type", json!("Real")),
        ("reading_type", json!("calculada")),
        ("tariff_flag", json!("Verde")),
        ("tariff_flag", json!("vermelha")),
        ("tariff_flag", json!("vermelha_3")),
        ("tariff_flag", json!("")),
    ] {
        let mut v = energia();
        v["energy"][campo] = valor.clone();
        assert!(parse(&v).is_err(), "{campo}={valor}");
    }
}

#[test]
fn todas_as_bandeiras_validas_e_nula_sao_aceitas() {
    for f in ["verde", "amarela", "vermelha_1", "vermelha_2", "escassez"] {
        let mut v = energia();
        v["energy"]["tariff_flag"] = json!(f);
        assert!(parse(&v).is_ok(), "bandeira {f}");
    }
    let mut v = energia();
    v["energy"]["tariff_flag"] = Value::Null;
    assert!(parse(&v).unwrap().energy.unwrap().tariff_flag.is_none());
}

#[test]
fn historico_ausente_vazio_ou_com_kwh_zero_e_aceito() {
    let mut v = energia();
    v["energy"].as_object_mut().unwrap().remove("history");
    assert!(parse(&v).unwrap().energy.unwrap().history.is_empty());

    let mut v = energia();
    v["energy"]["history"] = json!([{"month": "2026-09", "kwh": 0}]);
    assert_eq!(parse(&v).unwrap().energy.unwrap().history[0].kwh, 0);
}

#[test]
fn historico_com_kwh_negativo_e_rejeitado() {
    let mut v = energia();
    v["energy"]["history"] = json!([{"month": "2026-09", "kwh": -5}]);
    assert!(parse(&v).is_err());
}

#[test]
fn historico_longo_demais_ainda_e_aceito() {
    // O prompt pede até 12 meses, mas o parse não impõe limite.
    let mut v = energia();
    let pontos: Vec<Value> = (0..500)
        .map(|i| json!({"month": format!("2026-{:02}", i % 12 + 1), "kwh": i}))
        .collect();
    v["energy"]["history"] = Value::Array(pontos);
    assert_eq!(parse(&v).unwrap().energy.unwrap().history.len(), 500);
}

// ---------- vários problemas ao mesmo tempo ----------

#[test]
fn varios_problemas_sao_acumulados_na_ordem_de_deteccao() {
    let mut v = energia();
    v["items"] = json!([item("i1", 100), item("i1", 100)]);
    v["total_cents"] = json!(-1);
    v["reference_month"] = json!("2026-13");
    v["extraction_confidence"] = json!(5);
    v["energy"]["reading_prev_date"] = json!("2026-10-29");
    v["energy"]["reading_curr_date"] = json!("2026-10-28");
    let issues = parse(&v).unwrap_err();
    assert_eq!(
        issues,
        vec![
            ValidationIssue::NegativeTotal,
            ValidationIssue::BadReferenceMonth,
            ValidationIssue::ConfidenceOutOfRange,
            ValidationIssue::SumMismatch {
                items_cents: 200,
                total_cents: -1
            },
            ValidationIssue::DuplicateItemId { id: "i1".into() },
            ValidationIssue::ReadingDatesInverted,
        ]
    );
}

#[test]
fn json_vazio_de_itens_e_energia_ausente_acumulam_no_items_e_missing_energy() {
    let mut v = energia();
    v["items"] = json!([]);
    v["total_cents"] = json!(0);
    v["energy"] = Value::Null;
    assert_eq!(
        parse(&v).unwrap_err(),
        vec![ValidationIssue::NoItems, ValidationIssue::MissingEnergyData]
    );
}

// ---------- Unicode e espaços ----------

#[test]
fn unicode_em_textos_e_preservado() {
    let mut v = base();
    v["provider_name"] = json!("Vivaz Telecom — São João ção ñ 日本語 🚀");
    v["plan_name"] = json!("Plano \u{202e}RTL\u{202c} ZWJ\u{200d}");
    v["items"][0]["original_text"] = json!("PLANO\u{a0}99,90\u{2003}R$");
    v["items"][0]["description"] = json!("Descrição com “aspas” e \"escape\" \\ barra");
    let bill = parse(&v).unwrap();
    assert_eq!(
        bill.provider_name,
        "Vivaz Telecom — São João ção ñ 日本語 🚀"
    );
    assert_eq!(bill.items[0].original_text, "PLANO\u{a0}99,90\u{2003}R$");
    assert!(bill.items[0].description.contains("\"escape\" \\ barra"));
}

#[test]
fn json_com_escapes_unicode_e_pares_substitutos_e_decodificado() {
    let txt = base().to_string().replace("Vivaz Telecom", r"Vivaz ç🚀");
    let bill = parse_and_validate(&txt).unwrap();
    assert_eq!(bill.provider_name, "Vivaz ç🚀");
}

#[test]
fn par_substituto_solitario_no_json_e_rejeitado() {
    let txt = base().to_string().replace("Vivaz Telecom", r"Vivaz \ud83d");
    assert!(parse_and_validate(&txt).is_err());
}

#[test]
fn strings_vazias_nos_textos_sao_aceitas() {
    let mut v = base();
    v["provider_name"] = json!("");
    v["items"][0]["description"] = json!("");
    v["items"][0]["original_text"] = json!("");
    assert!(parse(&v).is_ok());
}

#[test]
fn explicacao_vazia_ou_so_espacos_vira_string_vazia() {
    for e in ["", "   ", "\n\t \u{a0}"] {
        let mut v = base();
        v["items"][0]["plain_explanation"] = json!(e);
        let bill = parse(&v).unwrap();
        assert_eq!(bill.items[0].plain_explanation, "", "explicação {e:?}");
    }
}

#[test]
fn explicacao_com_espacos_e_quebras_e_normalizada() {
    let mut v = base();
    v["items"][0]["plain_explanation"] = json!("  Custo \n do   plano\t\tmensal.  ");
    let bill = parse(&v).unwrap();
    assert_eq!(bill.items[0].plain_explanation, "Custo do plano mensal.");
}

#[test]
fn explicacao_com_exatamente_25_palavras_nao_e_cortada() {
    let e = (1..=25)
        .map(|i| format!("p{i}"))
        .collect::<Vec<_>>()
        .join(" ");
    let mut v = base();
    v["items"][0]["plain_explanation"] = json!(e);
    let bill = parse(&v).unwrap();
    assert_eq!(bill.items[0].plain_explanation, e);
    assert!(!bill.items[0].plain_explanation.ends_with('…'));
}

#[test]
fn explicacao_com_26_palavras_e_cortada_em_25_com_reticencias() {
    let e = (1..=26)
        .map(|i| format!("p{i}"))
        .collect::<Vec<_>>()
        .join(" ");
    let mut v = base();
    v["items"][0]["plain_explanation"] = json!(e);
    let got = parse(&v).unwrap().items[0].plain_explanation.clone();
    assert!(got.ends_with("p25…"), "got {got}");
    assert_eq!(got.split_whitespace().count(), 25);
}

#[test]
fn corte_remove_pontuacao_final_antes_das_reticencias() {
    let mut palavras: Vec<String> = (1..=24).map(|i| format!("p{i}")).collect();
    palavras.push("p25,".into());
    palavras.push("p26".into());
    let mut v = base();
    v["items"][0]["plain_explanation"] = json!(palavras.join(" "));
    let got = parse(&v).unwrap().items[0].plain_explanation.clone();
    assert!(got.ends_with("p25…"), "got {got}");
}

#[test]
fn explicacao_gigante_em_unicode_e_cortada_sem_quebrar_caracteres() {
    let e = "ação ".repeat(100_000);
    let mut v = base();
    v["items"][0]["plain_explanation"] = json!(e);
    let got = parse(&v).unwrap().items[0].plain_explanation.clone();
    assert_eq!(got.split_whitespace().count(), 25);
    assert!(got.ends_with("ação…"));
}

#[test]
fn explicacao_com_juridiques_passa_pelo_parse_hoje() {
    // Documenta o comportamento atual: parse_and_validate só limita o tamanho;
    // o filtro de juridiquês (FORBIDDEN_TERMS) não rejeita nem remove termos.
    let mut v = base();
    v["items"][0]["plain_explanation"] = json!("Valor supramencionado, outrossim devido.");
    let bill = parse(&v).unwrap();
    assert!(!humanobill_core::is_plain_explanation(
        &bill.items[0].plain_explanation
    ));
}

// ---------- volume ----------

#[test]
fn muitos_itens_somam_corretamente() {
    let n = 5_000;
    let mut v = base();
    v["total_cents"] = json!(n * 10);
    v["items"] = Value::Array((0..n).map(|i| item(&format!("i{i}"), 10)).collect());
    assert_eq!(parse(&v).unwrap().items.len(), n as usize);
}

#[test]
fn texto_original_muito_longo_e_aceito() {
    let mut v = base();
    v["items"][0]["original_text"] = json!("X".repeat(2_000_000));
    assert_eq!(parse(&v).unwrap().items[0].original_text.len(), 2_000_000);
}

// ---------- BUGS conhecidos (ignorados) ----------

#[test]
fn soma_de_itens_que_estoura_i64_nao_deve_dar_panic() {
    let mut v = base();
    v["total_cents"] = json!(1);
    v["items"] = json!([item("i1", i64::MAX), item("i2", 1)]);
    // Deve devolver Err(issues) (soma divergente), nunca derrubar o processo.
    assert!(parse(&v).is_err());
}

#[test]
fn total_i64_min_nao_deve_dar_panic() {
    let mut v = base();
    v["total_cents"] = json!(i64::MIN);
    v["items"] = json!([item("i1", 0)]);
    let issues = parse(&v).unwrap_err();
    assert!(issues.contains(&ValidationIssue::NegativeTotal));
}

#[test]
fn pagina_zero_deveria_ser_rejeitada() {
    let mut v = base();
    v["items"][0]["page"] = json!(0);
    assert!(parse(&v).is_err());
}

#[test]
fn periodo_invertido_deveria_ser_rejeitado() {
    let mut v = base();
    v["items"][0]["period"] = json!({"start": "2026-10-31", "end": "2026-10-01"});
    assert!(parse(&v).is_err());
}
