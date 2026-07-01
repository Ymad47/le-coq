//! API Routes

use axum::{
    routing::{get, post},
    Router,
    extract::State,
    response::Json,
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
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
    // TODO: Implement database query
    Ok(Json(vec![]))
}

async fn create_entity(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<EntityResponse>, StatusCode> {
    use sqlx::Row;
    
    // Extract required fields from payload
    let name = payload.get("name").and_then(|v| v.as_str()).ok_or(StatusCode::BAD_REQUEST)?;
    let entity_type = payload.get("entity_type").and_then(|v| v.as_str()).ok_or(StatusCode::BAD_REQUEST)?;
    
    // Extract optional fields
    let aliases = payload.get("aliases").and_then(|v| v.as_array()).unwrap_or(&vec![]);
    let properties = payload.get("properties").unwrap_or(&serde_json::Value::Object(serde_json::Map::new()));
    let sources = payload.get("sources").and_then(|v| v.as_array()).unwrap_or(&vec![]);
    
    // Convert aliases to Vec<String>
    let aliases_vec: Vec<String> = aliases.iter()
        .filter_map(|v| v.as_str())
        .map(|s| s.to_string())
        .collect();
    
    // Convert sources to Vec<String>
    let sources_vec: Vec<String> = sources.iter()
        .filter_map(|v| v.as_str())
        .map(|s| s.to_string())
        .collect();
    
    // Insert into database
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
    .bind(properties)
    .bind(&sources_vec)
    .fetch_one(&state.postgres)
    .await
    .map_err(|e| {
        tracing::error!("Database error: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    
    let id: String = row.get("id");
    
    // Return the created entity
    let entity = EntityResponse {
        id,
        name: name.to_string(),
        r#type: entity_type.to_string(),
        properties: properties.clone(),
    };
    
    Ok(Json(entity))
}

async fn get_entity(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<EntityResponse>, StatusCode> {
    // TODO: Implement entity retrieval
    Err(StatusCode::NOT_IMPLEMENTED)
}

async fn list_relationships(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<serde_json::Value>>, StatusCode> {
    // TODO: Implement relationship listing
    Ok(Json(vec![]))
}

async fn create_relationship(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // TODO: Implement relationship creation
    Err(StatusCode::NOT_IMPLEMENTED)
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
    // TODO: Implement search across all databases
    Err(StatusCode::NOT_IMPLEMENTED)
}

async fn ingest_data(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // TODO: Implement data ingestion pipeline trigger
    Err(StatusCode::NOT_IMPLEMENTED)
}
