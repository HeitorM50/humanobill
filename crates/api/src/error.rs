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
                "Muitas análises seguidas, tente em alguns minutos",
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
