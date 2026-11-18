//! API HTTP do HumanoBill (Axum). Rotas em docs/04-arquitetura.md.

mod config;
mod error;
mod limits;
mod mailer;
mod repo;
mod routes;
mod upload;

use std::{net::SocketAddr, sync::Arc, time::Duration};

use axum::http::{Method, header};
use humanobill_extract::{BillExtractor, GeminiExtractor};
use sqlx::postgres::PgPoolOptions;
use tower_http::{cors::CorsLayer, timeout::TimeoutLayer, trace::TraceLayer};
use tracing_subscriber::EnvFilter;

use crate::{config::Config, repo::PgRepo, routes::AppState};

#[tokio::main]
async fn main() {
    // Logs em JSON e sem conteúdo de fatura (RNF13)
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    let cfg = match Config::from_env() {
        Ok(c) => c,
        Err(e) => {
            tracing::error!(error = %e, "configuração inválida");
            std::process::exit(1);
        }
    };

    // O banco pode demorar a responder no boot (Supabase acordando, rede do Render).
    // Tenta por ~1 min antes de desistir, para não entrar em loop de reinício (502).
    let mut attempt = 0u32;
    let pool = loop {
        attempt += 1;
        match PgPoolOptions::new()
            .max_connections(5)
            .acquire_timeout(Duration::from_secs(5))
            .connect(&cfg.database_url)
            .await
        {
            Ok(pool) => break pool,
            Err(e) if attempt < 12 => {
                tracing::warn!(attempt, error = %e, "banco indisponível no boot, tentando de novo");
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
            Err(e) => {
                tracing::error!(error = %e, "não foi possível conectar no banco");
                std::process::exit(1);
            }
        }
    };
    sqlx::migrate!("../../migrations")
        .run(&pool)
        .await
        .expect("falha nas migrations");

    let extractor: Option<Arc<dyn BillExtractor>> = match cfg.gemini.clone() {
        Some(g) => {
            tracing::info!(model = %g.model, "extração por IA ligada");
            Some(Arc::new(GeminiExtractor::new(g)))
        }
        None => {
            tracing::warn!("GEMINI_API_KEY ausente: só a demo funciona");
            None
        }
    };

    let state = AppState {
        repo: Arc::new(PgRepo::new(pool)),
        extractor,
        fixtures_dir: cfg.fixtures_dir.clone(),
        limits: Some(limits::Limits::PRODUCTION),
    };

    // Lembrete por e-mail (RF18): sem RESEND_API_KEY/RESEND_FROM a rota responde 503
    // e o resto da API segue normal.
    let mailer: Option<Arc<dyn mailer::Mailer>> = match cfg.resend.clone() {
        Some(r) => {
            tracing::info!("lembrete por e-mail ligado");
            Some(Arc::new(mailer::ResendMailer::new(r)))
        }
        None => {
            tracing::warn!("RESEND_API_KEY/RESEND_FROM ausentes: lembrete por e-mail desligado");
            None
        }
    };
    let reminders = routes::reminders::ReminderState {
        repo: state.repo.clone(),
        mailer,
        cron_secret: cfg.cron_secret.clone().map(|s| s.0),
        web_url: cfg.web_url.clone(),
    };

    let cors = CorsLayer::new()
        .allow_origin(cfg.cors_origins.clone())
        .allow_methods([Method::GET, Method::POST, Method::PATCH])
        .allow_headers([header::CONTENT_TYPE]);

    let app = routes::router(state)
        .merge(routes::reminders::router(
            reminders,
            Some(limits::Limits::PRODUCTION),
        ))
        .layer(TimeoutLayer::with_status_code(
            axum::http::StatusCode::REQUEST_TIMEOUT,
            Duration::from_secs(90),
        ))
        .layer(cors)
        .layer(TraceLayer::new_for_http());

    let addr = SocketAddr::from(([0, 0, 0, 0], cfg.port));
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("falha ao abrir a porta");
    tracing::info!(%addr, "api no ar");
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await
    .expect("falha no servidor");
}

async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };
    #[cfg(unix)]
    let term = async {
        if let Ok(mut s) = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        {
            s.recv().await;
        }
    };
    #[cfg(not(unix))]
    let term = std::future::pending::<()>();
    tokio::select! { _ = ctrl_c => {}, _ = term => {} }
}
