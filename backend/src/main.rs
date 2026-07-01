//! Le Coq Backend - Rust Core
//! 
//! High-performance API server, graph engine, and ingestion pipelines.

mod api;
mod db;
mod graph;
mod ingestion;
mod models;

use anyhow::Result;
use std::sync::Arc;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "le_coq=info,tower_http=debug".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("🐓 Le Coq backend starting...");

    // Load configuration
    let config = Config::from_env()?;
    
    // Initialize databases
    let postgres_pool = db::postgres::create_pool(&config.database_url).await?;
    let neo4j_pool = db::neo4j::create_pool(&config.neo4j_url)?;
    
    // Initialize search
    let search_client = meilisearch_sdk::client::Client::new(&config.meilisearch_url, Some(&config.meilisearch_key))?;
    
    // Create shared state
    let state = Arc::new(AppState {
        postgres: postgres_pool,
        neo4j: neo4j_pool,
        search: search_client,
        config: config.clone(),
    });

    // Build router
    let app = api::create_router(state)
        .layer(TraceLayer::new_for_http());

    // Start server
    let addr = config.bind_address;
    tracing::info!("🚀 Server listening on {}", addr);
    
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

#[derive(Clone)]
pub struct AppState {
    pub postgres: sqlx::Pool<sqlx::Postgres>,
    pub neo4j: neo4rs::Pool,
    pub search: meilisearch_sdk::client::Client,
    pub config: Config,
}

#[derive(Clone)]
pub struct Config {
    pub bind_address: String,
    pub database_url: String,
    pub neo4j_url: String,
    pub meilisearch_url: String,
    pub meilisearch_key: String,
}

impl Config {
    fn from_env() -> Result<Self> {
        Ok(Self {
            bind_address: std::env::var("BIND_ADDRESS").unwrap_or_else(|_| "0.0.0.0:8000".into()),
            database_url: std::env::var("DATABASE_URL")?,
            neo4j_url: std::env::var("NEO4J_URL")?,
            meilisearch_url: std::env::var("MEILISEARCH_URL")?,
            meilisearch_key: std::env::var("MEILISEARCH_KEY")?,
        })
    }
}
