//! Data ingestion pipelines

use crate::plugin_bridge::{call_plugin_method, list_plugins};
use crate::AppState;
use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;

#[derive(Debug, Serialize, Deserialize)]
pub struct EntityData {
    pub id: String,
    pub name: String,
    pub entity_type: String,
    pub aliases: Vec<String>,
    pub properties: serde_json::Value,
    pub sources: Vec<String>,
    pub created_at: Option<String>,
}

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
    State(state): State<Arc<AppState>>,
    Json(payload): Json<IngestionRequest>,
) -> Result<Json<IngestionResponse>, (StatusCode, String)> {
    tracing::info!("Ingesting data from source: {}", payload.source);
    
    // Validate the source exists
    let plugins = list_plugins().map_err(|e| {
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
    let result = call_plugin_method(&payload.source, "fetch", &payload.query).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to call plugin: {}", e),
        )
    })?;
    
    if result.success {
        // Parse and store the data in the database
        let entities_data = result.data.clone().unwrap_or(Value::Array(vec![]));
        
        if let Value::Array(entities) = entities_data {
            tracing::info!("Attempting to store {} entities", entities.len());
            
            for entity_value in entities {
                match serde_json::from_value::<EntityData>(entity_value.clone()) {
                    Ok(entity) => {
                        tracing::info!("Inserting entity: {} ({})", entity.id, entity.name);
                        
                        let insert_sql = r#"
                            INSERT INTO entities (id, name, entity_type, aliases, properties, sources, created_at)
                            VALUES ($1, $2, $3, $4, $5, $6, COALESCE($7::timestamp, NOW()))
                            ON CONFLICT (id) DO UPDATE SET
                                name = EXCLUDED.name,
                                entity_type = EXCLUDED.entity_type,
                                aliases = EXCLUDED.aliases,
                                properties = EXCLUDED.properties,
                                sources = EXCLUDED.sources,
                                updated_at = NOW()
                        "#;
                        
                        let created_at = entity.created_at.and_then(|s| s.parse::<chrono::NaiveDateTime>().ok());
                        
                        match sqlx::query(insert_sql)
                            .bind(&entity.id)
                            .bind(&entity.name)
                            .bind(&entity.entity_type)
                            .bind(&entity.aliases)
                            .bind(sqlx::types::Json(&entity.properties))
                            .bind(&entity.sources)
                            .bind(created_at)
                            .execute(&state.postgres)
                            .await
                        {
                            Ok(_) => tracing::info!("Successfully inserted {}", entity.id),
                            Err(e) => tracing::error!("Failed to insert {}: {}", entity.id, e),
                        }
                    }
                    Err(e) => tracing::error!("Failed to parse entity: {}", e),
                }
            }
        }
        
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
    let plugins = list_plugins().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to list plugins: {}", e),
        )
    })?;
    
    Ok(Json(plugins))
}