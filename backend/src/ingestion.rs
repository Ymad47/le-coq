//! Data ingestion pipelines

use crate::plugin_bridge::{call_python_plugin, list_python_plugins};
use crate::AppState;
use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;

#[derive(Debug, Serialize, Deserialize)]
pub struct IngestionRequest {
    pub source: String,
    pub query: Value,
    pub deduplicate: Option<bool>,
    pub rate_limit: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IngestionResponse {
    pub success: bool,
    pub message: String,
    pub data: Option<Value>,
}

pub async fn ingest_data(
    State(_state): State<Arc<AppState>>,
    Json(payload): Json<IngestionRequest>,
) -> Result<Json<IngestionResponse>, (StatusCode, String)> {
    tracing::info!("Ingesting data from source: {}", payload.source);
    
    // Validate the source exists
    let plugins = list_python_plugins().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to list plugins: {}", e),
        )
    })?;
    
    if !plugins.contains(&payload.source) {
        return Err((
            StatusCode::BAD_REQUEST,
            format!("Unknown source: {}", payload.source),
        ));
    }
    
    // Call the Python plugin
    let result = call_python_plugin(&payload.source, &payload.query).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to call plugin: {}", e),
        )
    })?;
    
    if result.success {
        // In a real implementation, we would store the data in the database here
        // For now, we'll just return the data from the plugin
        
        Ok(Json(IngestionResponse {
            success: true,
            message: "Data ingested successfully".to_string(),
            data: result.data,
        }))
    } else {
        Err((
            StatusCode::BAD_REQUEST,
            result
                .error
                .unwrap_or_else(|| "Plugin execution failed".to_string()),
        ))
    }
}

pub async fn list_sources(
    State(_state): State<Arc<AppState>>,
) -> Result<Json<Vec<String>>, (StatusCode, String)> {
    let plugins = list_python_plugins().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to list plugins: {}", e),
        )
    })?;
    
    Ok(Json(plugins))
}