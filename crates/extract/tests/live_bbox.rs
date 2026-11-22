//! #51 contra o Gemini real (gasta 1 chamada por teste; rode um de cada vez).
//!
//! - Parcela do aparelho no modelo reserva: tem de sair como `outros`, senão a
//!   `telecom-3` ganha um TEL-01 "suspeito" falso.
//! - Bbox numa foto: as caixas devolvidas caem perto das medidas na fixture.
//!   (20/11: pedindo `x/y/w/h` de 0 a 1, o modelo omitiu as 3 caixas; pedindo o
//!   `box_2d` nativo, que é o formato atual, acertou as 3 linhas com erro de ~0,005.)
//!
//! `set -a && . ./.env && set +a && \
//!   cargo test -p humanobill-extract --test live_bbox -- --ignored --nocapture <nome>`

use humanobill_core::{ExtractedBill, ItemCategory};
use humanobill_extract::{BillExtractor, GeminiConfig, GeminiExtractor, InputMime};

fn root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn extrator(model: String) -> GeminiExtractor {
    GeminiExtractor::new(GeminiConfig {
        api_key: std::env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY"),
        model,
        // Sem fallback: o teste mede exatamente o modelo pedido.
        fallback_model: None,
        base_url: GeminiConfig::DEFAULT_BASE_URL.into(),
    })
}

#[tokio::test]
#[ignore]
async fn live_parcela_do_aparelho_no_modelo_reserva_vira_outros() {
    let model =
        std::env::var("GEMINI_RESERVE_MODEL").unwrap_or_else(|_| "gemini-3.5-flash-lite".into());
    let pdf = std::fs::read(root().join("fixtures/bills/telecom-3.pdf")).unwrap();
    let bill = extrator(model)
        .extract(&pdf, InputMime::Pdf)
        .await
        .expect("extração");
    let parcela = bill
        .items
        .iter()
        .find(|i| i.description.to_lowercase().contains("parcela"))
        .expect("item da parcela do aparelho");
    assert_eq!(
        parcela.category,
        ItemCategory::Outros,
        "{:?}",
        parcela.category
    );
    assert!(
        bill.items
            .iter()
            .all(|i| i.category != ItemCategory::ServicoAdicional),
        "nenhum item da telecom-3 é serviço adicional"
    );
}

#[tokio::test]
#[ignore]
async fn live_bbox_da_foto_cai_perto_da_linha_medida() {
    let model =
        std::env::var("GEMINI_MODEL").unwrap_or_else(|_| GeminiConfig::DEFAULT_MODEL.into());
    let png = std::fs::read(root().join("fixtures/bills/telecom-1.png")).unwrap();
    let got = extrator(model)
        .extract(&png, InputMime::Png)
        .await
        .expect("extração");
    let want: ExtractedBill = serde_json::from_str(
        &std::fs::read_to_string(root().join("fixtures/extracted/telecom-1.json")).unwrap(),
    )
    .unwrap();
    let mut com_caixa = 0;
    for (g, w) in got.items.iter().zip(&want.items) {
        let (Some(gb), Some(wb)) = (g.bbox, w.bbox) else {
            eprintln!("{}: sem bbox", g.id);
            continue;
        };
        com_caixa += 1;
        let dy = (gb.y + gb.h / 2.0) - (wb.y + wb.h / 2.0);
        eprintln!("{}: ia={gb:?} medido={wb:?} dy_centro={dy:.4}", g.id);
        // A linha tem ~0,023 de altura: o centro precisa cair a menos de 2 linhas.
        assert!(dy.abs() < 0.05, "{}: caixa longe da linha (dy={dy})", g.id);
    }
    eprintln!("itens com bbox: {com_caixa}/{}", got.items.len());
}
