//! API Routes — thin HTTP layer over db modules.
//! Handlers should: parse input → call db fn → format response. No SQL here.

use axum::{
    extract::State,
    http::StatusCode,
    middleware,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::db;
use crate::ingestion::{ingest_data, ingest_from_source, list_sources};
use crate::AppState;

pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/health", get(health_check))
        .route("/api/v1/entities", get(list_entities).post(create_entity))
        .route("/api/v1/entities/:id", get(get_entity))
        .route("/api/v1/entities/:id/summary", get(entity_summary))
        .route(
            "/api/v1/relationships",
            get(list_relationships).post(create_relationship),
        )
        .route("/api/v1/search", get(smart_search).post(search))
        .route("/api/v1/topic", get(topic_search))
        .route("/api/v1/graph/neighborhood", post(graph_neighborhood))
        .route("/api/v1/timeline", get(timeline))
        .route("/api/v1/ingest", post(ingest_data))
        .route("/api/v1/sources", get(list_sources))
        .route("/api/v1/plugins/health", get(plugins_health))
        .route("/api/v1/plugins/reload", post(plugins_reload))
        .layer(middleware::from_fn_with_state(state.clone(), api_key_auth))
        .with_state(state)
}

async fn health_check() -> &'static str {
    "Tout fonctionne  Bg !"
}

async fn api_key_auth(
    State(state): State<Arc<AppState>>,
    req: axum::http::Request<axum::body::Body>,
    next: axum::middleware::Next,
) -> Result<axum::response::Response, StatusCode> {
    if req.uri().path() == "/health" {
        return Ok(next.run(req).await);
    }

    let provided = req
        .headers()
        .get(crate::auth::API_KEY_HEADER)
        .map(|v| v.as_bytes());

    if state.config.auth.allows(provided) {
        Ok(next.run(req).await)
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}

#[cfg(test)]
mod auth_middleware_tests {
    use super::*;
    use crate::auth::AuthMode;
    use crate::{AppState, Config};
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    const KEY: &str = "0123456789abcdef0123456789abcdef";

    /// Real middleware in front of a trivial handler; the DB pool is lazy and never used.
    fn app() -> Router {
        let pool = sqlx::postgres::PgPoolOptions::new()
            .connect_lazy("postgres://unused@127.0.0.1:1/unused")
            .expect("lazy pool");
        let config = Config {
            bind_address: "127.0.0.1:0".into(),
            database_url: String::new(),
            auth: AuthMode::from_settings(None, Some(KEY.into()), "127.0.0.1:0").unwrap(),
        };
        let state = Arc::new(AppState::new(pool, config));
        Router::new()
            .route("/health", get(|| async { "ok" }))
            .route("/api/v1/protected", get(|| async { "ok" }))
            .layer(middleware::from_fn_with_state(state.clone(), api_key_auth))
            .with_state(state)
    }

    async fn status(path: &str, key: Option<&str>) -> StatusCode {
        let mut req = Request::builder().uri(path);
        if let Some(k) = key {
            req = req.header("X-API-Key", k);
        }
        app()
            .oneshot(req.body(Body::empty()).unwrap())
            .await
            .unwrap()
            .status()
    }

    #[tokio::test]
    async fn health_needs_no_key() {
        assert_eq!(status("/health", None).await, StatusCode::OK);
    }

    #[tokio::test]
    async fn missing_key_is_401() {
        assert_eq!(
            status("/api/v1/protected", None).await,
            StatusCode::UNAUTHORIZED
        );
    }

    #[tokio::test]
    async fn wrong_key_is_401() {
        assert_eq!(
            status("/api/v1/protected", Some("not-the-key")).await,
            StatusCode::UNAUTHORIZED
        );
    }

    #[tokio::test]
    async fn correct_key_passes() {
        assert_eq!(status("/api/v1/protected", Some(KEY)).await, StatusCode::OK);
    }
}

// ============================================================
// GET/POST /api/v1/entities
// ============================================================

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

impl From<db::entities::EntityRow> for EntityResponse {
    fn from(r: db::entities::EntityRow) -> Self {
        Self {
            id: r.id,
            name: r.name,
            r#type: r.entity_type,
            properties: r.properties,
        }
    }
}

async fn list_entities(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(query): axum::extract::Query<EntityQuery>,
) -> Result<Json<Vec<EntityResponse>>, StatusCode> {
    let filters = db::entities::EntityFilters {
        entity_type: query.r#type,
        search: query.q,
        limit: query.limit.unwrap_or(100).min(1000) as i32,
    };

    let rows = db::entities::list(&state.postgres, filters)
        .await
        .map_err(|e| {
            tracing::error!("DB error: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(rows.into_iter().map(EntityResponse::from).collect()))
}

async fn create_entity(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<EntityResponse>, StatusCode> {
    let name = payload
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or(StatusCode::BAD_REQUEST)?;
    let entity_type = payload
        .get("entity_type")
        .and_then(|v| v.as_str())
        .ok_or(StatusCode::BAD_REQUEST)?;

    let aliases_vec: Vec<String> = payload
        .get("aliases")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str())
                .map(String::from)
                .collect()
        })
        .unwrap_or_default();

    let properties = payload
        .get("properties")
        .cloned()
        .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));

    let sources_vec: Vec<String> = payload
        .get("sources")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str())
                .map(String::from)
                .collect()
        })
        .unwrap_or_default();

    let id = payload
        .get("id")
        .and_then(|v| v.as_str())
        .map(String::from)
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    // Reserved namespace guard
    const RESERVED_PREFIXES: &[&str] = &[
        "wiki_",
        "wd_item_",
        "acled_",
        "gdelt_",
        "eurostat_country_",
        "gleif_",
        "uncomtrade_",
        "oc_",
    ];
    if payload.get("id").is_some() && RESERVED_PREFIXES.iter().any(|p| id.starts_with(p)) {
        tracing::warn!("create_entity rejected: reserved id prefix '{}'", id);
        return Err(StatusCode::BAD_REQUEST);
    }

    db::entities::insert(
        &state.postgres,
        &id,
        name,
        entity_type,
        &aliases_vec,
        &properties,
        &sources_vec,
    )
    .await
    .map_err(|e| {
        tracing::error!("Database error: {}", e);
        if e.to_string().contains("duplicate key") {
            StatusCode::CONFLICT
        } else {
            StatusCode::INTERNAL_SERVER_ERROR
        }
    })?;

    Ok(Json(EntityResponse {
        id,
        name: name.to_string(),
        r#type: entity_type.to_string(),
        properties,
    }))
}

async fn get_entity(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<EntityResponse>, StatusCode> {
    let row = db::entities::get_by_id(&state.postgres, &id)
        .await
        .map_err(|e| {
            tracing::error!("DB error: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    row.map(|r| Json(EntityResponse::from(r)))
        .ok_or(StatusCode::NOT_FOUND)
}

// ============================================================
// GET /api/v1/entities/:id/summary
// ============================================================

async fn entity_summary(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let row = db::entities::get_by_id(&state.postgres, &id)
        .await
        .map_err(|e| {
            tracing::error!("entity fetch error: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "DB error".into())
        })?
        .ok_or((StatusCode::NOT_FOUND, "entity not found".into()))?;

    let description = row
        .properties
        .get("description")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let type_breakdown = db::summary::type_breakdown(&state.postgres, &id)
        .await
        .map_err(|e| {
            tracing::error!("type breakdown error: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "DB error".into())
        })?;

    let rel_types = db::summary::top_rel_types(&state.postgres, &id)
        .await
        .map_err(|e| {
            tracing::error!("rel types error: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "DB error".into())
        })?;

    let top_neighbor = db::summary::top_neighbor(&state.postgres, &id)
        .await
        .map_err(|e| {
            tracing::error!("top neighbor error: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "DB error".into())
        })?;

    // Build narrative
    let mut sentences: Vec<String> = Vec::new();

    let first_sentence = description
        .split('.')
        .next()
        .map(|s| s.trim().to_string())
        .filter(|s| s.len() > 20);
    if let Some(s) = first_sentence {
        sentences.push(format!("{}.", s));
    } else {
        sentences.push(format!("{} ({}).", row.name, row.entity_type));
    }

    let total_links: i64 = type_breakdown.iter().map(|t| t.count).sum();
    if total_links > 0 {
        let breakdown_str: Vec<String> = type_breakdown
            .iter()
            .take(4)
            .map(|t| format!("{} {}", t.count, t.entity_type.to_lowercase()))
            .collect();
        sentences.push(format!(
            "Lié à {} entités ({}).",
            total_links,
            breakdown_str.join(", ")
        ));
    } else {
        sentences.push("Aucune relation connue pour le moment.".to_string());
    }

    if !rel_types.is_empty() {
        let types: Vec<String> = rel_types
            .iter()
            .map(|t| format!("{} ({})", t.relationship_type.replace('_', " "), t.count))
            .collect();
        sentences.push(format!("Types de liens: {}.", types.join(", ")));
    }

    if let Some(n) = top_neighbor {
        sentences.push(format!(
            "Lien le plus fort: {} (confiance {:.0}%).",
            n.name,
            n.weight * 50.0
        ));
    }

    Ok(Json(serde_json::json!({
        "id": id,
        "name": row.name,
        "summary": sentences.join(" "),
    })))
}

// ============================================================
// GET/POST /api/v1/relationships
// ============================================================

#[derive(Deserialize)]
struct RelQuery {
    #[serde(default)]
    limit: Option<u32>,
    #[serde(default)]
    offset: Option<u32>,
}

async fn list_relationships(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(query): axum::extract::Query<RelQuery>,
) -> Result<Json<Vec<serde_json::Value>>, StatusCode> {
    let limit = query.limit.unwrap_or(100).min(5000) as i32;
    let offset = query.offset.unwrap_or(0) as i32;

    let rows = db::relationships::list(&state.postgres, limit, offset)
        .await
        .map_err(|e| {
            tracing::error!("DB error: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let rels: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "id": r.id.to_string(),
                "from_entity": r.from_entity,
                "to_entity": r.to_entity,
                "relationship_type": r.relationship_type,
                "properties": r.properties,
                "confidence": r.confidence,
                "sources": r.sources,
            })
        })
        .collect();

    Ok(Json(rels))
}

async fn create_relationship(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let from_entity = payload
        .get("from_entity")
        .and_then(|v| v.as_str())
        .ok_or(StatusCode::BAD_REQUEST)?;
    let to_entity = payload
        .get("to_entity")
        .and_then(|v| v.as_str())
        .ok_or(StatusCode::BAD_REQUEST)?;
    let rel_type = payload
        .get("relationship_type")
        .and_then(|v| v.as_str())
        .ok_or(StatusCode::BAD_REQUEST)?;
    let confidence = payload
        .get("confidence")
        .and_then(|v| v.as_f64())
        .map(|c| c as f32);
    let properties = payload
        .get("properties")
        .cloned()
        .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));
    let sources_vec: Vec<String> = payload
        .get("sources")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str())
                .map(String::from)
                .collect()
        })
        .unwrap_or_default();

    let (id, was_inserted) = db::relationships::insert(
        &state.postgres,
        from_entity,
        to_entity,
        rel_type,
        &properties,
        confidence,
        &sources_vec,
    )
    .await
    .map_err(|e| {
        tracing::error!("DB error: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(serde_json::json!({
        "id": id.to_string(),
        "from_entity": from_entity,
        "to_entity": to_entity,
        "relationship_type": rel_type,
        "confidence": confidence,
        "inserted": was_inserted,
    })))
}

// ============================================================
// GET /api/v1/search (smart_search — DB + optional online)
// ============================================================

#[derive(Deserialize)]
struct SmartSearchQuery {
    q: String,
    #[serde(default)]
    online: Option<bool>,
    #[serde(default)]
    sources: Option<String>,
}

async fn smart_search(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(params): axum::extract::Query<SmartSearchQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let q = params.q.trim().to_string();
    let online = params.online.unwrap_or(false);
    let mut source_list: Vec<String> = params
        .sources
        .unwrap_or_default()
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    if online && source_list.is_empty() {
        source_list = vec!["wikipedia".into(), "wikidata".into()];
    }

    // 1) Local DB search
    let rows = db::search::search_entities(&state.postgres, &q)
        .await
        .map_err(|e| {
            tracing::error!("Smart search DB error: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "DB error".to_string())
        })?;

    let mut results: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "id": r.id,
                "name": r.name,
                "entity_type": r.entity_type,
                "properties": r.properties,
                "origin": "cache",
                "source": "db",
            })
        })
        .collect();

    let mut new_ingested = 0usize;
    let mut errors = Vec::new();

    // 2) Online fetch + persist (per-source 30s timeout + dedup across concurrent identical calls)
    if online {
        let mut handles = Vec::new();
        for src in source_list {
            let state_cl = state.clone();
            let q_cl = q.clone();
            let src_cl = src.clone();
            let dedup = state.dedup.clone();
            handles.push(tokio::spawn(async move {
                let key = format!("{}:{}", src_cl, q_cl);
                let src_name = src_cl.clone();
                let fut = dedup.call(key, move || {
                    let state_c = state_cl.clone();
                    let src_c = src_cl.clone();
                    let q_c = q_cl.clone();
                    async move {
                        let ingest_fut =
                            ingest_from_source(state_c, &src_c, serde_json::json!({"query": q_c}));
                        match tokio::time::timeout(std::time::Duration::from_secs(30), ingest_fut)
                            .await
                        {
                            Ok(Ok((count, errs))) => Ok(crate::plugin_bridge::PluginResult {
                                success: true,
                                data: Some(serde_json::json!({"inserted": count, "errors": errs})),
                                error: None,
                            }),
                            Ok(Err(e)) => Err(e),
                            Err(_) => Err(format!("source '{}' timed out after 30s", src_c)),
                        }
                    }
                });
                let (result, was_executed) = fut.await;
                (result, was_executed, src_name)
            }));
        }

        for handle in handles {
            match handle.await {
                Ok((Ok(result), was_executed, src)) => {
                    if let Some(data) = &result.data {
                        if let Some(count) = data.get("inserted").and_then(|v| v.as_u64()) {
                            new_ingested += count as usize;
                            if !was_executed {
                                tracing::info!(
                                    "dedup hit: plugin='{}' result shared from in-flight request",
                                    src
                                );
                            }
                        }
                    }
                }
                Ok((Err(e), _, src)) => errors.push(format!("{}: {}", src, e)),
                Err(e) => errors.push(format!("task error: {}", e)),
            }
        }

        if new_ingested > 0 {
            let rows2 = db::search::search_entities(&state.postgres, &q)
                .await
                .map_err(|e| {
                    tracing::error!("Smart search refresh error: {}", e);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "DB refresh error".to_string(),
                    )
                })?;
            results = rows2
                .iter()
                .map(|r| {
                    serde_json::json!({
                        "id": r.id,
                        "name": r.name,
                        "entity_type": r.entity_type,
                        "properties": r.properties,
                        "origin": "cache",
                        "source": "db",
                    })
                })
                .collect();
        }
    }

    Ok(Json(serde_json::json!({
        "query": q,
        "online": online,
        "count": results.len(),
        "new_items": new_ingested,
        "errors": errors,
        "results": results,
    })))
}

// ============================================================
// GET /api/v1/topic — quality-focused theme search
// ============================================================

async fn topic_search(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(params): axum::extract::Query<SmartSearchQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let q = params.q.trim().to_string();
    if q.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "q is required".into()));
    }

    let entity_rows = db::search::topic_entities(&state.postgres, &q)
        .await
        .map_err(|e| {
            tracing::error!("topic entities DB error: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "DB error".to_string())
        })?;

    let entity_ids: Vec<String> = entity_rows.iter().map(|r| r.id.clone()).collect();
    if entity_ids.is_empty() {
        return Ok(Json(serde_json::json!({
            "theme": q,
            "entities": [],
            "relations": [],
            "count": 0,
        })));
    }

    let rel_rows = db::relationships::between_entities(&state.postgres, &entity_ids)
        .await
        .map_err(|e| {
            tracing::error!("topic relations DB error: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "DB error".to_string())
        })?;

    // Score relation strength: +3 if typed (not generic), +confidence
    let weak_types = ["related", "related_to", "related topic", "mentions"];
    let mut scored: Vec<serde_json::Value> = rel_rows
        .iter()
        .map(|r| {
            let is_weak = weak_types.contains(&r.relationship_type.to_lowercase().as_str());
            let conf = r.confidence.unwrap_or(0.0);
            let strength = if is_weak { 0.0 } else { 3.0 } + conf;
            serde_json::json!({
                "id": r.id.to_string(),
                "from": r.from_entity,
                "to": r.to_entity,
                "type": r.relationship_type,
                "strength": strength,
                "confidence": conf,
                "properties": r.properties,
            })
        })
        .collect();

    scored.sort_by(|a, b| {
        let sa = a.get("strength").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let sb = b.get("strength").and_then(|v| v.as_f64()).unwrap_or(0.0);
        sb.partial_cmp(&sa).unwrap_or(std::cmp::Ordering::Equal)
    });

    let entities: Vec<serde_json::Value> = entity_rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "id": r.id,
                "name": r.name,
                "entity_type": r.entity_type,
                "properties": r.properties,
            })
        })
        .collect();

    Ok(Json(serde_json::json!({
        "theme": q,
        "entities": entities,
        "relations": scored,
        "count": entities.len(),
        "relation_count": scored.len(),
    })))
}

// ============================================================
// POST /api/v1/search — FTS with structured filters
// ============================================================

#[derive(Deserialize)]
struct SearchRequest {
    query: String,
    #[serde(default)]
    filters: Option<serde_json::Value>,
}

async fn search(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<SearchRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    // Parse optional filters
    let mut entity_type: Option<String> = None;
    let mut confidence: Option<f64> = None;
    let mut date_start: Option<String> = None;
    let mut date_end: Option<String> = None;

    if let Some(filters) = &payload.filters {
        if let Some(obj) = filters.as_object() {
            entity_type = obj.get("type").and_then(|v| v.as_str()).map(String::from);
            confidence = obj.get("confidence").and_then(|v| v.as_f64());
            if let Some(dr) = obj.get("date_range").and_then(|v| v.as_object()) {
                date_start = dr.get("start").and_then(|v| v.as_str()).map(String::from);
                date_end = dr.get("end").and_then(|v| v.as_str()).map(String::from);
            }
        }
        tracing::info!("Filters applied: {:?}", filters);
    }

    let rows = db::search::search_filtered(
        &state.postgres,
        &payload.query,
        entity_type.as_deref(),
        confidence,
        date_start.as_deref(),
        date_end.as_deref(),
    )
    .await
    .map_err(|e| {
        tracing::error!("Search DB error: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, "DB error".to_string())
    })?;

    let results: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "id": r.entity.id,
                "name": r.entity.name,
                "entity_type": r.entity.entity_type,
                "properties": r.entity.properties,
                "rank": r.rank,
            })
        })
        .collect();

    Ok(Json(serde_json::json!({
        "query": payload.query,
        "count": results.len(),
        "results": results,
    })))
}

// ============================================================
// POST /api/v1/graph/neighborhood — 1-hop subgraph around seed entities
// ============================================================

const NEIGHBORHOOD_MAX_SEEDS: usize = 500;
const NEIGHBORHOOD_MAX_ID_LEN: usize = 512;
const NEIGHBORHOOD_DEFAULT_LIMIT: u32 = 2000;
const NEIGHBORHOOD_MAX_LIMIT: u32 = 5000;

#[derive(Deserialize)]
struct NeighborhoodRequest {
    ids: Vec<String>,
    #[serde(default)]
    limit: Option<u32>,
}

/// Validates the request: returns deduplicated seed ids (order kept) and the
/// relationship limit. Pure function so it can be tested without a database.
fn validate_neighborhood(req: NeighborhoodRequest) -> Result<(Vec<String>, i64), String> {
    let mut seen = std::collections::HashSet::new();
    let mut ids = Vec::new();
    for id in req.ids {
        let id = id.trim().to_string();
        if id.is_empty() {
            continue;
        }
        if id.len() > NEIGHBORHOOD_MAX_ID_LEN {
            return Err(format!("id longer than {} bytes", NEIGHBORHOOD_MAX_ID_LEN));
        }
        if seen.insert(id.clone()) {
            ids.push(id);
        }
    }
    if ids.is_empty() {
        return Err("ids must contain at least one non-empty id".into());
    }
    if ids.len() > NEIGHBORHOOD_MAX_SEEDS {
        return Err(format!("at most {} ids allowed", NEIGHBORHOOD_MAX_SEEDS));
    }
    let limit = req
        .limit
        .unwrap_or(NEIGHBORHOOD_DEFAULT_LIMIT)
        .clamp(1, NEIGHBORHOOD_MAX_LIMIT);
    Ok((ids, i64::from(limit)))
}

async fn graph_neighborhood(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<NeighborhoodRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let (seeds, limit) =
        validate_neighborhood(payload).map_err(|e| (StatusCode::BAD_REQUEST, e))?;

    // Fetch one extra row: `truncated` is then exact, even when exactly
    // `limit` relations exist.
    let mut rels = db::relationships::touching(&state.postgres, &seeds, limit + 1)
        .await
        .map_err(|e| {
            tracing::error!("neighborhood relations DB error: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "DB error".to_string())
        })?;
    let truncated = rels.len() as i64 > limit;
    rels.truncate(limit as usize);

    // Seeds first, then every neighbor referenced by a returned relation.
    let mut wanted: Vec<String> = seeds.clone();
    let mut seen: std::collections::HashSet<&str> = seeds.iter().map(String::as_str).collect();
    let mut extra = Vec::new();
    for r in &rels {
        for end in [&r.from_entity, &r.to_entity] {
            if seen.insert(end.as_str()) {
                extra.push(end.clone());
            }
        }
    }
    wanted.extend(extra);

    let entity_rows = db::entities::get_many(&state.postgres, &wanted)
        .await
        .map_err(|e| {
            tracing::error!("neighborhood entities DB error: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "DB error".to_string())
        })?;

    let entities: Vec<serde_json::Value> = entity_rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "id": r.id,
                "name": r.name,
                "entity_type": r.entity_type,
                "properties": r.properties,
            })
        })
        .collect();

    let relationships: Vec<serde_json::Value> = rels
        .iter()
        .map(|r| {
            serde_json::json!({
                "id": r.id.to_string(),
                "from_entity": r.from_entity,
                "to_entity": r.to_entity,
                "relationship_type": r.relationship_type,
                "properties": r.properties,
                "confidence": r.confidence,
                "sources": r.sources,
            })
        })
        .collect();

    Ok(Json(serde_json::json!({
        "seeds": seeds,
        "entities": entities,
        "relationships": relationships,
        "truncated": truncated,
    })))
}

#[cfg(test)]
mod neighborhood_tests {
    use super::*;

    fn req(ids: &[&str], limit: Option<u32>) -> NeighborhoodRequest {
        NeighborhoodRequest {
            ids: ids.iter().map(|s| s.to_string()).collect(),
            limit,
        }
    }

    #[test]
    fn dedups_and_trims_keeping_order() {
        let (ids, _) = validate_neighborhood(req(&["b", " a ", "b", "", "a"], None)).unwrap();
        assert_eq!(ids, vec!["b", "a"]);
    }

    #[test]
    fn rejects_empty() {
        assert!(validate_neighborhood(req(&[], None)).is_err());
        assert!(validate_neighborhood(req(&["", "  "], None)).is_err());
    }

    #[test]
    fn rejects_too_many_seeds() {
        let many: Vec<String> = (0..=NEIGHBORHOOD_MAX_SEEDS)
            .map(|i| i.to_string())
            .collect();
        let r = NeighborhoodRequest {
            ids: many,
            limit: None,
        };
        assert!(validate_neighborhood(r).is_err());
    }

    #[test]
    fn rejects_oversized_id() {
        let long = "x".repeat(NEIGHBORHOOD_MAX_ID_LEN + 1);
        assert!(validate_neighborhood(req(&[long.as_str()], None)).is_err());
    }

    #[test]
    fn clamps_limit() {
        assert_eq!(validate_neighborhood(req(&["a"], None)).unwrap().1, 2000);
        assert_eq!(validate_neighborhood(req(&["a"], Some(0))).unwrap().1, 1);
        assert_eq!(
            validate_neighborhood(req(&["a"], Some(99_999))).unwrap().1,
            5000
        );
    }
}

// ============================================================
// GET /api/v1/timeline — dated articles about a topic, oldest first
// ============================================================

const TIMELINE_MAX_QUERY_LEN: usize = 200;
const TIMELINE_DEFAULT_LIMIT: u32 = 500;
const TIMELINE_MAX_LIMIT: u32 = 2000;
/// GDELT DOC API maximum per call.
const TIMELINE_FETCH_LIMIT: u32 = 250;

#[derive(Deserialize)]
struct TimelineQuery {
    q: String,
    /// Inclusive start day, YYYY-MM-DD (UTC).
    #[serde(default)]
    from: Option<String>,
    /// Inclusive end day, YYYY-MM-DD (UTC).
    #[serde(default)]
    to: Option<String>,
    /// Fetch articles for the period from GDELT before answering.
    #[serde(default)]
    online: Option<bool>,
    #[serde(default)]
    limit: Option<u32>,
}

#[derive(Debug, PartialEq)]
struct TimelineParams {
    q: String,
    from: Option<chrono::NaiveDate>,
    to: Option<chrono::NaiveDate>,
    online: bool,
    limit: i64,
}

fn parse_day(field: &str, value: Option<String>) -> Result<Option<chrono::NaiveDate>, String> {
    match value.as_deref().map(str::trim) {
        None | Some("") => Ok(None),
        Some(s) => chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d")
            .map(Some)
            .map_err(|_| format!("{field} must be a date in YYYY-MM-DD format")),
    }
}

/// Validates the request. Pure function so it can be tested without a database.
fn validate_timeline(req: TimelineQuery) -> Result<TimelineParams, String> {
    let q = req.q.trim().to_string();
    if q.is_empty() {
        return Err("q is required".into());
    }
    if q.chars().count() > TIMELINE_MAX_QUERY_LEN {
        return Err(format!(
            "q must be at most {TIMELINE_MAX_QUERY_LEN} characters"
        ));
    }
    let from = parse_day("from", req.from)?;
    let to = parse_day("to", req.to)?;
    if let (Some(f), Some(t)) = (from, to) {
        if f > t {
            return Err("from must be on or before to".into());
        }
    }
    let limit = req
        .limit
        .unwrap_or(TIMELINE_DEFAULT_LIMIT)
        .clamp(1, TIMELINE_MAX_LIMIT);
    Ok(TimelineParams {
        q,
        from,
        to,
        online: req.online.unwrap_or(false),
        limit: i64::from(limit),
    })
}

fn day_start_utc(d: chrono::NaiveDate) -> chrono::DateTime<chrono::Utc> {
    d.and_hms_opt(0, 0, 0)
        .expect("midnight is a valid time")
        .and_utc()
}

async fn timeline(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(req): axum::extract::Query<TimelineQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let p = validate_timeline(req).map_err(|e| (StatusCode::BAD_REQUEST, e))?;

    let mut ingested = 0usize;
    let mut errors: Vec<String> = Vec::new();
    if p.online {
        let mut args = serde_json::json!({ "q": p.q, "limit": TIMELINE_FETCH_LIMIT });
        if let Some(f) = p.from {
            args["start"] = serde_json::json!(f.format("%Y-%m-%d").to_string());
        }
        if let Some(t) = p.to {
            args["end"] = serde_json::json!(t.format("%Y-%m-%d").to_string());
        }
        match crate::ingestion::ingest_from_source(state.clone(), "gdelt", args).await {
            Ok((n, errs)) => {
                ingested = n;
                errors.extend(errs);
            }
            Err(e) => errors.push(format!("gdelt: {e}")),
        }
    }

    // `to` is inclusive: the window ends at the start of the next day
    let from = p.from.map(day_start_utc);
    let to = p.to.and_then(|d| d.succ_opt()).map(day_start_utc);
    let mut rows = db::timeline::articles(&state.postgres, &p.q, from, to, p.limit)
        .await
        .map_err(|e| {
            tracing::error!("timeline DB error: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "DB error".to_string())
        })?;
    let truncated = rows.len() as i64 > p.limit;
    rows.truncate(p.limit as usize);

    let prop = |r: &db::timeline::TimelineRow, k: &str| {
        r.properties
            .get(k)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string()
    };
    let articles: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "id": r.id,
                "title": r.title,
                "url": prop(r, "url"),
                "domain": prop(r, "domain"),
                "source_country": prop(r, "source_country"),
                "language": prop(r, "language"),
                "published_at": r.published_at.to_rfc3339(),
            })
        })
        .collect();

    Ok(Json(serde_json::json!({
        "query": p.q,
        "from": p.from.map(|d| d.to_string()),
        "to": p.to.map(|d| d.to_string()),
        "count": articles.len(),
        "truncated": truncated,
        "ingested": ingested,
        "errors": errors,
        "articles": articles,
    })))
}

#[cfg(test)]
mod timeline_tests {
    use super::*;

    fn req(q: &str, from: Option<&str>, to: Option<&str>, limit: Option<u32>) -> TimelineQuery {
        TimelineQuery {
            q: q.into(),
            from: from.map(String::from),
            to: to.map(String::from),
            online: None,
            limit,
        }
    }

    #[test]
    fn accepts_valid_range() {
        let p = validate_timeline(req(
            " Johannesburg march ",
            Some("2026-03-01"),
            Some("2026-03-31"),
            None,
        ))
        .unwrap();
        assert_eq!(p.q, "Johannesburg march");
        assert_eq!(p.from.unwrap().to_string(), "2026-03-01");
        assert_eq!(p.limit, 500);
        assert!(!p.online);
    }

    #[test]
    fn dates_are_optional() {
        let p = validate_timeline(req("x", None, Some(""), None)).unwrap();
        assert!(p.from.is_none() && p.to.is_none());
    }

    #[test]
    fn rejects_bad_input() {
        assert!(validate_timeline(req("  ", None, None, None)).is_err());
        assert!(validate_timeline(req(&"a".repeat(201), None, None, None)).is_err());
        assert!(validate_timeline(req("x", Some("2026-02-30"), None, None)).is_err());
        assert!(validate_timeline(req("x", Some("01/03/2026"), None, None)).is_err());
        assert!(validate_timeline(req("x", Some("2026-03-31"), Some("2026-03-01"), None)).is_err());
    }

    #[test]
    fn clamps_limit() {
        assert_eq!(
            validate_timeline(req("x", None, None, Some(0)))
                .unwrap()
                .limit,
            1
        );
        assert_eq!(
            validate_timeline(req("x", None, None, Some(99_999)))
                .unwrap()
                .limit,
            2000
        );
    }

    #[test]
    fn end_day_is_inclusive() {
        let to = chrono::NaiveDate::from_ymd_opt(2026, 3, 31).unwrap();
        assert_eq!(
            day_start_utc(to.succ_opt().unwrap()).to_rfc3339(),
            "2026-04-01T00:00:00+00:00"
        );
    }
}

// ============================================================
// GET /api/v1/plugins/health + POST /api/v1/plugins/reload
// ============================================================

async fn plugins_health() -> Json<serde_json::Value> {
    let loaded = crate::plugin_bridge::list_plugins().unwrap_or_default();
    let failures = crate::plugin_bridge::plugin_failures()
        .unwrap_or_default()
        .into_iter()
        .map(|(name, err)| serde_json::json!({ "name": name, "error": err }))
        .collect::<Vec<_>>();
    Json(serde_json::json!({
        "loaded": loaded,
        "failed": failures,
    }))
}

async fn plugins_reload() -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    match crate::plugin_bridge::reload_plugins() {
        Ok(loaded) => {
            let failures = crate::plugin_bridge::plugin_failures()
                .unwrap_or_default()
                .into_iter()
                .map(|(name, err)| serde_json::json!({ "name": name, "error": err }))
                .collect::<Vec<_>>();
            Ok(Json(serde_json::json!({
                "reloaded": true,
                "loaded": loaded,
                "failed": failures,
            })))
        }
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e)),
    }
}
