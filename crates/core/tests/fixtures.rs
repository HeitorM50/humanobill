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
