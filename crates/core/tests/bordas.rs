//! Testes de borda do núcleo (`humanobill-core`): explicações, validação de
//! consistência, (des)serialização JSON e caminhos de falha.
//!
//! Só dados sintéticos (Vivaz Telecom / Luzia Energia). Nada de rede.

use std::collections::BTreeMap;

use chrono::NaiveDate;
use humanobill_core::*;
use serde_json::{Value, json};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn data(a: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(a, m, d).unwrap()
}

fn item(id: &str, cents: Cents) -> BillItem {
    BillItem {
        id: id.into(),
        description: format!("Item {id}"),
        amount_cents: cents,
        category: ItemCategory::Outros,
        period: None,
        original_text: format!("ITEM {id}"),
        page: 1,
        plain_explanation: "Cobrança de exemplo.".into(),
    }
}

/// Fatura telecom consistente: 99,90 + 12,90 + 10,00 = 122,80.
fn fatura() -> ExtractedBill {
    ExtractedBill {
        kind: BillKind::Telecom,
        provider_name: "Vivaz Telecom".into(),
        reference_month: "2026-10".into(),
        due_date: data(2026, 11, 10),
        issue_date: None,
        total_cents: 12_280,
        plan_name: None,
        items: vec![item("i1", 9_990), item("i2", 1_290), item("i3", 1_000)],
        late_fees: None,
        energy: None,
        extraction_confidence: 0.9,
    }
}

fn energia(prev: NaiveDate, curr: NaiveDate) -> EnergyData {
    EnergyData {
        consumption_kwh: 210,
        reading_type: ReadingType::Real,
        reading_prev_date: prev,
        reading_curr_date: curr,
        tariff_flag: None,
        history: vec![],
    }
}

/// Fatura mínima em JSON (só campos obrigatórios).
fn json_minimo() -> Value {
    json!({
        "kind": "telecom",
        "provider_name": "Vivaz Telecom",
        "reference_month": "2026-10",
        "due_date": "2026-11-10",
        "total_cents": 1000,
        "items": [{
            "id": "i1",
            "description": "Plano",
            "amount_cents": 1000,
            "category": "plano",
            "original_text": "PLANO 10,00",
            "page": 1,
            "plain_explanation": "Mensalidade."
        }],
        "extraction_confidence": 0.5
    })
}

fn palavras(n: usize) -> String {
    (1..=n)
        .map(|i| format!("p{i}"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn analise() -> Analysis {
    Analysis {
        id: "a1".into(),
        created_at: "2026-11-17T10:00:00Z".into(),
        expires_at: "2026-12-17T10:00:00Z".into(),
        source: AnalysisSource::Demo,
        bill: fatura(),
        answers: Answers::new(),
        paid: None,
        findings: vec![],
        questions: vec![],
        recoverable_cents: 0,
        rules_version: "0.1.0".into(),
    }
}

// ---------------------------------------------------------------------------
// is_plain_explanation / tidy_explanation
// ---------------------------------------------------------------------------

#[test]
fn explicacao_vazia_e_so_espacos_e_simples() {
    assert!(is_plain_explanation(""));
    assert!(is_plain_explanation("   \t\n  "));
}

#[test]
fn explicacao_com_exatamente_25_palavras_passa_e_26_falha() {
    assert!(is_plain_explanation(&palavras(25)));
    assert!(!is_plain_explanation(&palavras(26)));
}

#[test]
fn explicacao_conta_palavras_ignorando_espacos_repetidos_e_quebras() {
    // 25 palavras separadas por espaços variados continuam sendo 25.
    let s = (1..=25)
        .map(|i| format!("p{i}"))
        .collect::<Vec<_>>()
        .join(" \t\n  ");
    assert!(is_plain_explanation(&s));
}

#[test]
fn explicacao_espaco_inseparavel_unicode_separa_palavras() {
    // U+00A0 (NBSP) e U+3000 (espaço ideográfico) são whitespace Unicode.
    let s = format!("{}\u{00A0}{}\u{3000}p26", palavras(24), "p25");
    assert_eq!(s.split_whitespace().count(), 26);
    assert!(!is_plain_explanation(&s));
}

#[test]
fn juridiques_e_detectado_ignorando_maiusculas_e_acentos_de_caixa() {
    assert!(!is_plain_explanation("SUPRAMENCIONADO valor."));
    assert!(!is_plain_explanation("Isto Ensejará multa."));
    assert!(!is_plain_explanation("DESTARTE, pague."));
    assert!(!is_plain_explanation("Outrossim, o valor."));
    assert!(!is_plain_explanation("Doravante será cobrado."));
}

#[test]
fn juridiques_e_detectado_mesmo_colado_em_outra_palavra() {
    // Busca por substring: "ensejar" dentro de outra palavra também reprova.
    assert!(!is_plain_explanation("xxensejarxx"));
}

#[test]
fn juridiques_com_hifen_ou_espaco_no_meio_nao_e_detectado() {
    // Documenta o limite da checagem: o termo quebrado não bate.
    assert!(is_plain_explanation("supra mencionado"));
    assert!(is_plain_explanation("out-rossim"));
}

#[test]
fn explicacao_curta_com_juridiques_falha_mesmo_dentro_do_limite() {
    assert!(!is_plain_explanation("Destarte."));
}

#[test]
fn tidy_vazio_e_so_espacos_vira_string_vazia() {
    assert_eq!(tidy_explanation(""), "");
    assert_eq!(tidy_explanation("   \n\t "), "");
}

#[test]
fn tidy_com_exatamente_25_palavras_nao_corta_nem_poe_reticencias() {
    let s = palavras(25);
    assert_eq!(tidy_explanation(&s), s);
}

#[test]
fn tidy_com_26_palavras_corta_e_poe_reticencias() {
    let t = tidy_explanation(&palavras(26));
    assert!(t.ends_with("p25…"), "saída: {t}");
    assert!(!t.contains("p26"));
}

#[test]
fn tidy_remove_pontuacao_final_antes_das_reticencias() {
    let mut p: Vec<String> = (1..=24).map(|i| format!("p{i}")).collect();
    p.push("fim;".into());
    p.push("sobra".into());
    let t = tidy_explanation(&p.join(" "));
    assert!(t.ends_with("p24 fim…"), "saída: {t}");
}

#[test]
fn tidy_remove_varias_pontuacoes_seguidas() {
    let mut p: Vec<String> = (1..=24).map(|i| format!("p{i}")).collect();
    p.push("fim,;:.".into());
    p.push("sobra".into());
    let t = tidy_explanation(&p.join(" "));
    assert!(t.ends_with("p24 fim…"), "saída: {t}");
}

#[test]
#[ignore = "bug: tidy_explanation deixa espaço antes de '…' quando a 25ª palavra é só pontuação (ex.: 'p24 …')"]
fn tidy_25a_palavra_so_pontuacao_nao_deixa_espaco_sobrando_antes_das_reticencias() {
    // A 25ª "palavra" é só pontuação: sobra "p24 " + "…"? Aqui o trim só tira
    // pontuação, não espaço — o resultado pode ficar com espaço antes do "…".
    let mut p: Vec<String> = (1..=24).map(|i| format!("p{i}")).collect();
    p.push("...".into());
    p.push("sobra".into());
    let t = tidy_explanation(&p.join(" "));
    assert!(t.ends_with('…'));
    assert!(
        !t.ends_with(" …"),
        "espaço indevido antes das reticências: {t:?}"
    );
}

#[test]
fn tidy_nao_remove_ponto_de_exclamacao_nem_interrogacao() {
    let mut p: Vec<String> = (1..=24).map(|i| format!("p{i}")).collect();
    p.push("acabou!".into());
    p.push("sobra".into());
    assert!(tidy_explanation(&p.join(" ")).ends_with("acabou!…"));
}

#[test]
fn tidy_normaliza_espacos_e_resultado_e_idempotente() {
    let t = tidy_explanation("  a \t b\n\nc  ");
    assert_eq!(t, "a b c");
    assert_eq!(tidy_explanation(&t), t);
}

#[test]
fn tidy_com_unicode_nao_corta_no_meio_do_caractere() {
    let longa = (1..=40).map(|_| "ação").collect::<Vec<_>>().join(" ");
    let t = tidy_explanation(&longa);
    assert_eq!(t.split_whitespace().count(), MAX_EXPLANATION_WORDS);
    assert!(t.ends_with("ação…"));
}

#[test]
fn tidy_resultado_truncado_continua_sendo_simples() {
    // Truncado: 25 palavras + "…" colado na última. Deve passar na checagem.
    let t = tidy_explanation(&palavras(500));
    assert!(is_plain_explanation(&t));
}

#[test]
fn tidy_nao_remove_juridiques_so_corta() {
    // tidy não filtra termos proibidos: quem decide é is_plain_explanation.
    let t = tidy_explanation("Destarte o valor.");
    assert!(!is_plain_explanation(&t));
}

// ---------------------------------------------------------------------------
// validate_bill — tolerância da soma
// ---------------------------------------------------------------------------

#[test]
fn soma_diferenca_de_exatamente_tolerancia_passa_nos_dois_sentidos() {
    let mut b = fatura();
    b.total_cents += SUM_TOLERANCE_CENTS;
    assert!(validate_bill(&b).is_empty());
    b.total_cents -= 2 * SUM_TOLERANCE_CENTS;
    assert!(validate_bill(&b).is_empty());
}

#[test]
fn soma_um_centavo_acima_da_tolerancia_falha_nos_dois_sentidos() {
    let mut b = fatura();
    b.total_cents += SUM_TOLERANCE_CENTS + 1;
    assert!(matches!(
        validate_bill(&b)[..],
        [ValidationIssue::SumMismatch { .. }]
    ));
    b.total_cents -= 2 * (SUM_TOLERANCE_CENTS + 1);
    assert!(matches!(
        validate_bill(&b)[..],
        [ValidationIssue::SumMismatch { .. }]
    ));
}

#[test]
fn soma_divergente_reporta_valores_corretos() {
    let mut b = fatura();
    b.total_cents = 20_000;
    assert_eq!(
        validate_bill(&b),
        vec![ValidationIssue::SumMismatch {
            items_cents: 12_280,
            total_cents: 20_000
        }]
    );
}

#[test]
fn soma_com_item_negativo_desconto_e_consistente() {
    let mut b = fatura();
    b.items.push(item("i4", -500));
    b.total_cents = 11_780;
    assert!(validate_bill(&b).is_empty());
}

#[test]
fn soma_com_itens_zerados_e_total_zero_e_consistente() {
    let mut b = fatura();
    b.items = vec![item("i1", 0), item("i2", 0)];
    b.total_cents = 0;
    assert!(validate_bill(&b).is_empty());
}

#[test]
fn sem_itens_so_aponta_no_items_sem_soma_divergente() {
    let mut b = fatura();
    b.items.clear();
    b.total_cents = 12_280;
    assert_eq!(validate_bill(&b), vec![ValidationIssue::NoItems]);
    b.total_cents = 0;
    assert_eq!(validate_bill(&b), vec![ValidationIssue::NoItems]);
}

#[test]
fn total_negativo_e_apontado() {
    let mut b = fatura();
    b.items = vec![item("i1", -1_000)];
    b.total_cents = -1_000;
    assert_eq!(validate_bill(&b), vec![ValidationIssue::NegativeTotal]);
}

#[test]
fn total_menos_um_centavo_e_negativo_e_zero_nao() {
    let mut b = fatura();
    b.items = vec![item("i1", 0)];
    b.total_cents = 0;
    assert!(validate_bill(&b).is_empty());
    b.items = vec![item("i1", -1)];
    b.total_cents = -1;
    assert_eq!(validate_bill(&b), vec![ValidationIssue::NegativeTotal]);
}

#[test]
fn valores_grandes_sem_estourar_sao_validos() {
    let mut b = fatura();
    b.items = vec![item("i1", i64::MAX / 2), item("i2", i64::MAX / 2)];
    b.total_cents = (i64::MAX / 2) * 2;
    assert!(validate_bill(&b).is_empty());
}

#[test]
fn soma_de_itens_que_estoura_i64_nao_deveria_dar_panico() {
    // Entrada vinda da IA pode ter valores absurdos; a validação deve apontar
    // o problema, e não derrubar a API.
    let mut b = fatura();
    b.items = vec![item("i1", i64::MAX), item("i2", 1)];
    b.total_cents = 1;
    let r = std::panic::catch_unwind(|| validate_bill(&b));
    assert!(
        r.is_ok(),
        "validate_bill entrou em pânico (overflow na soma)"
    );
}

#[test]
fn total_i64_min_nao_deveria_dar_panico() {
    let mut b = fatura();
    b.total_cents = i64::MIN;
    let r = std::panic::catch_unwind(|| validate_bill(&b));
    assert!(
        r.is_ok(),
        "validate_bill entrou em pânico (overflow em items - total / abs)"
    );
}

// ---------------------------------------------------------------------------
// validate_bill — mês de referência
// ---------------------------------------------------------------------------

fn mes_e_valido(mes: &str) -> bool {
    let mut b = fatura();
    b.reference_month = mes.into();
    let issues = validate_bill(&b);
    assert!(
        issues.is_empty() || issues == vec![ValidationIssue::BadReferenceMonth],
        "inesperado para {mes:?}: {issues:?}"
    );
    issues.is_empty()
}

#[test]
fn mes_de_referencia_aceita_limites_01_e_12() {
    assert!(mes_e_valido("2026-01"));
    assert!(mes_e_valido("2026-12"));
    assert!(mes_e_valido("0000-01"));
    assert!(mes_e_valido("9999-12"));
}

#[test]
fn mes_de_referencia_rejeita_00_13_e_99() {
    assert!(!mes_e_valido("2026-00"));
    assert!(!mes_e_valido("2026-13"));
    assert!(!mes_e_valido("2026-99"));
}

#[test]
fn mes_de_referencia_rejeita_formatos_malformados() {
    for ruim in [
        "",
        "2026",
        "2026-1",
        "2026-010",
        "26-10",
        "2026/10",
        "2026_10",
        "202610-",
        "2026-1x",
        "abcd-10",
        " 2026-10",
        "2026-10 ",
        "2026-10\n",
        "-2026-1",
        "+026-10",
        "2026--1",
        "2026-+1",
        "2026-10-01",
    ] {
        assert!(!mes_e_valido(ruim), "deveria rejeitar {ruim:?}");
    }
}

#[test]
fn mes_de_referencia_rejeita_unicode_sem_entrar_em_panico() {
    // Dígitos de largura total, acentos e emoji: nunca podem causar panic de
    // fatiamento em fronteira de char.
    for ruim in [
        "２０２６-１０",
        "2026-１０",
        "é2026-1",
        "202é-10",
        "2026-é",
        "2026-1é",
        "20\u{1F600}-1",
        "٢٠٢٦-١٠",
    ] {
        assert!(!mes_e_valido(ruim), "deveria rejeitar {ruim:?}");
    }
}

// ---------------------------------------------------------------------------
// validate_bill — confiança
// ---------------------------------------------------------------------------

fn confianca_fora(c: f32) -> bool {
    let mut b = fatura();
    b.extraction_confidence = c;
    validate_bill(&b).contains(&ValidationIssue::ConfidenceOutOfRange)
}

#[test]
fn confianca_limites_0_e_1_sao_validos() {
    assert!(!confianca_fora(0.0));
    assert!(!confianca_fora(-0.0));
    assert!(!confianca_fora(1.0));
    assert!(!confianca_fora(f32::MIN_POSITIVE));
}

#[test]
fn confianca_logo_fora_do_intervalo_e_apontada() {
    assert!(confianca_fora(1.000_000_1));
    assert!(confianca_fora(-0.000_000_1));
    assert!(confianca_fora(-1.0));
    assert!(confianca_fora(100.0));
}

#[test]
fn confianca_nan_e_infinito_sao_apontados() {
    assert!(confianca_fora(f32::NAN));
    assert!(confianca_fora(f32::INFINITY));
    assert!(confianca_fora(f32::NEG_INFINITY));
    assert!(confianca_fora(f32::MAX));
    assert!(confianca_fora(f32::MIN));
}

// ---------------------------------------------------------------------------
// validate_bill — ids duplicados
// ---------------------------------------------------------------------------

#[test]
fn id_triplicado_gera_dois_avisos() {
    let mut b = fatura();
    b.items = vec![item("x", 100), item("x", 100), item("x", 100)];
    b.total_cents = 300;
    let dups = validate_bill(&b)
        .into_iter()
        .filter(|i| matches!(i, ValidationIssue::DuplicateItemId { id } if id == "x"))
        .count();
    assert_eq!(dups, 2);
}

#[test]
fn ids_diferem_por_caixa_e_por_espaco_nao_sao_duplicados() {
    let mut b = fatura();
    b.items = vec![item("i1", 100), item("I1", 100), item("i1 ", 100)];
    b.total_cents = 300;
    assert!(validate_bill(&b).is_empty());
}

#[test]
fn ids_vazios_repetidos_sao_duplicados() {
    let mut b = fatura();
    b.items = vec![item("", 100), item("", 100)];
    b.total_cents = 200;
    assert_eq!(
        validate_bill(&b),
        vec![ValidationIssue::DuplicateItemId { id: "".into() }]
    );
}

#[test]
fn ids_unicode_iguais_sao_duplicados() {
    let mut b = fatura();
    b.items = vec![item("ítem-ç", 100), item("ítem-ç", 100)];
    b.total_cents = 200;
    assert_eq!(
        validate_bill(&b),
        vec![ValidationIssue::DuplicateItemId {
            id: "ítem-ç".into()
        }]
    );
}

// ---------------------------------------------------------------------------
// validate_bill — energia e datas de leitura
// ---------------------------------------------------------------------------

#[test]
fn energia_sem_bloco_energy_nao_acusa_datas_invertidas() {
    let mut b = fatura();
    b.kind = BillKind::Energy;
    assert_eq!(validate_bill(&b), vec![ValidationIssue::MissingEnergyData]);
}

#[test]
fn leituras_no_mesmo_dia_sao_validas() {
    let mut b = fatura();
    b.kind = BillKind::Energy;
    b.energy = Some(energia(data(2026, 10, 5), data(2026, 10, 5)));
    assert!(validate_bill(&b).is_empty());
}

#[test]
fn leitura_atual_um_dia_antes_da_anterior_e_apontada() {
    let mut b = fatura();
    b.kind = BillKind::Energy;
    b.energy = Some(energia(data(2026, 10, 5), data(2026, 10, 4)));
    assert_eq!(
        validate_bill(&b),
        vec![ValidationIssue::ReadingDatesInverted]
    );
}

#[test]
fn leituras_atravessando_ano_bissexto_e_virada_de_ano() {
    let mut b = fatura();
    b.kind = BillKind::Energy;
    b.energy = Some(energia(data(2023, 12, 31), data(2024, 2, 29)));
    assert!(validate_bill(&b).is_empty());
    b.energy = Some(energia(data(2024, 1, 1), data(2023, 12, 31)));
    assert_eq!(
        validate_bill(&b),
        vec![ValidationIssue::ReadingDatesInverted]
    );
}

#[test]
fn fatura_telecom_com_bloco_energy_invertido_tambem_e_apontada() {
    let mut b = fatura();
    b.energy = Some(energia(data(2026, 10, 5), data(2026, 9, 5)));
    assert_eq!(
        validate_bill(&b),
        vec![ValidationIssue::ReadingDatesInverted]
    );
}

#[test]
fn fatura_telecom_com_bloco_energy_consistente_passa() {
    let mut b = fatura();
    b.energy = Some(energia(data(2026, 9, 5), data(2026, 10, 5)));
    assert!(validate_bill(&b).is_empty());
}

#[test]
fn datas_extremas_do_chrono_nao_dao_panico() {
    let mut b = fatura();
    b.kind = BillKind::Energy;
    b.energy = Some(energia(NaiveDate::MIN, NaiveDate::MAX));
    assert!(validate_bill(&b).is_empty());
    b.energy = Some(energia(NaiveDate::MAX, NaiveDate::MIN));
    assert_eq!(
        validate_bill(&b),
        vec![ValidationIssue::ReadingDatesInverted]
    );
}

// ---------------------------------------------------------------------------
// validate_bill — vários problemas ao mesmo tempo
// ---------------------------------------------------------------------------

#[test]
fn varios_problemas_vem_na_ordem_de_verificacao() {
    let mut b = fatura();
    b.items = vec![item("a", 100), item("a", 100)];
    b.total_cents = -50;
    b.reference_month = "xx".into();
    b.extraction_confidence = 2.0;
    b.kind = BillKind::Energy;
    let issues = validate_bill(&b);
    assert_eq!(
        issues,
        vec![
            ValidationIssue::NegativeTotal,
            ValidationIssue::BadReferenceMonth,
            ValidationIssue::ConfidenceOutOfRange,
            ValidationIssue::SumMismatch {
                items_cents: 200,
                total_cents: -50
            },
            ValidationIssue::DuplicateItemId { id: "a".into() },
            ValidationIssue::MissingEnergyData,
        ]
    );
}

#[test]
fn fatura_totalmente_vazia_acumula_problemas() {
    let mut b = fatura();
    b.items.clear();
    b.total_cents = -1;
    b.reference_month.clear();
    let issues = validate_bill(&b);
    assert_eq!(
        issues,
        vec![
            ValidationIssue::NoItems,
            ValidationIssue::NegativeTotal,
            ValidationIssue::BadReferenceMonth,
        ]
    );
}

#[test]
fn validate_bill_nao_altera_a_fatura_e_e_deterministico() {
    let b = fatura();
    let antes = b.clone();
    assert_eq!(validate_bill(&b), validate_bill(&b));
    assert_eq!(b, antes);
}

// ---------------------------------------------------------------------------
// ValidationIssue: formato JSON
// ---------------------------------------------------------------------------

#[test]
fn validation_issue_serializa_com_tag_code_em_snake_case() {
    assert_eq!(
        serde_json::to_value(ValidationIssue::NoItems).unwrap(),
        json!({"code": "no_items"})
    );
    assert_eq!(
        serde_json::to_value(ValidationIssue::SumMismatch {
            items_cents: 1,
            total_cents: -2
        })
        .unwrap(),
        json!({"code": "sum_mismatch", "items_cents": 1, "total_cents": -2})
    );
    assert_eq!(
        serde_json::to_value(ValidationIssue::DuplicateItemId { id: "é".into() }).unwrap(),
        json!({"code": "duplicate_item_id", "id": "é"})
    );
}

#[test]
fn validation_issue_variantes_unitarias_roundtrip() {
    for v in [
        ValidationIssue::NoItems,
        ValidationIssue::NegativeTotal,
        ValidationIssue::BadReferenceMonth,
        ValidationIssue::MissingEnergyData,
        ValidationIssue::ReadingDatesInverted,
        ValidationIssue::ConfidenceOutOfRange,
    ] {
        let s = serde_json::to_string(&v).unwrap();
        assert_eq!(serde_json::from_str::<ValidationIssue>(&s).unwrap(), v);
    }
}

#[test]
fn validation_issue_codigo_desconhecido_ou_ausente_falha() {
    assert!(serde_json::from_value::<ValidationIssue>(json!({"code": "nao_existe"})).is_err());
    assert!(serde_json::from_value::<ValidationIssue>(json!({})).is_err());
    assert!(serde_json::from_value::<ValidationIssue>(json!({"code": "sum_mismatch"})).is_err());
}

// ---------------------------------------------------------------------------
// ExtractedBill — JSON: campos ausentes, tipos errados, limites
// ---------------------------------------------------------------------------

#[test]
fn fatura_minima_desserializa_com_opcionais_none() {
    let b: ExtractedBill = serde_json::from_value(json_minimo()).unwrap();
    assert_eq!(b.issue_date, None);
    assert_eq!(b.plan_name, None);
    assert_eq!(b.late_fees, None);
    assert_eq!(b.energy, None);
    assert_eq!(b.items[0].period, None);
    assert!(validate_bill(&b).is_empty());
}

#[test]
fn campos_opcionais_aceitam_null_explicito() {
    let mut v = json_minimo();
    v["issue_date"] = Value::Null;
    v["plan_name"] = Value::Null;
    v["late_fees"] = Value::Null;
    v["energy"] = Value::Null;
    v["items"][0]["period"] = Value::Null;
    assert!(serde_json::from_value::<ExtractedBill>(v).is_ok());
}

#[test]
fn cada_campo_obrigatorio_ausente_falha() {
    for campo in [
        "kind",
        "provider_name",
        "reference_month",
        "due_date",
        "total_cents",
        "items",
        "extraction_confidence",
    ] {
        let mut v = json_minimo();
        v.as_object_mut().unwrap().remove(campo);
        assert!(
            serde_json::from_value::<ExtractedBill>(v).is_err(),
            "faltando {campo} deveria falhar"
        );
    }
}

#[test]
fn cada_campo_obrigatorio_do_item_ausente_falha() {
    for campo in [
        "id",
        "description",
        "amount_cents",
        "category",
        "original_text",
        "page",
        "plain_explanation",
    ] {
        let mut v = json_minimo();
        v["items"][0].as_object_mut().unwrap().remove(campo);
        assert!(
            serde_json::from_value::<ExtractedBill>(v).is_err(),
            "item sem {campo} deveria falhar"
        );
    }
}

#[test]
fn campos_obrigatorios_null_falham() {
    for campo in [
        "kind",
        "due_date",
        "total_cents",
        "items",
        "extraction_confidence",
    ] {
        let mut v = json_minimo();
        v[campo] = Value::Null;
        assert!(
            serde_json::from_value::<ExtractedBill>(v).is_err(),
            "{campo}=null deveria falhar"
        );
    }
}

#[test]
fn json_malformado_falha_sem_panico() {
    for ruim in [
        "",
        "{",
        "}",
        "null",
        "[]",
        "42",
        "\"texto\"",
        "{\"kind\":}",
        "{\"kind\":\"telecom\",}",
        "{'kind':'telecom'}",
        "```json\n{}\n```",
    ] {
        assert!(
            serde_json::from_str::<ExtractedBill>(ruim).is_err(),
            "deveria falhar: {ruim:?}"
        );
    }
}

#[test]
fn campo_desconhecido_e_ignorado_e_nao_volta_na_saida() {
    // RNF04: se a IA devolver um campo de dado pessoal por conta própria,
    // ele é descartado na desserialização e não é reemitido.
    let mut v = json_minimo();
    v["cpf"] = json!("000.000.000-00");
    v["holder_name"] = json!("Fulano de Tal");
    v["items"][0]["endereco"] = json!("Rua Fictícia, 1");
    let b: ExtractedBill = serde_json::from_value(v).unwrap();
    let saida = serde_json::to_string(&b).unwrap();
    assert!(!saida.contains("000.000"));
    assert!(!saida.contains("Fulano"));
    assert!(!saida.contains("Rua Fict"));
}

#[test]
fn categoria_tipo_e_bandeira_desconhecidos_falham() {
    let mut v = json_minimo();
    v["kind"] = json!("agua");
    assert!(serde_json::from_value::<ExtractedBill>(v).is_err());

    let mut v = json_minimo();
    v["items"][0]["category"] = json!("Plano"); // caixa errada
    assert!(serde_json::from_value::<ExtractedBill>(v).is_err());

    let mut v = json_minimo();
    v["items"][0]["category"] = json!("iluminacao-publica"); // hífen em vez de _
    assert!(serde_json::from_value::<ExtractedBill>(v).is_err());
}

#[test]
fn todas_as_categorias_fazem_roundtrip_em_snake_case() {
    let todas = [
        (ItemCategory::Plano, "plano"),
        (ItemCategory::ServicoAdicional, "servico_adicional"),
        (ItemCategory::EncargoAtraso, "encargo_atraso"),
        (ItemCategory::Imposto, "imposto"),
        (ItemCategory::Energia, "energia"),
        (ItemCategory::Bandeira, "bandeira"),
        (ItemCategory::IluminacaoPublica, "iluminacao_publica"),
        (ItemCategory::Outros, "outros"),
    ];
    for (cat, nome) in todas {
        assert_eq!(serde_json::to_value(cat).unwrap(), json!(nome));
        assert_eq!(
            serde_json::from_value::<ItemCategory>(json!(nome)).unwrap(),
            cat
        );
    }
}

#[test]
fn centavos_aceitam_extremos_i64_e_rejeitam_alem_do_limite() {
    let mut v = json_minimo();
    v["total_cents"] = json!(i64::MAX);
    assert_eq!(
        serde_json::from_value::<ExtractedBill>(v)
            .unwrap()
            .total_cents,
        i64::MAX
    );
    let mut v = json_minimo();
    v["total_cents"] = json!(i64::MIN);
    assert_eq!(
        serde_json::from_value::<ExtractedBill>(v)
            .unwrap()
            .total_cents,
        i64::MIN
    );
    // i64::MAX + 1 como texto JSON cru (não cabe em i64).
    let txt = serde_json::to_string(&json_minimo()).unwrap().replace(
        "\"total_cents\":1000",
        "\"total_cents\":9223372036854775808",
    );
    assert!(serde_json::from_str::<ExtractedBill>(&txt).is_err());
}

#[test]
fn centavos_fracionarios_ou_notacao_cientifica_falham() {
    // Dinheiro nunca é float: 99.9 e 1e3 não são aceitos como Cents.
    for valor in ["99.9", "1e3", "1000.0", "-0.5"] {
        let txt = serde_json::to_string(&json_minimo())
            .unwrap()
            .replace("\"total_cents\":1000", &format!("\"total_cents\":{valor}"));
        assert!(
            serde_json::from_str::<ExtractedBill>(&txt).is_err(),
            "total_cents={valor} deveria falhar"
        );
    }
}

#[test]
fn centavos_como_string_falham() {
    let mut v = json_minimo();
    v["total_cents"] = json!("1000");
    assert!(serde_json::from_value::<ExtractedBill>(v).is_err());
}

#[test]
fn pagina_negativa_ou_acima_de_u32_falha_e_zero_e_aceita() {
    for ruim in [json!(-1), json!(4_294_967_296_u64), json!(1.5), json!("1")] {
        let mut v = json_minimo();
        v["items"][0]["page"] = ruim.clone();
        assert!(
            serde_json::from_value::<ExtractedBill>(v).is_err(),
            "page={ruim} deveria falhar"
        );
    }
    let mut v = json_minimo();
    v["items"][0]["page"] = json!(u32::MAX);
    assert!(serde_json::from_value::<ExtractedBill>(v).is_ok());
    // Documenta: página 0 (o doc diz "1-based") desserializa sem reclamar.
    let mut v = json_minimo();
    v["items"][0]["page"] = json!(0);
    assert!(serde_json::from_value::<ExtractedBill>(v).is_ok());
}

#[test]
fn datas_invalidas_falham() {
    for ruim in [
        "2026-02-30",
        "2026-02-29", // 2026 não é bissexto
        "2026-13-01",
        "2026-00-10",
        "2026-11-31",
        "10/11/2026",
        "",
        "ontem",
        "2026-11-10T00:00:00Z",
    ] {
        let mut v = json_minimo();
        v["due_date"] = json!(ruim);
        assert!(
            serde_json::from_value::<ExtractedBill>(v).is_err(),
            "due_date={ruim:?} deveria falhar"
        );
    }
}

#[test]
fn data_sem_zero_a_esquerda_e_aceita_pelo_chrono() {
    // Documenta: o parser do chrono é leniente com "2026-1-1".
    let mut v = json_minimo();
    v["due_date"] = json!("2026-1-1");
    let b: ExtractedBill = serde_json::from_value(v).unwrap();
    assert_eq!(b.due_date, data(2026, 1, 1));
}

#[test]
fn datas_limite_validas_29_de_fevereiro_bissexto() {
    let mut v = json_minimo();
    v["due_date"] = json!("2024-02-29");
    let b: ExtractedBill = serde_json::from_value(v).unwrap();
    assert_eq!(b.due_date, data(2024, 2, 29));
    let mut v = json_minimo();
    v["due_date"] = json!("2000-02-29"); // divisível por 400 → bissexto
    assert!(serde_json::from_value::<ExtractedBill>(v).is_ok());
    let mut v = json_minimo();
    v["due_date"] = json!("1900-02-29"); // divisível por 100, não por 400
    assert!(serde_json::from_value::<ExtractedBill>(v).is_err());
}

#[test]
fn confianca_fora_do_intervalo_desserializa_mas_validacao_aponta() {
    let mut v = json_minimo();
    v["extraction_confidence"] = json!(1.5);
    let b: ExtractedBill = serde_json::from_value(v).unwrap();
    assert!(validate_bill(&b).contains(&ValidationIssue::ConfidenceOutOfRange));

    let mut v = json_minimo();
    v["extraction_confidence"] = json!(-0.1);
    let b: ExtractedBill = serde_json::from_value(v).unwrap();
    assert!(validate_bill(&b).contains(&ValidationIssue::ConfidenceOutOfRange));
}

#[test]
fn confianca_inteira_0_e_1_sao_aceitas() {
    for n in [0, 1] {
        let mut v = json_minimo();
        v["extraction_confidence"] = json!(n);
        let b: ExtractedBill = serde_json::from_value(v).unwrap();
        assert!(validate_bill(&b).is_empty());
    }
}

#[test]
fn confianca_como_string_falha() {
    let mut v = json_minimo();
    v["extraction_confidence"] = json!("0.9");
    assert!(serde_json::from_value::<ExtractedBill>(v).is_err());
}

#[test]
fn strings_com_unicode_emoji_e_espacos_fazem_roundtrip() {
    let mut b = fatura();
    b.provider_name = "  Vivaz   Telecom ação 😀 \u{200B} ".into();
    b.items[0].description = "Plano \"aspas\" \\ barra \n quebra \t tab".into();
    b.items[0].original_text = "ÇÃO — ½ ‱ 日本語 \u{0}".into();
    let s = serde_json::to_string(&b).unwrap();
    let volta: ExtractedBill = serde_json::from_str(&s).unwrap();
    assert_eq!(volta, b);
}

#[test]
fn strings_vazias_sao_aceitas_pelo_schema() {
    // O schema não impõe tamanho mínimo; a regra de negócio fica nas camadas acima.
    let mut v = json_minimo();
    v["provider_name"] = json!("");
    v["items"][0]["id"] = json!("");
    v["items"][0]["original_text"] = json!("");
    assert!(serde_json::from_value::<ExtractedBill>(v).is_ok());
}

#[test]
fn items_vazio_desserializa_e_validacao_aponta() {
    let mut v = json_minimo();
    v["items"] = json!([]);
    v["total_cents"] = json!(0);
    let b: ExtractedBill = serde_json::from_value(v).unwrap();
    assert_eq!(validate_bill(&b), vec![ValidationIssue::NoItems]);
}

#[test]
fn items_com_tipo_errado_falha() {
    for ruim in [json!({}), json!("x"), json!(null), json!([1, 2])] {
        let mut v = json_minimo();
        v["items"] = ruim.clone();
        assert!(
            serde_json::from_value::<ExtractedBill>(v).is_err(),
            "items={ruim} deveria falhar"
        );
    }
}

#[test]
fn fatura_grande_com_muitos_itens_funciona() {
    let mut b = fatura();
    b.items = (0..5_000).map(|i| item(&format!("i{i}"), 1)).collect();
    b.total_cents = 5_000;
    assert!(validate_bill(&b).is_empty());
    let s = serde_json::to_string(&b).unwrap();
    assert_eq!(serde_json::from_str::<ExtractedBill>(&s).unwrap(), b);
}

// ---------------------------------------------------------------------------
// Energia, histórico, período, multas
// ---------------------------------------------------------------------------

#[test]
fn energia_minima_sem_bandeira_e_sem_historico_usa_defaults() {
    let v = json!({
        "consumption_kwh": 0,
        "reading_type": "real",
        "reading_prev_date": "2026-09-05",
        "reading_curr_date": "2026-10-05"
    });
    let e: EnergyData = serde_json::from_value(v).unwrap();
    assert_eq!(e.tariff_flag, None);
    assert!(e.history.is_empty());
    assert_eq!(e.consumption_kwh, 0);
}

#[test]
fn energia_consumo_negativo_ou_maior_que_u32_falha() {
    for ruim in [json!(-1), json!(4_294_967_296_u64), json!(10.5)] {
        let v = json!({
            "consumption_kwh": ruim,
            "reading_type": "real",
            "reading_prev_date": "2026-09-05",
            "reading_curr_date": "2026-10-05"
        });
        assert!(
            serde_json::from_value::<EnergyData>(v).is_err(),
            "consumption_kwh={ruim} deveria falhar"
        );
    }
}

#[test]
fn energia_consumo_u32_max_e_aceito() {
    let v = json!({
        "consumption_kwh": u32::MAX,
        "reading_type": "media",
        "reading_prev_date": "2026-09-05",
        "reading_curr_date": "2026-10-05"
    });
    assert_eq!(
        serde_json::from_value::<EnergyData>(v)
            .unwrap()
            .consumption_kwh,
        u32::MAX
    );
}

#[test]
fn tipo_de_leitura_so_aceita_os_tres_valores_em_minusculas() {
    for (t, nome) in [
        (ReadingType::Real, "real"),
        (ReadingType::Estimada, "estimada"),
        (ReadingType::Media, "media"),
    ] {
        assert_eq!(serde_json::to_value(t).unwrap(), json!(nome));
        assert_eq!(
            serde_json::from_value::<ReadingType>(json!(nome)).unwrap(),
            t
        );
    }
    for ruim in ["Real", "REAL", "média", "estimado", ""] {
        assert!(serde_json::from_value::<ReadingType>(json!(ruim)).is_err());
    }
}

#[test]
fn bandeiras_tarifarias_fazem_roundtrip_e_rejeitam_variacoes() {
    for (f, nome) in [
        (TariffFlag::Verde, "verde"),
        (TariffFlag::Amarela, "amarela"),
        (TariffFlag::Vermelha1, "vermelha_1"),
        (TariffFlag::Vermelha2, "vermelha_2"),
        (TariffFlag::Escassez, "escassez"),
    ] {
        assert_eq!(serde_json::to_value(f).unwrap(), json!(nome));
        assert_eq!(
            serde_json::from_value::<TariffFlag>(json!(nome)).unwrap(),
            f
        );
    }
    for ruim in [
        "Verde",
        "vermelha1",
        "vermelha 1",
        "vermelha",
        "vermelha_3",
        "azul",
        "",
    ] {
        assert!(
            serde_json::from_value::<TariffFlag>(json!(ruim)).is_err(),
            "{ruim:?} deveria falhar"
        );
    }
}

#[test]
fn historico_de_consumo_aceita_vazio_duplicado_e_mes_invalido_no_schema() {
    // O schema não valida o mês do histórico (só string): documenta o comportamento.
    let v = json!({
        "consumption_kwh": 100,
        "reading_type": "real",
        "reading_prev_date": "2026-09-05",
        "reading_curr_date": "2026-10-05",
        "history": [
            {"month": "2026-09", "kwh": 0},
            {"month": "2026-09", "kwh": 10},
            {"month": "xx", "kwh": 4294967295u64}
        ]
    });
    let e: EnergyData = serde_json::from_value(v).unwrap();
    assert_eq!(e.history.len(), 3);
    assert_eq!(e.history[2].kwh, u32::MAX);
}

#[test]
fn historico_com_kwh_negativo_ou_sem_mes_falha() {
    for ponto in [
        json!({"month": "2026-09", "kwh": -1}),
        json!({"month": "2026-09"}),
        json!({"kwh": 10}),
        json!({"month": 202609, "kwh": 10}),
    ] {
        assert!(
            serde_json::from_value::<ConsumptionPoint>(ponto.clone()).is_err(),
            "{ponto} deveria falhar"
        );
    }
}

#[test]
fn periodo_invertido_desserializa_sem_validacao() {
    // Documenta: `Period` não impede start > end (quem usa deve tratar).
    let p: Period =
        serde_json::from_value(json!({"start": "2026-10-31", "end": "2026-10-01"})).unwrap();
    assert!(p.start > p.end);
}

#[test]
fn periodo_com_campo_faltando_ou_data_ruim_falha() {
    assert!(serde_json::from_value::<Period>(json!({"start": "2026-10-01"})).is_err());
    assert!(
        serde_json::from_value::<Period>(json!({"start": "2026-10-01", "end": "2026-02-30"}))
            .is_err()
    );
}

#[test]
fn multas_com_zeros_e_negativos_fazem_roundtrip() {
    let zero = LateFees {
        base_cents: 0,
        fine_cents: 0,
        interest_cents: 0,
        days_late: 0,
    };
    let s = serde_json::to_string(&zero).unwrap();
    assert_eq!(serde_json::from_str::<LateFees>(&s).unwrap(), zero);

    let extremo = LateFees {
        base_cents: i64::MIN,
        fine_cents: i64::MAX,
        interest_cents: -1,
        days_late: u32::MAX,
    };
    let s = serde_json::to_string(&extremo).unwrap();
    assert_eq!(serde_json::from_str::<LateFees>(&s).unwrap(), extremo);
}

#[test]
fn multas_com_dias_negativos_ou_campo_faltando_falham() {
    let base = json!({"base_cents": 1, "fine_cents": 2, "interest_cents": 3, "days_late": 4});
    assert!(serde_json::from_value::<LateFees>(base.clone()).is_ok());
    let mut v = base.clone();
    v["days_late"] = json!(-1);
    assert!(serde_json::from_value::<LateFees>(v).is_err());
    for campo in ["base_cents", "fine_cents", "interest_cents", "days_late"] {
        let mut v = base.clone();
        v.as_object_mut().unwrap().remove(campo);
        assert!(
            serde_json::from_value::<LateFees>(v).is_err(),
            "sem {campo}"
        );
    }
}

// ---------------------------------------------------------------------------
// Severity, RuleStatus
// ---------------------------------------------------------------------------

#[test]
fn severity_ordena_do_mais_grave_ao_menos_grave() {
    let mut v = vec![Severity::Ok, Severity::Suspicious, Severity::Unlawful];
    v.sort();
    assert_eq!(
        v,
        vec![Severity::Unlawful, Severity::Suspicious, Severity::Ok]
    );
}

#[test]
fn severity_aceita_so_os_nomes_em_portugues() {
    for (s, nome) in [
        (Severity::Unlawful, "indevido"),
        (Severity::Suspicious, "suspeito"),
        (Severity::Ok, "ok"),
    ] {
        assert_eq!(serde_json::to_value(s).unwrap(), json!(nome));
        assert_eq!(serde_json::from_value::<Severity>(json!(nome)).unwrap(), s);
    }
    for ruim in [
        "Unlawful", "unlawful", "Indevido", "INDEVIDO", "OK", "", "ok ",
    ] {
        assert!(
            serde_json::from_value::<Severity>(json!(ruim)).is_err(),
            "{ruim:?} deveria falhar"
        );
    }
}

#[test]
fn rule_status_roundtrip_e_rejeita_desconhecido() {
    assert_eq!(
        serde_json::to_value(RuleStatus::Pending).unwrap(),
        json!("pending")
    );
    assert_eq!(
        serde_json::from_value::<RuleStatus>(json!("validated")).unwrap(),
        RuleStatus::Validated
    );
    assert!(serde_json::from_value::<RuleStatus>(json!("a_validar")).is_err());
}

// ---------------------------------------------------------------------------
// Finding
// ---------------------------------------------------------------------------

#[test]
fn finding_minimo_usa_defaults_para_id_original_e_pagina() {
    let v = json!({
        "rule_id": "TEL-01",
        "severity": "indevido",
        "title": "Serviço não contratado",
        "explanation": "Cobrança sem pedido.",
        "item_refs": [],
        "amount_cents": 0,
        "legal_basis": []
    });
    let f: Finding = serde_json::from_value(v).unwrap();
    assert_eq!(f.id, "");
    assert_eq!(f.original_text, None);
    assert_eq!(f.page, None);
    assert!(f.item_refs.is_empty());
    assert!(f.legal_basis.is_empty());
}

#[test]
fn finding_valor_negativo_e_extremos_fazem_roundtrip() {
    for cents in [i64::MIN, -1, 0, 1, i64::MAX] {
        let f = Finding {
            id: "TEL-01:i1".into(),
            rule_id: "TEL-01".into(),
            severity: Severity::Suspicious,
            title: "Título".into(),
            explanation: "Explicação.".into(),
            item_refs: vec!["i1".into()],
            original_text: Some("TEXTO".into()),
            page: Some(u32::MAX),
            amount_cents: cents,
            legal_basis: vec![LegalRef {
                label: "CDC art. 42".into(),
                url: "https://example.com/".into(),
            }],
        };
        let s = serde_json::to_string(&f).unwrap();
        assert_eq!(serde_json::from_str::<Finding>(&s).unwrap(), f);
    }
}

#[test]
fn finding_sem_campos_obrigatorios_falha() {
    let base = json!({
        "rule_id": "TEL-01",
        "severity": "ok",
        "title": "t",
        "explanation": "e",
        "item_refs": [],
        "amount_cents": 0,
        "legal_basis": []
    });
    assert!(serde_json::from_value::<Finding>(base.clone()).is_ok());
    for campo in [
        "rule_id",
        "severity",
        "title",
        "explanation",
        "item_refs",
        "amount_cents",
        "legal_basis",
    ] {
        let mut v = base.clone();
        v.as_object_mut().unwrap().remove(campo);
        assert!(serde_json::from_value::<Finding>(v).is_err(), "sem {campo}");
    }
}

#[test]
fn finding_com_varias_refs_e_unicode_preserva_a_ordem() {
    let f = Finding {
        id: "R+é+😀".into(),
        rule_id: "ENE-ç".into(),
        severity: Severity::Ok,
        title: "".into(),
        explanation: "".into(),
        item_refs: vec!["b".into(), "a".into(), "b".into(), "".into()],
        original_text: Some("".into()),
        page: Some(0),
        amount_cents: 0,
        legal_basis: vec![],
    };
    let s = serde_json::to_string(&f).unwrap();
    let volta: Finding = serde_json::from_str(&s).unwrap();
    assert_eq!(volta.item_refs, vec!["b", "a", "b", ""]);
    assert_eq!(volta, f);
}

#[test]
fn legal_ref_exige_label_e_url() {
    assert!(serde_json::from_value::<LegalRef>(json!({"label": "x"})).is_err());
    assert!(serde_json::from_value::<LegalRef>(json!({"url": "x"})).is_err());
    assert!(serde_json::from_value::<LegalRef>(json!({"label": "", "url": ""})).is_ok());
}

// ---------------------------------------------------------------------------
// Answer / Answers / Question
// ---------------------------------------------------------------------------

#[test]
fn answer_todas_as_variantes_fazem_roundtrip() {
    for a in [
        Answer::Yes,
        Answer::No,
        Answer::Amount(0),
        Answer::Amount(-1),
        Answer::Amount(i64::MAX),
        Answer::Amount(i64::MIN),
        Answer::Date(data(2024, 2, 29)),
        Answer::Date(NaiveDate::MIN),
        Answer::Date(NaiveDate::MAX),
    ] {
        let s = serde_json::to_string(&a).unwrap();
        assert_eq!(serde_json::from_str::<Answer>(&s).unwrap(), a, "json: {s}");
    }
}

#[test]
fn answer_date_formato_iso() {
    assert_eq!(
        serde_json::to_string(&Answer::Date(data(2026, 1, 5))).unwrap(),
        r#"{"type":"date","value":"2026-01-05"}"#
    );
}

#[test]
fn answer_malformada_falha() {
    for ruim in [
        r#"{}"#,
        r#"{"type":"talvez"}"#,
        r#"{"type":"YES"}"#,
        r#"{"type":"amount"}"#,
        r#"{"type":"amount","value":"100"}"#,
        r#"{"type":"amount","value":1.5}"#,
        r#"{"type":"amount","value":null}"#,
        r#"{"type":"date"}"#,
        r#"{"type":"date","value":"2026-02-30"}"#,
        r#"{"type":"date","value":20261110}"#,
        r#"{"value":10}"#,
        r#""yes""#,
        "true",
        "null",
    ] {
        assert!(
            serde_json::from_str::<Answer>(ruim).is_err(),
            "deveria falhar: {ruim}"
        );
    }
}

#[test]
fn answers_mapa_vazio_e_com_chaves_unicode_fazem_roundtrip() {
    let vazio = Answers::new();
    assert_eq!(serde_json::to_string(&vazio).unwrap(), "{}");

    let mut m: Answers = BTreeMap::new();
    m.insert("TEL-01:i3".into(), Answer::Yes);
    m.insert("".into(), Answer::No);
    m.insert("ação 😀".into(), Answer::Amount(-5));
    let s = serde_json::to_string(&m).unwrap();
    assert_eq!(serde_json::from_str::<Answers>(&s).unwrap(), m);
}

#[test]
fn answers_serializa_chaves_em_ordem_deterministica() {
    let mut m = Answers::new();
    m.insert("b".into(), Answer::Yes);
    m.insert("a".into(), Answer::No);
    assert_eq!(
        serde_json::to_string(&m).unwrap(),
        r#"{"a":{"type":"no"},"b":{"type":"yes"}}"#
    );
}

#[test]
fn answers_chave_repetida_no_json_fica_com_a_ultima() {
    let s = r#"{"k":{"type":"yes"},"k":{"type":"no"}}"#;
    let m: Answers = serde_json::from_str(s).unwrap();
    assert_eq!(m.len(), 1);
    assert_eq!(m["k"], Answer::No);
}

#[test]
fn question_kinds_em_snake_case_e_item_id_opcional() {
    for (k, nome) in [
        (QuestionKind::YesNo, "yes_no"),
        (QuestionKind::Amount, "amount"),
        (QuestionKind::Date, "date"),
    ] {
        assert_eq!(serde_json::to_value(k).unwrap(), json!(nome));
    }
    assert!(serde_json::from_value::<QuestionKind>(json!("yesno")).is_err());

    let q: Question = serde_json::from_value(json!({
        "id": "TEL-01:i3", "rule_id": "TEL-01", "text": "Você contratou?", "kind": "yes_no"
    }))
    .unwrap();
    assert_eq!(q.item_id, None);
}

#[test]
fn question_sem_campos_obrigatorios_falha() {
    let base = json!({"id": "x", "rule_id": "r", "text": "t", "kind": "date"});
    assert!(serde_json::from_value::<Question>(base.clone()).is_ok());
    for campo in ["id", "rule_id", "text", "kind"] {
        let mut v = base.clone();
        v.as_object_mut().unwrap().remove(campo);
        assert!(
            serde_json::from_value::<Question>(v).is_err(),
            "sem {campo}"
        );
    }
}

// ---------------------------------------------------------------------------
// Dispute / DisputeStep
// ---------------------------------------------------------------------------

#[test]
fn dispute_vazia_e_com_extremos_faz_roundtrip() {
    let vazia = Dispute {
        text: "".into(),
        total_cents: 0,
        steps: vec![],
        deadlines: vec![],
        disclaimer: "".into(),
    };
    let s = serde_json::to_string(&vazia).unwrap();
    assert_eq!(serde_json::from_str::<Dispute>(&s).unwrap(), vazia);

    let cheia = Dispute {
        text: "Prezados,\n\nContesto a cobrança.\n".into(),
        total_cents: i64::MAX,
        steps: vec![
            DisputeStep {
                title: "Ligue".into(),
                description: "Peça o protocolo.".into(),
                url: None,
            },
            DisputeStep {
                title: "Anatel".into(),
                description: "Registre.".into(),
                url: Some("https://example.com/".into()),
            },
        ],
        deadlines: vec!["30 dias".into()],
        disclaimer: "Não é aconselhamento jurídico.".into(),
    };
    let s = serde_json::to_string(&cheia).unwrap();
    assert_eq!(serde_json::from_str::<Dispute>(&s).unwrap(), cheia);
}

#[test]
fn dispute_step_url_e_opcional_mas_titulo_nao() {
    assert!(
        serde_json::from_value::<DisputeStep>(json!({"title": "t", "description": "d"})).is_ok()
    );
    assert!(serde_json::from_value::<DisputeStep>(json!({"description": "d"})).is_err());
    assert!(
        serde_json::from_value::<DisputeStep>(json!({"title": "t", "description": "d", "url": 42}))
            .is_err()
    );
}

#[test]
fn dispute_sem_disclaimer_falha() {
    let v = json!({"text": "t", "total_cents": 0, "steps": [], "deadlines": []});
    assert!(serde_json::from_value::<Dispute>(v).is_err());
}

// ---------------------------------------------------------------------------
// Analysis
// ---------------------------------------------------------------------------

#[test]
fn analysis_minima_faz_roundtrip_e_paid_default_none() {
    let a = analise();
    let mut v = serde_json::to_value(&a).unwrap();
    v.as_object_mut().unwrap().remove("paid");
    let volta: Analysis = serde_json::from_value(v).unwrap();
    assert_eq!(volta.paid, None);
    assert_eq!(volta, a);
}

#[test]
fn analysis_paid_distingue_none_some_true_e_some_false() {
    for p in [None, Some(true), Some(false)] {
        let mut a = analise();
        a.paid = p;
        let s = serde_json::to_string(&a).unwrap();
        assert_eq!(serde_json::from_str::<Analysis>(&s).unwrap().paid, p);
    }
}

#[test]
fn analysis_source_so_aceita_upload_ou_demo() {
    for (s, nome) in [
        (AnalysisSource::Upload, "upload"),
        (AnalysisSource::Demo, "demo"),
    ] {
        assert_eq!(serde_json::to_value(s).unwrap(), json!(nome));
    }
    assert!(serde_json::from_value::<AnalysisSource>(json!("Demo")).is_err());
    assert!(serde_json::from_value::<AnalysisSource>(json!("sintetico")).is_err());
}

#[test]
fn analysis_sem_campo_obrigatorio_falha() {
    let base = serde_json::to_value(analise()).unwrap();
    for campo in [
        "id",
        "created_at",
        "expires_at",
        "source",
        "bill",
        "answers",
        "findings",
        "questions",
        "recoverable_cents",
        "rules_version",
    ] {
        let mut v = base.clone();
        v.as_object_mut().unwrap().remove(campo);
        assert!(
            serde_json::from_value::<Analysis>(v).is_err(),
            "sem {campo} deveria falhar"
        );
    }
}

#[test]
fn analysis_datas_rfc3339_sao_apenas_strings_no_schema() {
    // Documenta: created_at/expires_at não são validados pelo tipo.
    let mut a = analise();
    a.created_at = "ontem".into();
    a.expires_at = "".into();
    let s = serde_json::to_string(&a).unwrap();
    assert_eq!(serde_json::from_str::<Analysis>(&s).unwrap(), a);
}

#[test]
fn analysis_recuperavel_negativo_e_extremo_fazem_roundtrip() {
    for c in [i64::MIN, -1, 0, i64::MAX] {
        let mut a = analise();
        a.recoverable_cents = c;
        let s = serde_json::to_string(&a).unwrap();
        assert_eq!(
            serde_json::from_str::<Analysis>(&s)
                .unwrap()
                .recoverable_cents,
            c
        );
    }
}

#[test]
fn analysis_com_respostas_achados_e_perguntas_faz_roundtrip() {
    let mut a = analise();
    a.answers.insert("TEL-01:i3".into(), Answer::Yes);
    a.answers
        .insert("TEL-02:i2".into(), Answer::Date(data(2026, 10, 1)));
    a.questions.push(Question {
        id: "TEL-01:i3".into(),
        rule_id: "TEL-01".into(),
        item_id: Some("i3".into()),
        text: "Você contratou o Seguro Tela?".into(),
        kind: QuestionKind::YesNo,
    });
    a.findings.push(Finding {
        id: "TEL-01:i3".into(),
        rule_id: "TEL-01".into(),
        severity: Severity::Unlawful,
        title: "Seguro sem contrato".into(),
        explanation: "Cobrança de serviço que você não pediu.".into(),
        item_refs: vec!["i3".into()],
        original_text: Some("SEG TELA 10,00".into()),
        page: Some(1),
        amount_cents: 1_000,
        legal_basis: vec![],
    });
    a.recoverable_cents = 2_000; // em dobro (CDC art. 42)
    let s = serde_json::to_string(&a).unwrap();
    assert_eq!(serde_json::from_str::<Analysis>(&s).unwrap(), a);
}

// ---------------------------------------------------------------------------
// Constantes e tipos
// ---------------------------------------------------------------------------

#[test]
fn constantes_publicas_tem_os_valores_documentados() {
    assert_eq!(SUM_TOLERANCE_CENTS, 5);
    assert_eq!(MAX_EXPLANATION_WORDS, 25);
    assert_eq!(FORBIDDEN_TERMS.len(), 5);
    // Os termos precisam estar em minúsculas, senão nunca batem com o texto
    // já convertido por `to_lowercase`.
    for t in FORBIDDEN_TERMS {
        assert_eq!(
            t,
            t.to_lowercase(),
            "termo proibido fora de minúsculas: {t}"
        );
        assert!(!t.is_empty());
    }
}
