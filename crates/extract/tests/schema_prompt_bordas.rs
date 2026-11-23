//! Testes de borda do `responseJsonSchema` e da coerência com os tipos do core
//! e com as fixtures em `fixtures/extracted`. Sem rede.

use std::collections::BTreeSet;
use std::path::Path;

use humanobill_core::{BillKind, ExtractedBill, ItemCategory, ReadingType, TariffFlag};
use humanobill_extract::{InputMime, parse_and_validate, schema::extracted_bill_schema};
use serde_json::Value;

fn raiz() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Nomes serializados (serde) de uma lista de variantes.
fn nomes<T: serde::Serialize>(vs: &[T]) -> BTreeSet<String> {
    vs.iter()
        .map(|v| {
            serde_json::to_value(v)
                .unwrap()
                .as_str()
                .unwrap()
                .to_owned()
        })
        .collect()
}

fn enum_do_schema(v: &Value) -> BTreeSet<String> {
    v["enum"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|x| x.as_str().map(str::to_owned))
        .collect()
}

fn fixtures() -> Vec<(String, String)> {
    let dir = raiz().join("fixtures/extracted");
    let mut out: Vec<_> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "json"))
        .map(|p| {
            (
                p.file_name().unwrap().to_string_lossy().into_owned(),
                std::fs::read_to_string(&p).unwrap(),
            )
        })
        .collect();
    out.sort();
    out
}

// ---------- schema x tipos do core ----------

#[test]
fn schema_e_um_objeto_com_required_dentro_de_properties() {
    fn confere(no: &Value, caminho: &str) {
        let Some(obj) = no.as_object() else { return };
        if let (Some(props), Some(req)) = (obj.get("properties"), obj.get("required")) {
            for r in req.as_array().unwrap() {
                let r = r.as_str().unwrap();
                assert!(
                    props.get(r).is_some(),
                    "{caminho}: required '{r}' não existe em properties"
                );
            }
        }
        for (k, v) in obj {
            confere(v, &format!("{caminho}/{k}"));
        }
        if let Some(arr) = no.as_array() {
            for (i, v) in arr.iter().enumerate() {
                confere(v, &format!("{caminho}[{i}]"));
            }
        }
    }
    let s = extracted_bill_schema();
    assert_eq!(s["type"], "object");
    confere(&s, "");
}

#[test]
fn schema_e_estavel_entre_chamadas() {
    assert_eq!(extracted_bill_schema(), extracted_bill_schema());
}

#[test]
fn enum_de_categorias_do_schema_bate_com_o_core() {
    let core = nomes(&[
        ItemCategory::Plano,
        ItemCategory::ServicoAdicional,
        ItemCategory::EncargoAtraso,
        ItemCategory::Imposto,
        ItemCategory::Energia,
        ItemCategory::Bandeira,
        ItemCategory::IluminacaoPublica,
        ItemCategory::Tarifa,
        ItemCategory::Outros,
    ]);
    let s = extracted_bill_schema();
    assert_eq!(
        enum_do_schema(&s["properties"]["items"]["items"]["properties"]["category"]),
        core
    );
}

#[test]
fn enum_de_kind_do_schema_bate_com_o_core() {
    let s = extracted_bill_schema();
    assert_eq!(
        enum_do_schema(&s["properties"]["kind"]),
        nomes(&[
            BillKind::Telecom,
            BillKind::Energy,
            BillKind::BankStatement,
            BillKind::CardBill,
        ])
    );
}

#[test]
fn enum_de_reading_type_do_schema_bate_com_o_core() {
    let s = extracted_bill_schema();
    assert_eq!(
        enum_do_schema(&s["properties"]["energy"]["properties"]["reading_type"]),
        nomes(&[ReadingType::Real, ReadingType::Estimada, ReadingType::Media])
    );
}

#[test]
fn enum_de_bandeira_do_schema_bate_com_o_core_e_aceita_nulo() {
    let s = extracted_bill_schema();
    let flag = &s["properties"]["energy"]["properties"]["tariff_flag"];
    let core = nomes(&[
        TariffFlag::Verde,
        TariffFlag::Amarela,
        TariffFlag::Vermelha1,
        TariffFlag::Vermelha2,
        TariffFlag::Escassez,
    ]);
    assert_eq!(enum_do_schema(flag), core);
    assert!(flag["enum"].as_array().unwrap().contains(&Value::Null));
    assert!(
        flag["type"]
            .as_array()
            .unwrap()
            .contains(&Value::from("null"))
    );
}

#[test]
fn campos_obrigatorios_do_schema_sao_os_que_o_core_nao_tem_default() {
    // Campos com `#[serde(default)]`/Option no core precisam ser opcionais no schema,
    // e os demais obrigatórios, senão a IA pode omitir algo que o parse exige.
    let s = extracted_bill_schema();
    let req: BTreeSet<&str> = s["required"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    for obrigatorio in [
        "kind",
        "provider_name",
        "reference_month",
        "due_date",
        "total_cents",
        "items",
        "extraction_confidence",
    ] {
        assert!(
            req.contains(obrigatorio),
            "{obrigatorio} deveria ser required"
        );
    }
    for opcional in ["issue_date", "plan_name", "late_fees", "energy"] {
        assert!(!req.contains(opcional), "{opcional} deveria ser opcional");
    }
}

#[test]
fn campos_obrigatorios_do_item_cobrem_o_que_o_serde_exige() {
    let s = extracted_bill_schema();
    let req: BTreeSet<&str> = s["properties"]["items"]["items"]["required"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    for c in [
        "id",
        "description",
        "amount_cents",
        "category",
        "original_text",
        "page",
        "plain_explanation",
    ] {
        assert!(req.contains(c), "item.{c} deveria ser required");
    }
    assert!(!req.contains("period"));
}

#[test]
fn limites_numericos_do_schema_batem_com_as_regras_de_dominio() {
    let s = extracted_bill_schema();
    let item = &s["properties"]["items"]["items"]["properties"];
    assert_eq!(item["page"]["minimum"], 1);
    assert_eq!(s["properties"]["extraction_confidence"]["minimum"], 0);
    assert_eq!(s["properties"]["extraction_confidence"]["maximum"], 1);
    assert_eq!(
        s["properties"]["late_fees"]["properties"]["days_late"]["minimum"],
        0
    );
    let energy = &s["properties"]["energy"]["properties"];
    assert_eq!(energy["consumption_kwh"]["minimum"], 0);
    assert_eq!(
        energy["history"]["items"]["properties"]["kwh"]["minimum"],
        0
    );
}

#[test]
fn valores_monetarios_do_schema_sao_inteiros_nunca_numero_decimal() {
    fn varre(no: &Value, achados: &mut Vec<String>, caminho: &str) {
        if let Some(obj) = no.as_object() {
            for (k, v) in obj {
                if k.ends_with("_cents") {
                    if v["type"] != "integer" {
                        achados.push(format!("{caminho}/{k}: {}", v["type"]));
                    }
                } else {
                    varre(v, achados, &format!("{caminho}/{k}"));
                }
            }
        }
    }
    let mut ruins = Vec::new();
    varre(&extracted_bill_schema(), &mut ruins, "");
    assert!(
        ruins.is_empty(),
        "campos *_cents que não são integer: {ruins:?}"
    );
}

#[test]
fn schema_nao_expoe_nenhum_campo_pessoal_nem_em_descricoes() {
    // RNF04, versão mais estrita que o teste do módulo: varre chaves E textos.
    let s = extracted_bill_schema().to_string().to_lowercase();
    for p in [
        "cpf",
        "cnpj",
        "address",
        "endereco",
        "endereço",
        "holder",
        "titular",
        "instala",
        "phone",
        "telefone",
        "email",
        "e-mail",
        "linha",
        "customer",
        "cliente",
        "\"name\"",
        "rg\"",
        "birth",
        "nascimento",
        "agencia",
        "agência",
        "account",
        "card_number",
        "digit",
        "dígito",
        "estabelecimento",
    ] {
        assert!(!s.contains(p), "termo pessoal no schema: {p}");
    }
}

// ---------- fixtures x schema x parse ----------

#[test]
fn toda_fixture_extraida_passa_no_parse_and_validate() {
    let todas = fixtures();
    assert!(
        todas.len() >= 7,
        "esperava ao menos 7 fixtures, achei {}",
        todas.len()
    );
    for (nome, txt) in todas {
        let bill =
            parse_and_validate(&txt).unwrap_or_else(|e| panic!("fixture {nome} reprovada: {e:?}"));
        assert!(!bill.items.is_empty(), "{nome} sem itens");
    }
}

#[test]
fn toda_fixture_so_usa_chaves_conhecidas_pelo_schema() {
    // Se a fixture tiver uma chave que o schema não declara, a IA nunca a produziria
    // (ou o schema ficou para trás do tipo do core).
    fn confere(valor: &Value, schema: &Value, caminho: &str, nome: &str) {
        match valor {
            Value::Object(obj) => {
                let Some(props) = schema.get("properties") else {
                    return;
                };
                for (k, v) in obj {
                    // `bbox` é o formato do core; a IA manda `box_2d`, e o parse converte (#51).
                    if k == "bbox" && props.get("box_2d").is_some() {
                        continue;
                    }
                    let sub = props
                        .get(k)
                        .unwrap_or_else(|| panic!("{nome}{caminho}: chave '{k}' fora do schema"));
                    confere(v, sub, &format!("{caminho}/{k}"), nome);
                }
            }
            Value::Array(arr) => {
                if let Some(it) = schema.get("items") {
                    for (i, v) in arr.iter().enumerate() {
                        confere(v, it, &format!("{caminho}[{i}]"), nome);
                    }
                }
            }
            _ => {}
        }
    }
    let schema = extracted_bill_schema();
    for (nome, txt) in fixtures() {
        let v: Value = serde_json::from_str(&txt).unwrap();
        confere(&v, &schema, "", &nome);
    }
}

#[test]
fn toda_fixture_tem_os_campos_required_do_schema() {
    let schema = extracted_bill_schema();
    let req: Vec<&str> = schema["required"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    for (nome, txt) in fixtures() {
        let v: Value = serde_json::from_str(&txt).unwrap();
        for r in &req {
            assert!(v.get(*r).is_some(), "{nome} sem {r}");
        }
    }
}

#[test]
fn fixtures_sobrevivem_a_ida_e_volta_serde_sem_perda() {
    for (nome, txt) in fixtures() {
        let bill: ExtractedBill = parse_and_validate(&txt).unwrap();
        let json = serde_json::to_string(&bill).unwrap();
        let back = parse_and_validate(&json).unwrap();
        assert_eq!(back, bill, "fixture {nome}");
    }
}

#[test]
fn fixture_de_injecao_nao_traz_o_texto_da_injecao() {
    // RNF07: a extração esperada da fatura com injeção não pode carregar a instrução.
    let txt =
        std::fs::read_to_string(raiz().join("fixtures/extracted/telecom-injecao.json")).unwrap();
    let baixo = txt.to_lowercase();
    for t in [
        "ignore as instru",
        "ignore previous",
        "desconsidere",
        "marque como ok",
    ] {
        assert!(!baixo.contains(t), "fixture de injeção contém '{t}'");
    }
    let normal = std::fs::read_to_string(raiz().join("fixtures/extracted/telecom-1.json")).unwrap();
    let a: ExtractedBill = serde_json::from_str(&txt).unwrap();
    let b: ExtractedBill = serde_json::from_str(&normal).unwrap();
    assert_eq!(a.total_cents, b.total_cents);
    assert_eq!(a.items.len(), b.items.len());
}

#[test]
fn fixtures_nao_contem_dados_pessoais_reais_evidentes() {
    // Só padrões óbvios (CPF formatado, e-mail, telefone com DDD): dados sintéticos apenas.
    for (nome, txt) in fixtures() {
        let b = txt.as_bytes();
        // CPF no formato 000.000.000-00
        for w in b.windows(14) {
            let cpf = w.len() == 14
                && w[3] == b'.'
                && w[7] == b'.'
                && w[11] == b'-'
                && [0, 1, 2, 4, 5, 6, 8, 9, 10, 12, 13]
                    .iter()
                    .all(|&i| w[i].is_ascii_digit());
            assert!(!cpf, "{nome}: parece CPF");
        }
        assert!(!txt.contains('@'), "{nome}: parece e-mail");
    }
}

// ---------- InputMime ----------

#[test]
fn mime_rejeita_variacoes_de_caixa_espacos_e_parametros() {
    for m in [
        "",
        " application/pdf",
        "application/pdf ",
        "APPLICATION/PDF",
        "application/pdf; charset=utf-8",
        "image/jpg",
        "image/gif",
        "image/svg+xml",
        "application/octet-stream",
        "pdf",
        "image/png\n",
        "ímage/png",
    ] {
        assert_eq!(InputMime::from_mime(m), None, "mime {m:?}");
    }
}

#[test]
fn mime_strings_sao_unicas_e_bem_formadas() {
    let todos = [
        InputMime::Pdf,
        InputMime::Png,
        InputMime::Jpeg,
        InputMime::Webp,
    ];
    let strs: BTreeSet<&str> = todos.iter().map(|m| m.as_str()).collect();
    assert_eq!(strs.len(), todos.len());
    for s in strs {
        assert!(s.is_ascii() && s.contains('/') && s == s.to_lowercase());
    }
}
