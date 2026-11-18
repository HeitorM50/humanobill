//! RF16: comparar duas análises já existentes (docs/04). Só lê: sem IA, sem escrita.

use axum::{
    Json,
    extract::{State, rejection::JsonRejection},
    http::StatusCode,
    response::IntoResponse,
};
use chrono::Utc;
use humanobill_core::ExtractedBill;
use humanobill_rules::compare::{CompareError, compare};
use serde::Deserialize;
use uuid::Uuid;

use super::AppState;
use crate::error::ApiError;

/// Corpo do `POST /comparisons`: exatamente dois ids de análise.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CompareBody {
    analysis_ids: [String; 2],
}

/// Lê a fatura de uma análise: 404 se não existe, 410 se já expirou (RF14).
async fn load_bill(s: &AppState, id: &str) -> Result<ExtractedBill, ApiError> {
    let id = Uuid::parse_str(id).map_err(|_| ApiError::not_found())?;
    let stored = s.repo.get(id).await?.ok_or_else(ApiError::not_found)?;
    if stored.expires_at <= Utc::now() {
        return Err(ApiError::new(
            StatusCode::GONE,
            "expired",
            "Este resultado expirou. Analise de novo",
        ));
    }
    Ok(stored.analysis.bill)
}

/// RF16: diferença entre duas faturas da mesma operadora (cálculo puro, sem IA).
pub(super) async fn create_comparison(
    State(s): State<AppState>,
    body: Result<Json<CompareBody>, JsonRejection>,
) -> Result<impl IntoResponse, ApiError> {
    let Json(body) = body.map_err(|_| {
        ApiError::new(
            StatusCode::BAD_REQUEST,
            "invalid_body",
            "Envie os ids de duas análises",
        )
    })?;
    let [first, second] = &body.analysis_ids;
    let a = load_bill(&s, first).await?;
    let b = load_bill(&s, second).await?;
    let comparison = compare(&a, &b).map_err(|e| match e {
        CompareError::DifferentProvider => ApiError::new(
            StatusCode::BAD_REQUEST,
            "different_provider",
            "Compare faturas da mesma operadora",
        ),
        CompareError::SameMonth => ApiError::new(
            StatusCode::BAD_REQUEST,
            "same_month",
            "Envie faturas de meses diferentes",
        ),
    })?;
    // Log sem conteúdo da fatura: só contagens.
    tracing::info!(
        added = comparison.added.len(),
        removed = comparison.removed.len(),
        changed = comparison.changed.len(),
        "comparação feita"
    );
    Ok(Json(comparison))
}

#[cfg(test)]
mod tests {
    use std::{path::PathBuf, sync::Arc};

    use axum::{Router, body::Body, http::Request};
    use chrono::{Duration, Utc};
    use http_body_util::BodyExt;
    use humanobill_core::{Analysis, AnalysisSource, ExtractedBill};
    use serde_json::{Value, json};
    use tower::ServiceExt;

    use super::super::{new_analysis, router};
    use super::*;
    use crate::repo::{MemRepo, Repo};

    fn fixture(name: &str) -> ExtractedBill {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/extracted")
            .join(format!("{name}.json"));
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
    }

    /// Outubro: a fatura da fixture sem o "Seguro Tela".
    fn outubro() -> ExtractedBill {
        let mut b = fixture("telecom-1");
        b.reference_month = "2026-09".into();
        b.items.retain(|i| i.description != "Seguro Tela");
        b.total_cents = b.items.iter().map(|i| i.amount_cents).sum();
        b
    }

    /// Novembro: a fixture como está (tem "Seguro Tela" de R$ 9,90).
    fn novembro() -> ExtractedBill {
        let mut b = fixture("telecom-1");
        b.reference_month = "2026-10".into();
        b
    }

    struct Ctx {
        app: Router,
        repo: Arc<MemRepo>,
    }

    fn ctx() -> Ctx {
        let repo = Arc::new(MemRepo::default());
        let state = AppState {
            repo: repo.clone(),
            extractor: None,
            fixtures_dir: PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures"),
            limits: None,
        };
        Ctx {
            app: router(state),
            repo,
        }
    }

    async fn guardar(c: &Ctx, bill: ExtractedBill) -> String {
        let a = new_analysis(bill, AnalysisSource::Demo);
        c.repo.insert(&a).await.unwrap();
        a.id
    }

    async fn guardar_expirada(c: &Ctx, bill: ExtractedBill) -> String {
        let mut a: Analysis = new_analysis(bill, AnalysisSource::Demo);
        a.expires_at = (Utc::now() - Duration::days(1)).to_rfc3339();
        c.repo.insert(&a).await.unwrap();
        a.id
    }

    async fn comparar(c: &Ctx, body: Value) -> (StatusCode, Value) {
        let req = Request::post("/comparisons")
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap();
        enviar(c, req).await
    }

    async fn enviar(c: &Ctx, req: Request<Body>) -> (StatusCode, Value) {
        let resp = c.app.clone().oneshot(req).await.unwrap();
        let status = resp.status();
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        (
            status,
            serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        )
    }

    fn erro(v: &Value) -> (&str, &str) {
        (
            v["error"]["code"].as_str().unwrap(),
            v["error"]["message"].as_str().unwrap(),
        )
    }

    // Cenário: Item novo aparece
    #[tokio::test]
    async fn item_novo_aparece_com_variacao_em_reais_e_porcentagem() {
        let c = ctx();
        let out = guardar(&c, outubro()).await;
        let nov = guardar(&c, novembro()).await;
        let (st, v) = comparar(&c, json!({"analysis_ids": [out, nov]})).await;

        assert_eq!(st, StatusCode::OK, "{v}");
        assert_eq!(v["added"][0]["description"], "Seguro Tela");
        assert_eq!(v["added"][0]["after_cents"], 990);
        assert_eq!(v["added"][0]["new_charge"], true);
        assert_eq!(v["before_month"], "2026-09");
        assert_eq!(v["after_month"], "2026-10");
        assert_eq!(v["before_total_cents"], 11280);
        assert_eq!(v["after_total_cents"], 12270);
        assert_eq!(v["total_delta_cents"], 990);
        assert_eq!(v["total_delta_pct"], 8.8);
        assert_eq!(v["removed"], json!([]));
        assert_eq!(v["changed"], json!([]));
        assert_eq!(v["unchanged"].as_array().unwrap().len(), 2);
    }

    /// Cria uma análise de demo pela rota, como a página `/comparar` faz no exemplo.
    async fn demo_id(c: &Ctx, sample: &str) -> String {
        let req = Request::post(format!("/analyses/demo/{sample}"))
            .body(Body::empty())
            .unwrap();
        let (st, v) = enviar(c, req).await;
        assert_eq!(st, StatusCode::CREATED, "{sample}: {v}");
        v["id"].as_str().unwrap().to_string()
    }

    // P13: exemplo de comparação da demo (fixtures de setembro e outubro)
    #[tokio::test]
    async fn exemplo_de_demo_telecom_1_anterior_contra_telecom_1_mostra_itens_novos() {
        let c = ctx();
        let anterior = demo_id(&c, "telecom-1-anterior").await;
        let atual = demo_id(&c, "telecom-1").await;
        let (st, v) = comparar(&c, json!({"analysis_ids": [anterior, atual]})).await;

        assert_eq!(st, StatusCode::OK, "{v}");
        assert_eq!(v["provider_name"], "Vivaz Telecom");
        assert_eq!(v["before_month"], "2026-09");
        assert_eq!(v["after_month"], "2026-10");
        assert_eq!(v["before_total_cents"], 9990);
        assert_eq!(v["after_total_cents"], 12270);
        assert_eq!(v["total_delta_cents"], 2280);
        assert_eq!(v["total_delta_pct"], 22.8);

        // Os dois serviços extras (TEL-01) aparecem como cobrança nova.
        let novos: Vec<(&str, bool)> = v["added"]
            .as_array()
            .unwrap()
            .iter()
            .map(|i| {
                (
                    i["description"].as_str().unwrap(),
                    i["new_charge"].as_bool().unwrap(),
                )
            })
            .collect();
        assert_eq!(novos.len(), 2, "{v}");
        assert!(novos.contains(&("Clube Vivaz Música", true)));
        assert!(novos.contains(&("Seguro Tela", true)));
        assert_eq!(v["removed"], json!([]));
        assert_eq!(v["changed"], json!([]));
        assert_eq!(v["unchanged"].as_array().unwrap().len(), 1);
        assert_eq!(
            v["unchanged"][0]["description"],
            "Plano Vivaz Controle 20GB"
        );
    }

    #[tokio::test]
    async fn exemplo_de_demo_com_o_mesmo_mes_nas_duas_pontas_e_recusado() {
        let c = ctx();
        let anterior = demo_id(&c, "telecom-1-anterior").await;
        let outra_anterior = demo_id(&c, "telecom-1-anterior").await;
        for ids in [
            json!([anterior, outra_anterior]),
            json!([anterior, anterior]),
        ] {
            let (st, v) = comparar(&c, json!({ "analysis_ids": ids })).await;
            assert_eq!(st, StatusCode::BAD_REQUEST);
            assert_eq!(
                erro(&v),
                ("same_month", "Envie faturas de meses diferentes")
            );
        }
    }

    #[tokio::test]
    async fn a_ordem_dos_ids_nao_importa() {
        let c = ctx();
        let out = guardar(&c, outubro()).await;
        let nov = guardar(&c, novembro()).await;
        let (_, a) = comparar(&c, json!({"analysis_ids": [out, nov]})).await;
        let (_, b) = comparar(&c, json!({"analysis_ids": [nov, out]})).await;
        assert_eq!(a, b);
    }

    #[tokio::test]
    async fn resposta_nao_traz_ids_nem_trecho_original() {
        let c = ctx();
        let out = guardar(&c, outubro()).await;
        let nov = guardar(&c, novembro()).await;
        let (_, v) = comparar(&c, json!({"analysis_ids": [out, nov]})).await;
        let texto = v.to_string();
        assert!(!texto.contains(&out) && !texto.contains(&nov));
        assert!(!texto.contains("original_text"));
        let mut chaves: Vec<&str> = v.as_object().unwrap().keys().map(String::as_str).collect();
        chaves.sort_unstable();
        assert_eq!(
            chaves,
            [
                "added",
                "after_month",
                "after_total_cents",
                "before_month",
                "before_total_cents",
                "changed",
                "kind",
                "provider_name",
                "removed",
                "total_delta_cents",
                "total_delta_pct",
                "unchanged"
            ]
        );
    }

    // Cenário: Operadoras diferentes
    #[tokio::test]
    async fn operadoras_diferentes_dizem_para_comparar_a_mesma_operadora() {
        let c = ctx();
        let tel = guardar(&c, novembro()).await;
        let mut luz = fixture("energia-1");
        luz.reference_month = "2026-09".into();
        let luz = guardar(&c, luz).await;
        let (st, v) = comparar(&c, json!({"analysis_ids": [tel, luz]})).await;
        assert_eq!(st, StatusCode::BAD_REQUEST);
        assert_eq!(
            erro(&v),
            ("different_provider", "Compare faturas da mesma operadora")
        );
    }

    // Cenário: Mesma fatura duas vezes
    #[tokio::test]
    async fn mesmo_mes_de_referencia_pede_meses_diferentes() {
        let c = ctx();
        let a = guardar(&c, novembro()).await;
        let b = guardar(&c, novembro()).await;
        let esperado = ("same_month", "Envie faturas de meses diferentes");
        for ids in [json!([a, b]), json!([a, a])] {
            let (st, v) = comparar(&c, json!({ "analysis_ids": ids })).await;
            assert_eq!(st, StatusCode::BAD_REQUEST);
            assert_eq!(erro(&v), esperado);
        }
    }

    // RF14: expiração
    #[tokio::test]
    async fn analise_expirada_responde_410_em_qualquer_posicao() {
        let c = ctx();
        let viva = guardar(&c, outubro()).await;
        let velha = guardar_expirada(&c, novembro()).await;
        for ids in [json!([viva, velha]), json!([velha, viva])] {
            let (st, v) = comparar(&c, json!({ "analysis_ids": ids })).await;
            assert_eq!(st, StatusCode::GONE);
            assert_eq!(erro(&v).0, "expired");
        }
    }

    #[tokio::test]
    async fn analise_inexistente_ou_id_invalido_responde_404() {
        let c = ctx();
        let viva = guardar(&c, outubro()).await;
        let nunca = uuid::Uuid::new_v4().to_string();
        for outro in [nunca.as_str(), "nao-e-uuid", ""] {
            let (st, v) = comparar(&c, json!({"analysis_ids": [viva, outro]})).await;
            assert_eq!(st, StatusCode::NOT_FOUND, "{outro:?}");
            assert_eq!(erro(&v).0, "not_found");
        }
    }

    #[tokio::test]
    async fn corpo_invalido_responde_400() {
        let c = ctx();
        let id = guardar(&c, outubro()).await;
        let casos = [
            json!({}),
            json!({"analysis_ids": []}),
            json!({"analysis_ids": [id]}),
            json!({"analysis_ids": [id, id, id]}),
            json!({"analysis_ids": [id, 7]}),
            json!({"analysis_ids": [id, id], "extra": true}),
            json!([id, id]),
        ];
        for body in casos {
            let (st, v) = comparar(&c, body.clone()).await;
            assert_eq!(st, StatusCode::BAD_REQUEST, "{body}");
            assert_eq!(erro(&v).0, "invalid_body");
        }
        let req = Request::post("/comparisons")
            .header("content-type", "application/json")
            .body(Body::from("{não é json"))
            .unwrap();
        let (st, v) = enviar(&c, req).await;
        assert_eq!(st, StatusCode::BAD_REQUEST);
        assert_eq!(erro(&v).0, "invalid_body");
    }
}
