//! Le Coq Backend - Library Root
//! Re-exports all modules for testing

pub mod api;
pub mod auth;
pub mod config;
pub mod db;
pub mod dedup;
pub mod graph;
pub mod ingestion;
pub mod models;
pub mod plugin_bridge;
pub mod poller;

pub use plugin_bridge::*;

#[derive(Clone)]
pub struct AppState {
    pub postgres: sqlx::Pool<sqlx::Postgres>,
    pub config: Config,
    pub dedup: crate::dedup::InflightRequests,
}

impl AppState {
    pub fn new(postgres: sqlx::Pool<sqlx::Postgres>, config: Config) -> Self {
        Self {
            postgres,
            config,
            dedup: crate::dedup::InflightRequests::new(),
        }
    }
}

#[derive(Clone)]
pub struct Config {
    pub bind_address: String,
    pub database_url: String,
    pub api_key_header: String,
    pub auth: crate::auth::AuthMode,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        let bind_address =
            std::env::var("BIND_ADDRESS").unwrap_or_else(|_| "127.0.0.1:8000".into());
        let auth = crate::auth::AuthMode::from_settings(
            std::env::var("DISABLE_AUTH").ok().as_deref(),
            std::env::var("LE_COQ_API_KEY").ok(),
            &bind_address,
        )?;
        Ok(Self {
            bind_address,
            database_url: std::env::var("DATABASE_URL")?,
            api_key_header: std::env::var("API_KEY_HEADER").unwrap_or_else(|_| "X-API-Key".into()),
            auth,
        })
    }
}
