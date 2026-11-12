//! Rate limit por IP (RNF06): protege a quota do Gemini e a API free.
//!
//! Atrás do proxy do Render, o IP do cliente vem em `X-Forwarded-For`
//! (cabeçalho definido pelo próprio Render, confiável nesse contexto).

use std::{sync::Arc, time::Duration};

use axum::{Json, body::Body, http::StatusCode, response::IntoResponse};
use serde_json::json;
use tower_governor::{
    GovernorError, GovernorLayer,
    governor::{GovernorConfig, GovernorConfigBuilder},
    key_extractor::SmartIpKeyExtractor,
};

/// Limites por IP: rajada + tempo para recuperar 1 requisição.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// Upload real (chama a IA): 10 por hora.
    pub upload_burst: u32,
    pub upload_refill: Duration,
    /// Demais rotas que escrevem (demo, PATCH, contestação).
    pub write_burst: u32,
    pub write_refill: Duration,
}

impl Limits {
    pub const PRODUCTION: Limits = Limits {
        upload_burst: 10,
        upload_refill: Duration::from_secs(360),
        write_burst: 60,
        write_refill: Duration::from_secs(1),
    };
}

pub type Layer = GovernorLayer<SmartIpKeyExtractor, governor::middleware::NoOpMiddleware, Body>;

/// Cria a camada e devolve o config para a limpeza periódica.
pub fn layer(
    burst: u32,
    refill: Duration,
) -> (
    Layer,
    Arc<GovernorConfig<SmartIpKeyExtractor, governor::middleware::NoOpMiddleware>>,
) {
    let cfg = Arc::new(
        GovernorConfigBuilder::default()
            .period(refill)
            .burst_size(burst)
            .key_extractor(SmartIpKeyExtractor)
            .finish()
            .expect("config de rate limit válida"),
    );
    let layer = GovernorLayer::new(cfg.clone()).error_handler(|e| match e {
        GovernorError::TooManyRequests { .. } => (
            StatusCode::TOO_MANY_REQUESTS,
            Json(json!({"error": {"code": "rate_limited", "message": "Muitas análises seguidas, tente em alguns minutos"}})),
        )
            .into_response(),
        _ => (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": {"code": "bad_request", "message": "Requisição inválida"}})),
        )
            .into_response(),
    });
    (layer, cfg)
}

/// Remove IPs antigos da memória do limitador a cada minuto.
pub fn spawn_cleanup(
    cfgs: Vec<Arc<GovernorConfig<SmartIpKeyExtractor, governor::middleware::NoOpMiddleware>>>,
) {
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(Duration::from_secs(60));
        loop {
            tick.tick().await;
            for c in &cfgs {
                c.limiter().retain_recent();
            }
        }
    });
}
