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
    // Charge .env / .env.backend si présents (pas d'erreur si absents)
    let _ = dotenvy::from_filename(".env.backend");
    let _ = dotenvy::dotenv();

    // PYO3_PYTHON doit pointer vers Python 3.11 (pydantic_core compilé cp311)
    // avant d'initialiser l'interpréteur embarqué.
    if std::env::var("PYO3_PYTHON").is_err() {
        // fallback: cherche python3.11 dans le PATH
        if let Ok(path) = which_python311() {
            std::env::set_var("PYO3_PYTHON", path);
        }
    }

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "le_coq=info,tower_http=debug".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("🐓 Le Coq backend starting...");

    // Initialize Python interpreter (PyO3)
    plugin_bridge::init_python().map_err(|e| anyhow::anyhow!("Failed to init Python: {}", e))?;
    pyo3::Python::with_gil(|py| {
        if let Ok(sys) = py.import("sys") {
            if let Ok(v) = sys.getattr("version") {
                tracing::info!("🐍 Python runtime: {}", v);
            }
        }
    });
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

/// Cherche python3.11 dans le PATH (binaire ou shim)
fn which_python311() -> Result<std::path::PathBuf> {
    for candidate in ["/home/yaya/.local/bin/python3.11", "python3.11"] {
        let out = std::process::Command::new(candidate)
            .arg("--version")
            .output();
        if let Ok(o) = out {
            if o.status.success() {
                // résultat absolu si commande trouvée via PATH
                if candidate.contains('/') {
                    return Ok(candidate.into());
                }
                if let Ok(p) = which::which(candidate) {
                    return Ok(p);
                }
            }
        }
    }
    Err(anyhow::anyhow!("python3.11 introuvable"))
}
