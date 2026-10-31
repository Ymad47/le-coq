//! Le Coq Backend - Rust Core

use le_coq_backend::{api, db, plugin_bridge, AppState, Config};

use anyhow::Result;
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

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

    // Fail fast on bad config (e.g. missing API key) before starting Python.
    let config = Config::from_env()?;

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

    let postgres_pool = db::postgres::create_pool(&config.database_url).await?;

    match config.auth {
        le_coq_backend::auth::AuthMode::Disabled => {
            tracing::warn!("⚠️  DISABLE_AUTH=true — API authentication is OFF (loopback only)")
        }
        le_coq_backend::auth::AuthMode::ApiKey(_) => {
            tracing::info!(
                "🔐 API key auth enabled (header: {})",
                le_coq_backend::auth::API_KEY_HEADER
            )
        }
    }

    let state = Arc::new(AppState::new(postgres_pool, config.clone()));

    let app = api::create_router(state).layer(
        TraceLayer::new_for_http()
            .make_span_with(|request: &axum::http::Request<_>| {
                tracing::info_span!(
                    "http_request",
                    method = %request.method(),
                    uri = %request.uri(),
                )
            })
            .on_response(
                |response: &axum::http::Response<axum::body::Body>,
                 latency: std::time::Duration,
                 span: &tracing::Span| {
                    let _enter = span.enter();
                    tracing::info!(
                        status = %response.status(),
                        latency_ms = latency.as_millis(),
                        "request completed"
                    );
                },
            ),
    );

    let addr: SocketAddr = config.bind_address.parse()?;
    tracing::info!("🚀 Server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
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
