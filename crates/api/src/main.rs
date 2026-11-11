//! API HTTP do HumanoBill (Axum). Rotas em docs/04-arquitetura.md.

use std::net::SocketAddr;

use axum::{Json, Router, routing::get};
use serde_json::{Value, json};
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    // Logs em JSON e sem conteúdo de fatura (RNF13)
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    // Render injeta PORT; localmente usa 8080
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("falha ao abrir a porta");
    tracing::info!(%addr, "api no ar");
    axum::serve(listener, app())
        .with_graceful_shutdown(shutdown_signal())
        .await
        .expect("falha no servidor");
}

fn app() -> Router {
    // TODO(12/11): restringir CORS ao domínio do front (RNF06)
    Router::new()
        .route("/health", get(health))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
}

/// Usado pelo keep-alive (UptimeRobot) e pelo warm-up do front.
async fn health() -> Json<Value> {
    Json(json!({ "status": "ok", "version": env!("CARGO_PKG_VERSION") }))
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn health_responde_ok() {
        let Json(body) = health().await;
        assert_eq!(body["status"], "ok");
    }
}
