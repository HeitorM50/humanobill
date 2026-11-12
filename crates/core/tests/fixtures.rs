//! Valida que as faturas de exemplo desserializam e são consistentes.

use std::fs;
use std::path::PathBuf;

use humanobill_core::{ExtractedBill, validate_bill};

#[test]
fn fixtures_extraidas_sao_validas() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/extracted");
    let mut n = 0;
    for entry in fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let txt = fs::read_to_string(&path).unwrap();
        let bill: ExtractedBill =
            serde_json::from_str(&txt).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let issues = validate_bill(&bill);
        assert!(issues.is_empty(), "{}: {issues:?}", path.display());
        n += 1;
    }
    assert!(n >= 3, "esperava ao menos 3 fixtures, achei {n}");
}

/// RF06: as explicações das faturas de exemplo (demo) são simples e curtas.
#[test]
fn explicacoes_das_fixtures_sao_simples() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/extracted");
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        let bill: humanobill_core::ExtractedBill =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        for item in &bill.items {
            assert!(
                humanobill_core::is_plain_explanation(&item.plain_explanation),
                "{}: explicação longa ou com juridiquês em {}",
                path.display(),
                item.id
            );
        }
    }
}
