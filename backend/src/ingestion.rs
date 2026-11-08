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
    /// When the source published it (ISO 8601 / RFC 3339). Feeds the timeline.
    #[serde(default)]
    pub published_at: Option<String>,
}

/// Parses an RFC 3339 timestamp ("2026-03-01T00:30:00Z"); None if absent or invalid.
pub fn parse_published_at(s: Option<&str>) -> Option<chrono::DateTime<chrono::Utc>> {
    chrono::DateTime::parse_from_rfc3339(s?)
        .ok()
        .map(|d| d.with_timezone(&chrono::Utc))
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
    /// Entities upserted + relationships newly inserted.
    #[serde(default)]
    pub inserted: usize,
    /// Items that could not be stored (parse or database errors).
    #[serde(default)]
    pub errors: Vec<String>,
}

/// Runs a synchronous PyO3 call on the blocking thread pool.
/// Plugin calls hold the GIL and run their own asyncio loop for seconds; on a
/// tokio worker they would stall every other request scheduled on that thread,
/// and `tokio::time::timeout` could not fire until they returned.
pub(crate) async fn run_python<T, F>(f: F) -> Result<T, String>
where
    F: FnOnce() -> Result<T, String> + Send + 'static,
    T: Send + 'static,
{
    tokio::task::spawn_blocking(f)
        .await
        .map_err(|e| format!("plugin task failed: {}", e))?
}

/// Calls `<source>.fetch(query)` and returns the items it produced.
async fn fetch_items(source: &str, query: Value) -> Result<Vec<Value>, String> {
    let src = source.to_string();
    let result = run_python(move || call_plugin_method(&src, "fetch", &query))
        .await
        .map_err(|e| format!("Failed to call plugin {}: {}", source, e))?;

    if !result.success {
        return Err(result
            .error
            .unwrap_or_else(|| "Plugin execution failed".to_string()));
    }
    match result.data {
        Some(Value::Array(items)) => Ok(items),
        Some(Value::Null) | None => Ok(Vec::new()),
        Some(other) => Err(format!(
            "plugin {} returned {} instead of a list",
            source,
            json_type_name(&other)
        )),
    }
}

fn json_type_name(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "a boolean",
        Value::Number(_) => "a number",
        Value::String(_) => "a string",
        Value::Array(_) => "a list",
        Value::Object(_) => "an object",
    }
}

fn string_list(item: &Value, key: &str) -> Vec<String> {
    item.get(key)
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

/// Stores plugin items: an item with `from_entity` + `to_entity` is a
/// relationship, anything else must parse as an [`EntityData`].
/// Returns the number of rows written and one message per item that failed.
pub async fn store_items(state: &AppState, items: &[Value]) -> (usize, Vec<String>) {
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
            if from.is_empty() || to.is_empty() {
                errors.push("relation with an empty endpoint skipped".to_string());
                continue;
            }
            let rel_type = item
                .get("relationship_type")
                .and_then(|v| v.as_str())
                .unwrap_or("related_to");
            // Absent confidence stays NULL ("unknown"), see migration 003.
            let confidence = item
                .get("confidence")
                .and_then(|v| v.as_f64())
                .map(|c| c as f32);
            let properties = item
                .get("properties")
                .cloned()
                .unwrap_or(Value::Object(serde_json::Map::new()));
            let sources = string_list(item, "sources");

            // Ensure endpoint entities exist (FK constraint on relationships).
            for endpoint in [from, to] {
                let name = stub_name_from_id(endpoint);
                if let Err(e) =
                    db::entities::insert_stub(&state.postgres, endpoint, &name, &sources).await
                {
                    tracing::warn!("stub entity {} not created: {}", endpoint, e);
                }
            }

            match db::relationships::insert(
                &state.postgres,
                from,
                to,
                rel_type,
                &properties,
                confidence,
                &sources,
            )
            .await
            {
                Ok((_id, true)) => inserted += 1,
                Ok((_id, false)) => {} // already known
                Err(e) => {
                    let msg = format!("relation {} -[{}]-> {}: {}", from, rel_type, to, e);
                    tracing::warn!("ingest relation dropped: {}", msg);
                    errors.push(msg);
                }
            }
            continue;
        }

        let entity = match serde_json::from_value::<EntityData>(item.clone()) {
            Ok(e) => e,
            Err(e) => {
                let id = item.get("id").and_then(|v| v.as_str()).unwrap_or("?");
                let msg = format!("entity {}: invalid shape: {}", id, e);
                tracing::warn!("ingest entity dropped: {}", msg);
                errors.push(msg);
                continue;
            }
        };
        let created_at = entity
            .created_at
            .as_deref()
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
            parse_published_at(entity.published_at.as_deref()),
        )
        .await
        {
            Ok(_) => inserted += 1,
            Err(e) => {
                let msg = format!("entity {}: {}", entity.id, e);
                tracing::warn!("ingest entity dropped: {}", msg);
                errors.push(msg);
            }
        }
    }

    (inserted, errors)
}

/// POST /api/v1/ingest — fetch from one plugin and store the result.
pub async fn ingest_data(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<IngestionRequest>,
) -> Result<Json<IngestionResponse>, (StatusCode, String)> {
    tracing::info!("Ingesting data from source: {}", payload.source);

    let plugins = run_python(list_plugins).await.map_err(|e| {
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

    // A plugin error (bad credentials, upstream failure) is not a server bug.
    let items = fetch_items(&payload.source, payload.query)
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, e))?;

    tracing::info!("Attempting to store {} items", items.len());
    let (inserted, errors) = store_items(&state, &items).await;
    if !errors.is_empty() {
        tracing::warn!(
            "{} of {} items from {} not stored",
            errors.len(),
            items.len(),
            payload.source
        );
    }

    Ok(Json(IngestionResponse {
        success: true,
        message: if errors.is_empty() {
            "Data ingested successfully".to_string()
        } else {
            format!("Data ingested with {} errors", errors.len())
        },
        data: Some(Value::Array(items)),
        inserted,
        errors,
    }))
}

pub async fn list_sources(
    State(_state): State<Arc<AppState>>,
) -> Result<Json<Vec<String>>, (StatusCode, String)> {
    let plugins = run_python(list_plugins).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to list plugins: {}", e),
        )
    })?;
    Ok(Json(plugins))
}

/// Fetch from one plugin and store the result. Used by online search and the
/// timeline. Returns rows written and per-item errors.
pub async fn ingest_from_source(
    state: Arc<AppState>,
    source: &str,
    query: Value,
) -> Result<(usize, Vec<String>), String> {
    let items = fetch_items(source, query).await?;
    Ok(store_items(&state, &items).await)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_utc_timestamp() {
        let d = parse_published_at(Some("2026-03-01T00:30:00Z")).unwrap();
        assert_eq!(d.to_rfc3339(), "2026-03-01T00:30:00+00:00");
    }

    #[test]
    fn normalizes_offset_to_utc() {
        let d = parse_published_at(Some("2026-03-01T02:30:00+02:00")).unwrap();
        assert_eq!(d.to_rfc3339(), "2026-03-01T00:30:00+00:00");
    }

    #[test]
    fn rejects_missing_or_invalid() {
        assert!(parse_published_at(None).is_none());
        assert!(parse_published_at(Some("")).is_none());
        assert!(parse_published_at(Some("20260301T003000Z")).is_none()); // raw GDELT format
        assert!(parse_published_at(Some("yesterday")).is_none());
    }
}
