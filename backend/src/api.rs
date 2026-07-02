//! API Routes

use axum::{
    routing::{get, post},
    Router,
    extract::State,
    response::Json,
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::sync::Arc;

use crate::AppState;

pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/health", get(health_check))
        .route("/api/v1/entities", get(list_entities).post(create_entity))
        .route("/api/v1/entities/:id", get(get_entity))
        .route("/api/v1/relationships", get(list_relationships).post(create_relationship))
        .route("/api/v1/search", post(search))
        .route("/api/v1/ingest", post(ingest_data))
        .with_state(state)
}

async fn health_check() -> &'static str {
    "OK"
}

#[derive(Deserialize)]
struct EntityQuery {
    #[serde(default)]
    r#type: Option<String>,
    #[serde(default)]
    q: Option<String>,
    #[serde(default)]
    limit: Option<u32>,
}

#[derive(Serialize)]
struct EntityResponse {
    id: String,
    name: String,
    r#type: String,
    properties: serde_json::Value,
}

async fn list_entities(
    State(state): State<Arc<AppState>>,
    query: axum::extract::Query<EntityQuery>,
) -> Result<Json<Vec<EntityResponse>>, StatusCode> {
    let sql = "SELECT id, name, entity_type, properties FROM entities ORDER BY created_at DESC LIMIT $1";
    let limit = query.limit.unwrap_or(100).min(1000) as i32;
    
    let rows = sqlx::query(sql)
        .bind(limit)
        .fetch_all(&state.postgres)
        .await
        .map_err(|e| {
            tracing::error!("DB error: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    
    let entities: Vec<EntityResponse> = rows.iter().map(|row| {
        EntityResponse {
            id: row.get("id"),
            name: row.get("name"),
            r#type: row.get("entity_type"),
            properties: row.get("properties"),
        }
    }).collect();
    
    Ok(Json(entities))
}

async fn create_entity(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<EntityResponse>, StatusCode> {
    let name = payload.get("name").and_then(|v| v.as_str()).ok_or(StatusCode::BAD_REQUEST)?;
    let entity_type = payload.get("entity_type").and_then(|v| v.as_str()).ok_or(StatusCode::BAD_REQUEST)?;
    
    let aliases_vec: Vec<String> = payload.get("aliases")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str()).map(|s| s.to_string()).collect())
        .unwrap_or_default();
    
    let properties = payload.get("properties").cloned().unwrap_or(serde_json::Value::Object(serde_json::Map::new()));
    
    let sources_vec: Vec<String> = payload.get("sources")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str()).map(|s| s.to_string()).collect())
        .unwrap_or_default();
    
    let row = sqlx::query(
        r#"
        INSERT INTO entities (name, entity_type, aliases, properties, sources)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id
        "#
    )
    .bind(name)
    .bind(entity_type)
    .bind(&aliases_vec)
    .bind(&properties)
    .bind(&sources_vec)
    .fetch_one(&state.postgres)
    .await
    .map_err(|e| {
        tracing::error!("Database error: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    
    let id: String = row.get("id");
    
    let entity = EntityResponse {
        id,
        name: name.to_string(),
        r#type: entity_type.to_string(),
        properties,
    };
    
    Ok(Json(entity))
}

async fn get_entity(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<EntityResponse>, StatusCode> {
    let row = sqlx::query("SELECT id, name, entity_type, properties FROM entities WHERE id = $1")
        .bind(&id)
        .fetch_optional(&state.postgres)
        .await
        .map_err(|e| {
            tracing::error!("DB error: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    
    row.map(|r| {
        Json(EntityResponse {
            id: r.get("id"),
            name: r.get("name"),
            r#type: r.get("entity_type"),
            properties: r.get("properties"),
        })
    }).ok_or(StatusCode::NOT_FOUND)
}

async fn list_relationships(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<serde_json::Value>>, StatusCode> {
    let rows = sqlx::query("SELECT id, from_entity, to_entity, relationship_type, properties, confidence FROM relationships LIMIT 100")
        .fetch_all(&state.postgres)
        .await
        .map_err(|e| {
            tracing::error!("DB error: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    
    let rels: Vec<serde_json::Value> = rows.iter().map(|row| {
        serde_json::json!({
            "id": row.get::<String, _>("id"),
            "from_entity": row.get::<String, _>("from_entity"),
            "to_entity": row.get::<String, _>("to_entity"),
            "relationship_type": row.get::<String, _>("relationship_type"),
            "properties": row.get::<serde_json::Value, _>("properties"),
            "confidence": row.get::<f32, _>("confidence"),
        })
    }).collect();
    
    Ok(Json(rels))
}

async fn create_relationship(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let from_entity = payload.get("from_entity").and_then(|v| v.as_str()).ok_or(StatusCode::BAD_REQUEST)?;
    let to_entity = payload.get("to_entity").and_then(|v| v.as_str()).ok_or(StatusCode::BAD_REQUEST)?;
    let rel_type = payload.get("relationship_type").and_then(|v| v.as_str()).ok_or(StatusCode::BAD_REQUEST)?;
    let confidence = payload.get("confidence").and_then(|v| v.as_f64()).unwrap_or(1.0);
    let properties = payload.get("properties").cloned().unwrap_or(serde_json::Value::Object(serde_json::Map::new()));
    
    let row = sqlx::query(
        "INSERT INTO relationships (from_entity, to_entity, relationship_type, properties, confidence)
         VALUES ($1, $2, $3, $4, $5) RETURNING id"
    )
    .bind(from_entity)
    .bind(to_entity)
    .bind(rel_type)
    .bind(&properties)
    .bind(confidence as f32)
    .fetch_one(&state.postgres)
    .await
    .map_err(|e| {
        tracing::error!("DB error: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    
    Ok(Json(serde_json::json!({
        "id": row.get::<String, _>("id"),
        "from_entity": from_entity,
        "to_entity": to_entity,
        "relationship_type": rel_type,
        "confidence": confidence,
    })))
}

#[derive(Deserialize)]
struct SearchRequest {
    query: String,
    #[serde(default)]
    filters: Option<serde_json::Value>,
}

async fn search(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<SearchRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let q = &payload.query;
    
    let rows = sqlx::query(
        "SELECT id, name, entity_type, properties, 
                ts_rank(to_tsvector('english', name), plainto_tsquery('english', $1)) as rank
         FROM entities 
         WHERE to_tsvector('english', name) @@ plainto_tsquery('english', $1)
         ORDER BY rank DESC
         LIMIT 50"
    )
    .bind(q)
    .fetch_all(&state.postgres)
    .await
    .map_err(|e| {
        tracing::error!("Search DB error: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    
    let results: Vec<serde_json::Value> = rows.iter().map(|row| {
        serde_json::json!({
            "id": row.get::<String, _>("id"),
            "name": row.get::<String, _>("name"),
            "entity_type": row.get::<String, _>("entity_type"),
            "properties": row.get::<serde_json::Value, _>("properties"),
            "rank": row.get::<f32, _>("rank"),
        })
    }).collect();
    
    Ok(Json(serde_json::json!({
        "query": q,
        "count": results.len(),
        "results": results,
    })))
}

async fn ingest_data(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let source = payload.get("source").and_then(|v| v.as_str()).ok_or(StatusCode::BAD_REQUEST)?;
    let query_params = payload.get("query").cloned().unwrap_or(serde_json::Value::Object(serde_json::Map::new()));
    
    tracing::info!("Ingest request: source={}, query={}", source, query_params);
    
    Ok(Json(serde_json::json!({
        "status": "queued",
        "source": source,
        "query": query_params,
        "message": "Ingestion pipeline triggered (stub - implement Python plugin call)"
    })))
}
