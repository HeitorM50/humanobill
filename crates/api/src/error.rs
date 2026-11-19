//! Erros da API no formato `{"error": {"code", "message"}}` (docs/04).

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use humanobill_extract::ExtractError;
use serde_json::json;

#[derive(Debug)]
pub struct ApiError {
    pub status: StatusCode,
    pub code: &'static str,
    pub message: String,
}

impl ApiError {
    pub fn new(status: StatusCode, code: &'static str, message: impl Into<String>) -> Self {
        Self {
            status,
            code,
            message: message.into(),
        }
    }
    pub fn not_found() -> Self {
        Self::new(
            StatusCode::NOT_FOUND,
            "not_found",
            "Resultado não encontrado",
        )
    }
    pub fn internal() -> Self {
        Self::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal",
            "Erro interno. Tente de novo em instantes",
        )
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(json!({"error": {"code": self.code, "message": self.message}})),
        )
            .into_response()
    }
}

impl From<ExtractError> for ApiError {
    fn from(e: ExtractError) -> Self {
        match e {
            ExtractError::Unavailable(_) => Self::new(
                StatusCode::SERVICE_UNAVAILABLE,
                "extraction_unavailable",
                "A leitura por IA está indisponível agora. Use uma fatura de exemplo",
            ),
            ExtractError::RateLimited => Self::new(
                StatusCode::TOO_MANY_REQUESTS,
                "rate_limited",
                "Muitas tentativas seguidas, tente em alguns minutos",
            ),
            ExtractError::Provider(_) | ExtractError::Invalid { .. } => Self::new(
                StatusCode::UNPROCESSABLE_ENTITY,
                "extraction_failed",
                "Não consegui ler bem essa fatura. Tente um PDF ou uma foto mais nítida",
            ),
        }
    }
}

impl From<crate::repo::RepoError> for ApiError {
    fn from(e: crate::repo::RepoError) -> Self {
        tracing::error!(error = %e, "falha no banco");
        Self::internal()
    }
}

#[cfg(test)]
mod edge_tests {
    use axum::{body::Body, http::header};
    use http_body_util::BodyExt;
    use humanobill_core::ValidationIssue;
    use serde_json::Value;

    use super::*;
    use crate::repo::RepoError;

    async fn corpo(e: ApiError) -> (StatusCode, String, Value) {
        let resp = e.into_response();
        let st = resp.status();
        let ct = resp
            .headers()
            .get(header::CONTENT_TYPE)
            .map(|v| v.to_str().unwrap().to_string())
            .unwrap_or_default();
        let body: Body = resp.into_body();
        let bytes = body.collect().await.unwrap().to_bytes();
        let v = serde_json::from_slice(&bytes).expect("corpo deveria ser JSON válido");
        (st, ct, v)
    }

    #[tokio::test]
    async fn resposta_tem_content_type_json_e_so_as_chaves_code_e_message() {
        let (st, ct, v) = corpo(ApiError::new(StatusCode::IM_A_TEAPOT, "x", "y")).await;
        assert_eq!(st, StatusCode::IM_A_TEAPOT);
        assert!(ct.starts_with("application/json"), "{ct}");
        let obj = v.as_object().unwrap();
        assert_eq!(obj.len(), 1);
        let erro = obj["error"].as_object().unwrap();
        assert_eq!(erro.len(), 2);
        assert_eq!(erro["code"], "x");
        assert_eq!(erro["message"], "y");
    }

    #[tokio::test]
    async fn mensagem_vazia_unicode_aspas_e_quebra_de_linha_viram_json_valido() {
        let casos = [
            "",
            "Não consegui ler a fatura 🧾 — “tente de novo”",
            "linha 1\nlinha 2\t\"aspas\" \\ barra",
            "\u{0}\u{1f}",
            "日本語ñ",
        ];
        for m in casos {
            let (_, _, v) = corpo(ApiError::new(StatusCode::BAD_REQUEST, "c", m)).await;
            assert_eq!(v["error"]["message"], m, "{m:?}");
        }
    }

    #[tokio::test]
    async fn not_found_e_internal_tem_codigo_e_mensagem_fixos_em_portugues() {
        let (st, _, v) = corpo(ApiError::not_found()).await;
        assert_eq!(st, StatusCode::NOT_FOUND);
        assert_eq!(v["error"]["code"], "not_found");
        assert_eq!(v["error"]["message"], "Resultado não encontrado");
        let (st, _, v) = corpo(ApiError::internal()).await;
        assert_eq!(st, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(v["error"]["code"], "internal");
    }

    #[test]
    fn cada_erro_do_extrator_tem_status_e_codigo_proprios() {
        let casos = [
            (
                ExtractError::Unavailable("x".into()),
                StatusCode::SERVICE_UNAVAILABLE,
                "extraction_unavailable",
            ),
            (
                ExtractError::RateLimited,
                StatusCode::TOO_MANY_REQUESTS,
                "rate_limited",
            ),
            (
                ExtractError::Provider("".into()),
                StatusCode::UNPROCESSABLE_ENTITY,
                "extraction_failed",
            ),
            (
                ExtractError::Invalid { issues: vec![] },
                StatusCode::UNPROCESSABLE_ENTITY,
                "extraction_failed",
            ),
            (
                ExtractError::Invalid {
                    issues: vec![ValidationIssue::NoItems; 1000],
                },
                StatusCode::UNPROCESSABLE_ENTITY,
                "extraction_failed",
            ),
        ];
        for (e, st, code) in casos {
            let api = ApiError::from(e);
            assert_eq!(api.status, st);
            assert_eq!(api.code, code);
        }
    }

    #[test]
    fn mensagem_do_erro_do_extrator_nao_inclui_o_texto_interno() {
        let segredo = "AIzaSy-CHAVE-SECRETA";
        for e in [
            ExtractError::Unavailable(segredo.into()),
            ExtractError::Provider(segredo.into()),
        ] {
            assert!(!ApiError::from(e).message.contains(segredo));
        }
    }

    #[test]
    fn erro_de_banco_vira_500_generico_para_qualquer_variante() {
        let casos = [
            RepoError::Db(sqlx::Error::PoolTimedOut),
            RepoError::Db(sqlx::Error::RowNotFound),
            RepoError::Corrupt("senha=postgres://u:p@host/db".into()),
        ];
        for r in casos {
            let api = ApiError::from(r);
            assert_eq!(api.status, StatusCode::INTERNAL_SERVER_ERROR);
            assert_eq!(api.code, "internal");
            assert!(!api.message.contains("postgres"));
            assert!(!api.message.to_lowercase().contains("pool"));
        }
    }
}
