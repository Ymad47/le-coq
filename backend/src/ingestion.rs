//! Data ingestion pipelines

use crate::db;
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

        if let Value::Array(items) = entities_data {
            tracing::info!("Attempting to store {} items", items.len());

            for item in items {
                // Distingue entité vs relation par présence de from_entity/to_entity
                let is_relation =
                    item.get("from_entity").is_some() && item.get("to_entity").is_some();

                if is_relation {
                    let from = item
                        .get("from_entity")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    let to = item.get("to_entity").and_then(|v| v.as_str()).unwrap_or("");
                    let rel_type = item
                        .get("relationship_type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("related_to");
                    let confidence = item
                        .get("confidence")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(1.0) as f32;
                    let properties = item
                        .get("properties")
                        .cloned()
                        .unwrap_or(Value::Object(serde_json::Map::new()));
                    let sources: Vec<String> = item
                        .get("sources")
                        .and_then(|v| v.as_array())
                        .map(|a| {
                            a.iter()
                                .filter_map(|x| x.as_str().map(String::from))
                                .collect()
                        })
                        .unwrap_or_default();

                    // Stub endpoints to satisfy FK (insert_stub via db module)
                    for endpoint in [from, to] {
                        if endpoint.is_empty() {
                            continue;
                        }
                        let _ = db::entities::insert_stub(
                            &state.postgres,
                            endpoint,
                            &stub_name_from_id(endpoint),
                            &sources,
                        )
                        .await;
                    }

                    let rel = db::relationships::insert(
                        &state.postgres,
                        from,
                        to,
                        rel_type,
                        &properties,
                        Some(confidence),
                        &sources,
                    )
                    .await;
                    match rel {
                        Ok((_id, true)) => {
                            tracing::info!("Relation: {} -[{}]-> {}", from, rel_type, to)
                        }
                        Ok((_id, false)) => tracing::debug!(
                            "Relation exists, skipped: {} -[{}]-> {}",
                            from,
                            rel_type,
                            to
                        ),
                        Err(e) => {
                            tracing::error!("Failed to insert relation {}->{}: {}", from, to, e)
                        }
                    }
                    continue;
                }

                match serde_json::from_value::<EntityData>(item.clone()) {
                    Ok(entity) => {
                        tracing::info!("Inserting entity: {} ({})", entity.id, entity.name);

                        let created_at = entity
                            .created_at
                            .and_then(|s| s.parse::<chrono::NaiveDateTime>().ok());

                        match db::entities::upsert(
                            &state.postgres,
                            &entity.id,
                            &entity.name,
                            &entity.entity_type,
                            &entity.aliases,
                            &entity.properties,
                            &entity.sources,
                            created_at,
                        )
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

/// Core ingestion logic reusable from other handlers (e.g. smart search).
/// Fetches from source, upserts entities/relations in DB.
/// Returns (count of inserted items, list of per-relation drop errors).
pub async fn ingest_from_source(
    state: Arc<AppState>,
    source: &str,
    query: Value,
) -> Result<(usize, Vec<String>), String> {
    let result = call_plugin_method(source, "fetch", &query)
        .map_err(|e| format!("Failed to call plugin {}: {}", source, e))?;

    if !result.success {
        return Err(result
            .error
            .unwrap_or_else(|| "Plugin execution failed".to_string()));
    }

    let items = result.data.unwrap_or(Value::Array(vec![]));
    let Value::Array(items) = items else {
        return Ok((0, Vec::new()));
    };

    let mut inserted = 0usize;
    let mut errors: Vec<String> = Vec::new();

    for item in items {
        let is_relation = item.get("from_entity").is_some() && item.get("to_entity").is_some();

        if is_relation {
            let from = item
                .get("from_entity")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let to = item.get("to_entity").and_then(|v| v.as_str()).unwrap_or("");
            let rel_type = item
                .get("relationship_type")
                .and_then(|v| v.as_str())
                .unwrap_or("related_to");
            let confidence = item
                .get("confidence")
                .and_then(|v| v.as_f64())
                .unwrap_or(1.0) as f32;
            let properties = item
                .get("properties")
                .cloned()
                .unwrap_or(Value::Object(serde_json::Map::new()));
            let sources: Vec<String> = item
                .get("sources")
                .and_then(|v| v.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|x| x.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();

            // Ensure endpoint entities exist (FK constraint on relationships).
            for endpoint in [from, to] {
                if endpoint.is_empty() {
                    continue;
                }
                let name = stub_name_from_id(endpoint);
                let _ = db::entities::insert_stub(&state.postgres, endpoint, &name, &sources).await;
            }

            let rel = db::relationships::insert(
                &state.postgres,
                from,
                to,
                rel_type,
                &properties,
                Some(confidence),
                &sources,
            )
            .await;
            match rel {
                Ok((_id, inserted_flag)) => {
                    if inserted_flag {
                        inserted += 1;
                    }
                }
                Err(e) => {
                    let msg = format!("relation {} -> {}: {}", from, to, e);
                    tracing::warn!("ingest relation dropped: {}", msg);
                    errors.push(msg);
                }
            }
            continue;
        }

        match serde_json::from_value::<EntityData>(item.clone()) {
            Ok(entity) => {
                let created_at = entity
                    .created_at
                    .and_then(|s| s.parse::<chrono::NaiveDateTime>().ok());

                if db::entities::upsert(
                    &state.postgres,
                    &entity.id,
                    &entity.name,
                    &entity.entity_type,
                    &entity.aliases,
                    &entity.properties,
                    &entity.sources,
                    created_at,
                )
                .await
                .is_ok()
                {
                    inserted += 1;
                }
            }
            Err(e) => tracing::error!("Failed to parse entity: {}", e),
        }
    }

    Ok((inserted, errors))
}

/// Human-readable stub name derived from a plugin-generated entity id.
/// e.g. "wiki_South_Africa" -> "South Africa", "wd_item_Q142" -> "Q142".
fn stub_name_from_id(id: &str) -> String {
    let stripped = id
        .strip_prefix("wiki_")
        .map(|s| s.replace('_', " "))
        .or_else(|| id.strip_prefix("wd_item_").map(|s| s.to_string()))
        .or_else(|| {
            id.strip_prefix("eurostat_country_")
                .map(|s| s.replace('_', " "))
        })
        .unwrap_or_else(|| id.to_string());
    let trimmed = stripped.trim();
    if trimmed.is_empty() {
        id.to_string()
    } else {
        trimmed.to_string()
    }
}
