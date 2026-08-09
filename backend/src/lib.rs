//! Le Coq Backend - Library Root
//! Re-exports all modules for testing

pub mod api;
pub mod db;
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
}

#[derive(Clone)]
pub struct Config {
    pub bind_address: String,
    pub database_url: String,
    pub api_key_header: String,
}
