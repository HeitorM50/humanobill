//! Extrato bancário (#54) e fatura do cartão (#55) contra o Gemini real (gasta
//! 1 chamada por teste, 2 se a primeira resposta não passar na validação).
//!
//! `set -a && . ./.env && set +a && EXTRACT_CACHE_DIR=target/extract-cache \
//!   cargo test -p humanobill-extract --test live_banco_cartao -- --ignored --nocapture`
//!
//! Imprime só tipo, totais, categorias e o número de chamadas (RNF13).

use humanobill_core::{BillKind, ExtractedBill, ItemCategory};
use humanobill_extract::{BillExtractor, GeminiConfig, GeminiExtractor, InputMime};

fn root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn extractor() -> GeminiExtractor {
    let key = std::env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY");
    let model =
        std::env::var("GEMINI_MODEL").unwrap_or_else(|_| GeminiConfig::DEFAULT_MODEL.into());
    GeminiExtractor::new(GeminiConfig {
        api_key: key,
        model,
        fallback_model: Some(GeminiConfig::DEFAULT_FALLBACK_MODEL.into()),
        base_url: GeminiConfig::DEFAULT_BASE_URL.into(),
    })
}

fn expected(id: &str) -> ExtractedBill {
    let p = root().join(format!("fixtures/extracted/{id}.json"));
    serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap()
}

/// O JSON extraído não pode trazer conta, agência, cartão nem nomes das movimentações.
fn sem_dado_pessoal(bill: &ExtractedBill, proibidos: &[&str]) {
    let json = serde_json::to_string(bill).unwrap().to_lowercase();
    for p in proibidos {
        assert!(!json.contains(p), "dado pessoal na extração: {p}");
    }
}

fn categorias(b: &ExtractedBill) -> Vec<ItemCategory> {
    let mut c: Vec<_> = b.items.iter().map(|i| i.category).collect();
    c.sort_by_key(|x| format!("{x:?}"));
    c
}

#[tokio::test]
#[ignore]
async fn live_extrato_bancario_traz_so_as_cobrancas_do_banco() {
    let ex = extractor();
    let pdf = std::fs::read(root().join("fixtures/bills/extrato-1.pdf")).unwrap();
    let got = ex.extract(&pdf, InputMime::Pdf).await.expect("extração");
    let want = expected("extrato-1");
    println!(
        "extrato: kind {:?}, total {} (esperado {}), {} itens, {} chamadas",
        got.kind,
        got.total_cents,
        want.total_cents,
        got.items.len(),
        ex.http_calls()
    );
    assert_eq!(got.kind, BillKind::BankStatement);
    assert_eq!(got.total_cents, want.total_cents);
    assert_eq!(categorias(&got), categorias(&want));
    assert_eq!(got.due_date, want.due_date, "fim do período");
    sem_dado_pessoal(
        &got,
        &[
            "maria", "00000-0", "agência", "salario", "salário", "mercado", "farmacia", "pix",
        ],
    );
    let cesta = got.items.iter().any(|i| {
        i.category == ItemCategory::Tarifa && i.description.to_lowercase().contains("cesta")
    });
    assert!(cesta, "a cesta de serviços tem que vir como tarifa");
}

#[tokio::test]
#[ignore]
async fn live_fatura_do_cartao_junta_as_compras_e_separa_as_cobrancas() {
    let ex = extractor();
    let pdf = std::fs::read(root().join("fixtures/bills/cartao-1.pdf")).unwrap();
    let got = ex.extract(&pdf, InputMime::Pdf).await.expect("extração");
    let want = expected("cartao-1");
    println!(
        "cartão: kind {:?}, total {} (esperado {}), {} itens, multa {:?}, {} chamadas",
        got.kind,
        got.total_cents,
        want.total_cents,
        got.items.len(),
        got.late_fees.as_ref().map(|f| (f.base_cents, f.fine_cents)),
        ex.http_calls()
    );
    assert_eq!(got.kind, BillKind::CardBill);
    assert_eq!(got.total_cents, want.total_cents);
    assert_eq!(categorias(&got), categorias(&want));
    sem_dado_pessoal(
        &got,
        &[
            "maria",
            "0000 (",
            "mercado exemplo",
            "posto ficticio",
            "livraria",
        ],
    );
    let fees = got
        .late_fees
        .as_ref()
        .expect("late_fees da fatura paga em atraso");
    assert_eq!(
        (fees.base_cents, fees.fine_cents),
        (85_000, 3_400),
        "base e multa da CAR-04"
    );
}
