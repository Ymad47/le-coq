//! Le Coq Backend - Rust Core

mod api;
mod db;
mod graph;
mod plugin_bridge;
mod ingestion;
mod models;

use anyhow::Result;
use std::sync::Arc;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use std::net::SocketAddr;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "le_coq=info,tower_http=debug".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("🐓 Le Coq backend starting...");

    // Initialize Python interpreter (PyO3)
    plugin_bridge::init_python().map_err(|e| anyhow::anyhow!("Failed to init Python: {}", e))?;
    tracing::info!("🐍 Python interpreter initialized");

    let config = Config::from_env()?;
    let postgres_pool = db::postgres::create_pool(&config.database_url).await?;
    
    let state = Arc::new(AppState {
        postgres: postgres_pool,
        config: config.clone(),
    });

    let app = api::create_router(state).layer(TraceLayer::new_for_http());

    let addr: SocketAddr = config.bind_address.parse()?;
    tracing::info!("🚀 Server listening on {}", addr);
    
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

#[derive(Clone)]
pub struct AppState {
    pub postgres: sqlx::Pool<sqlx::Postgres>,
    pub config: Config,
}

#[derive(Clone)]
pub struct Config {
    pub bind_address: String,
    pub database_url: String,
    pub api_key_header: String,
}

impl Config {
    fn from_env() -> Result<Self> {
        Ok(Self {
            bind_address: std::env::var("BIND_ADDRESS").unwrap_or_else(|_| "0.0.0.0:8000".into()),
            database_url: std::env::var("DATABASE_URL")?,
            api_key_header: std::env::var("API_KEY_HEADER").unwrap_or_else(|_| "X-API-Key".into()),
        })
    }
}
