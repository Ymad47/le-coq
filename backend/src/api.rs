//! API Routes

use axum::{
    routing::{get, post},
    Router,
    extract::State,
    response::Json,
    http::StatusCode,
    middleware,
};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::sync::Arc;

use crate::ingestion::{ingest_data, list_sources};

use crate::AppState;

use crate::ingestion::ingest_from_source;

pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/health", get(health_check))
        .route("/api/v1/entities", get(list_entities).post(create_entity))
        .route("/api/v1/entities/:id", get(get_entity))
        .route("/api/v1/entities/:id/summary", get(entity_summary))
        .route("/api/v1/relationships", get(list_relationships).post(create_relationship))
        .route("/api/v1/search", get(smart_search).post(search))
        .route("/api/v1/topic", get(topic_search))
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
    // Skip auth for health check
    if req.uri().path() == "/health" {
        return Ok(next.run(req).await);
    }
    
    // Check for API key in header
    let api_key = req
        .headers()
        .get(&state.config.api_key_header)
        .and_then(|v| v.to_str().ok());
    
    match api_key {
        Some(key) if !key.is_empty() => Ok(next.run(req).await),
        _ if std::env::var("DISABLE_AUTH").unwrap_or_default() == "true" => Ok(next.run(req).await),
        _ => Err(StatusCode::UNAUTHORIZED),
    }
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
    let mut sql = String::from("SELECT id, name, entity_type, properties FROM entities WHERE 1=1");
    
    if query.r#type.is_some() && query.q.is_some() {
        sql.push_str(" AND entity_type = $1 AND (name ILIKE $2 OR properties::text ILIKE $2) ORDER BY created_at DESC LIMIT $3");
        let limit = query.limit.unwrap_or(100).min(1000) as i32;
        
        let rows = sqlx::query(&sql)
            .bind(&query.r#type.as_ref().unwrap())
            .bind(format!("%{}%", &query.q.as_ref().unwrap()))
            .bind(limit)
            .fetch_all(&state.postgres)
            .await
            .map_err(|e| {
                tracing::error!("DB error: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
            
        let entities: Vec<EntityResponse> = rows.iter().map(|row| {
            EntityResponse {
                id: row.get::<String, _>("id"),
                name: row.get("name"),
                r#type: row.get("entity_type"),
                properties: row.get("properties"),
            }
        }).collect();
        
        Ok(Json(entities))
    } else if query.r#type.is_some() {
        sql.push_str(" AND entity_type = $1 ORDER BY created_at DESC LIMIT $2");
        let limit = query.limit.unwrap_or(100).min(1000) as i32;
        
        let rows = sqlx::query(&sql)
            .bind(&query.r#type.as_ref().unwrap())
            .bind(limit)
            .fetch_all(&state.postgres)
            .await
            .map_err(|e| {
                tracing::error!("DB error: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
            
        let entities: Vec<EntityResponse> = rows.iter().map(|row| {
            EntityResponse {
                id: row.get::<String, _>("id"),
                name: row.get("name"),
                r#type: row.get("entity_type"),
                properties: row.get("properties"),
            }
        }).collect();
        
        Ok(Json(entities))
    } else if query.q.is_some() {
        sql.push_str(" AND (name ILIKE $1 OR properties::text ILIKE $1) ORDER BY created_at DESC LIMIT $2");
        let limit = query.limit.unwrap_or(100).min(1000) as i32;

        let rows = sqlx::query(&sql)
            .bind(format!("%{}%", &query.q.as_ref().unwrap()))
            .bind(limit)
            .fetch_all(&state.postgres)
            .await
            .map_err(|e| {
                tracing::error!("DB error: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

        let entities: Vec<EntityResponse> = rows.iter().map(|row| {
            EntityResponse {
                id: row.get::<String, _>("id"),
                name: row.get("name"),
                r#type: row.get("entity_type"),
                properties: row.get("properties"),
            }
        }).collect();

        Ok(Json(entities))
    } else {
        sql.push_str(" ORDER BY created_at DESC LIMIT $1");
        let limit = query.limit.unwrap_or(100).min(1000) as i32;
        
        let rows = sqlx::query(&sql)
            .bind(limit)
            .fetch_all(&state.postgres)
            .await
            .map_err(|e| {
                tracing::error!("DB error: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
            
        let entities: Vec<EntityResponse> = rows.iter().map(|row| {
            EntityResponse {
                id: row.get::<String, _>("id"),
                name: row.get("name"),
                r#type: row.get("entity_type"),
                properties: row.get("properties"),
            }
        }).collect();
        
        Ok(Json(entities))
    }
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

    // entities.id est TEXT — accepte un id custom ou génère un UUID
    let id = payload.get("id").and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    sqlx::query(
        r#"
        INSERT INTO entities (id, name, entity_type, aliases, properties, sources)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#
    )
    .bind(&id)
    .bind(name)
    .bind(entity_type)
    .bind(&aliases_vec)
    .bind(&properties)
    .bind(&sources_vec)
    .execute(&state.postgres)
    .await
    .map_err(|e| {
        tracing::error!("Database error: {}", e);
        if e.to_string().contains("duplicate key") {
            StatusCode::CONFLICT
        } else {
            StatusCode::INTERNAL_SERVER_ERROR
        }
    })?;

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
    // entities.id est TEXT (ex: wiki_Paris), pas UUID
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
            id: r.get::<String, _>("id"),
            name: r.get("name"),
            r#type: r.get("entity_type"),
            properties: r.get("properties"),
        })
    }).ok_or(StatusCode::NOT_FOUND)
}

/// GET /api/v1/entities/:id/summary
/// Narrative summary auto-generated from entity + its relations.
/// No LLM — SQL aggregations + templating. Local & instant.
async fn entity_summary(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    // 1) Fetch entity
    let row = sqlx::query(
        "SELECT id, name, entity_type, properties FROM entities WHERE id = $1"
    )
    .bind(&id)
    .fetch_optional(&state.postgres)
    .await
    .map_err(|e| {
        tracing::error!("entity fetch error: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, "DB error".into())
    })?;

    let Some(row) = row else {
        return Err((StatusCode::NOT_FOUND, "entity not found".into()));
    };

    let name: String = row.get("name");
    let entity_type: String = row.get("entity_type");
    let props: serde_json::Value = row.get("properties");
    let description = props
        .get("description")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    // 2) Count relations by endpoint type (outgoing + incoming)
    //    Classify neighbor entity_type, grouped.
    let type_breakdown = sqlx::query(
        r#"SELECT neighbor.entity_type, COUNT(*) as n
           FROM relationships r
           JOIN entities neighbor ON (
             neighbor.id = CASE WHEN r.from_entity = $1 THEN r.to_entity ELSE r.from_entity END
           )
           WHERE r.from_entity = $1 OR r.to_entity = $1
           GROUP BY neighbor.entity_type
           ORDER BY n DESC"#
    )
    .bind(&id)
    .fetch_all(&state.postgres)
    .await
    .map_err(|e| {
        tracing::error!("type breakdown error: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, "DB error".into())
    })?;

    // 3) Top 3 relation types (excluding generic)
    let rel_types = sqlx::query(
        r#"SELECT relationship_type, COUNT(*) as n
           FROM relationships
           WHERE (from_entity = $1 OR to_entity = $1)
             AND relationship_type NOT IN ('related', 'related_to', 'related topic', 'mentions')
           GROUP BY relationship_type
           ORDER BY n DESC
           LIMIT 3"#
    )
    .bind(&id)
    .fetch_all(&state.postgres)
    .await
    .map_err(|e| {
        tracing::error!("rel types error: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, "DB error".into())
    })?;

    // 4) Strongest neighbor (highest total confidence across shared relations)
    let top_neighbor = sqlx::query(
        r#"SELECT neighbor.id, neighbor.name, SUM(r.confidence) as weight, COUNT(*) as rels
           FROM relationships r
           JOIN entities neighbor ON (
             neighbor.id = CASE WHEN r.from_entity = $1 THEN r.to_entity ELSE r.from_entity END
           )
           WHERE (r.from_entity = $1 OR r.to_entity = $1)
             AND r.relationship_type NOT IN ('related', 'related_to', 'mentions')
           GROUP BY neighbor.id, neighbor.name
           ORDER BY weight DESC
           LIMIT 1"#
    )
    .bind(&id)
    .fetch_optional(&state.postgres)
    .await
    .map_err(|e| {
        tracing::error!("top neighbor error: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, "DB error".into())
    })?;

    // 5) Build narrative text
    let mut sentences: Vec<String> = Vec::new();

    // Intro: first sentence of description, or type + name
    let first_sentence = description
        .split('.')
        .next()
        .map(|s| s.trim().to_string())
        .filter(|s| s.len() > 20);
    if let Some(s) = first_sentence {
        sentences.push(format!("{}.", s));
    } else {
        sentences.push(format!("{} ({}).", name, entity_type));
    }

    // Link stats
    let total_links: i64 = type_breakdown
        .iter()
        .map(|r| r.get::<i64, _>("n"))
        .sum();
    if total_links > 0 {
        let breakdown_str: Vec<String> = type_breakdown
            .iter()
            .take(4)
            .map(|r| format!("{} {}", r.get::<i64, _>("n"), r.get::<String, _>("entity_type").to_lowercase()))
            .collect();
        sentences.push(format!(
            "Lié à {} entités ({}).",
            total_links,
            breakdown_str.join(", ")
        ));
    } else {
        sentences.push("Aucune relation connue pour le moment.".to_string());
    }

    // Dominant relation types
    if !rel_types.is_empty() {
        let types: Vec<String> = rel_types
            .iter()
            .map(|r| format!("{} ({})", r.get::<String, _>("relationship_type").replace('_', " "), r.get::<i64, _>("n")))
            .collect();
        sentences.push(format!("Types de liens: {}.", types.join(", ")));
    }

    // Strongest neighbor
    if let Some(r) = top_neighbor {
        let neighbor_name: String = r.get("name");
        let weight: f64 = r.get("weight");
        sentences.push(format!(
            "Lien le plus fort: {} (confiance {:.0}%).",
            neighbor_name,
            weight * 50.0 // rough normalization: avg confidence ~0.5-1 → ~50%+
        ));
    }

    Ok(Json(serde_json::json!({
        "id": id,
        "name": name,
        "summary": sentences.join(" "),
    })))
}

#[derive(Deserialize)]
struct RelQuery {
    #[serde(default)]
    limit: Option<u32>,
}

async fn list_relationships(
    State(state): State<Arc<AppState>>,
    query: axum::extract::Query<RelQuery>,
) -> Result<Json<Vec<serde_json::Value>>, StatusCode> {
    let limit = query.limit.unwrap_or(100).min(5000) as i32;
    let rows = sqlx::query("SELECT id, from_entity, to_entity, relationship_type, properties, confidence FROM relationships LIMIT $1")
        .bind(limit)
        .fetch_all(&state.postgres)
        .await
        .map_err(|e| {
            tracing::error!("DB error: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    
    let rels: Vec<serde_json::Value> = rows.iter().map(|row| {
        serde_json::json!({
            "id": row.get::<uuid::Uuid, _>("id").to_string(),
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
    let from_entity_str = payload.get("from_entity").and_then(|v| v.as_str()).ok_or(StatusCode::BAD_REQUEST)?;
    let to_entity_str = payload.get("to_entity").and_then(|v| v.as_str()).ok_or(StatusCode::BAD_REQUEST)?;
    let rel_type = payload.get("relationship_type").and_then(|v| v.as_str()).ok_or(StatusCode::BAD_REQUEST)?;
    let confidence = payload.get("confidence").and_then(|v| v.as_f64()).unwrap_or(1.0);
    let properties = payload.get("properties").cloned().unwrap_or(serde_json::Value::Object(serde_json::Map::new()));

    // from_entity / to_entity sont TEXT (référencent entities.id texte)
    let row = sqlx::query(
        "INSERT INTO relationships (from_entity, to_entity, relationship_type, properties, confidence)
         VALUES ($1, $2, $3, $4, $5) RETURNING id"
    )
    .bind(from_entity_str)
    .bind(to_entity_str)
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
        "id": row.get::<uuid::Uuid, _>("id").to_string(),
        "from_entity": from_entity_str,
        "to_entity": to_entity_str,
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

/// FTS + ILIKE fallback (multi-words OR per word, AND between words)
async fn run_entity_query(
    pool: &sqlx::PgPool,
    q: &str,
) -> Result<Vec<sqlx::postgres::PgRow>, sqlx::Error> {
    let sql = "SELECT id, name, entity_type, properties, \
               ts_rank(to_tsvector('english', name), plainto_tsquery('english', $1)) as rank \
               FROM entities \
               WHERE to_tsvector('english', name) @@ plainto_tsquery('english', $1) \
               ORDER BY rank DESC LIMIT 50";
    let rows = sqlx::query(sql).bind(q).fetch_all(pool).await?;
    if !rows.is_empty() {
        return Ok(rows);
    }

    // Fallback: ILIKE every word across name + properties (AND between words, OR per field)
    let words: Vec<&str> = q.split_whitespace().collect();
    if words.is_empty() {
        return Ok(rows);
    }
    if words.len() == 1 {
        let pat = format!("%{}%", words[0]);
        return sqlx::query(
            "SELECT id, name, entity_type, properties, 0.5::real as rank \
             FROM entities WHERE name ILIKE $1 OR properties::text ILIKE $1 \
             ORDER BY created_at DESC LIMIT 50",
        )
        .bind(&pat)
        .fetch_all(pool)
        .await;
    }

    let mut sql_like = String::from(
        "SELECT id, name, entity_type, properties, 0.5::real as rank \
         FROM entities WHERE ",
    );
    for i in 0..words.len() {
        if i > 0 {
            sql_like.push_str(" AND ");
        }
        sql_like.push_str(&format!(
            "(name ILIKE ${} OR properties::text ILIKE ${})",
            i + 1,
            i + 1
        ));
    }
    sql_like.push_str(" ORDER BY created_at DESC LIMIT 50");

    let mut qb = sqlx::query(&sql_like);
    for w in &words {
        qb = qb.bind(format!("%{}%", w));
    }
    qb.fetch_all(pool).await
}

#[derive(Deserialize)]
struct SmartSearchQuery {
    q: String,
    #[serde(default)]
    online: Option<bool>,
    #[serde(default)]
    sources: Option<String>,
}

/// GET /api/v1/search?q=...&online=true&sources=wikipedia,wikidata
/// DB-first; if online=true, also hits selected sources and persists results.
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

    // 1) Local DB search (FTS + ILIKE fallback if 0)
    let rows = run_entity_query(&state.postgres, &q).await.map_err(|e| {
        tracing::error!("Smart search DB error: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, "DB error".to_string())
    })?;

    let mut results: Vec<serde_json::Value> = rows.iter().map(|row| {
        serde_json::json!({
            "id": row.get::<String, _>("id"),
            "name": row.get::<String, _>("name"),
            "entity_type": row.get::<String, _>("entity_type"),
            "properties": row.get::<serde_json::Value, _>("properties"),
            "origin": "cache",
            "source": "db",
        })
    }).collect();

    let mut new_ingested = 0usize;
    let mut errors = Vec::new();

    // 2) Online fetch + persist
    if online {
        let mut handles = Vec::new();
        for src in source_list {
            let state_cl = state.clone();
            let q_cl = q.clone();
            handles.push(tokio::spawn(async move {
                ingest_from_source(state_cl, &src, serde_json::json!({"query": q_cl})).await
            }));
        }

        for handle in handles {
            match handle.await {
                Ok(Ok((count, rel_errors))) => {
                    new_ingested += count;
                    // surface relation drops (cap to avoid bloating the response)
                    errors.extend(rel_errors.into_iter().take(20).map(|e| format!("{} dropped", e)));
                }
                Ok(Err(e)) => errors.push(e),
                Err(e) => errors.push(format!("task error: {}", e)),
            }
        }

        // Refresh from DB after ingestion (dedup handled by ON CONFLICT)
        if new_ingested > 0 {
            let rows2 = run_entity_query(&state.postgres, &q).await.map_err(|e| {
                tracing::error!("Smart search refresh DB error: {}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, "DB refresh error".to_string())
            })?;
            results = rows2.iter().map(|row| {
                serde_json::json!({
                    "id": row.get::<String, _>("id"),
                    "name": row.get::<String, _>("name"),
                    "entity_type": row.get::<String, _>("entity_type"),
                    "properties": row.get::<serde_json::Value, _>("properties"),
                    "origin": "cache",
                    "source": "db",
                })
            }).collect();
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

/// GET /api/v1/topic?q=...
/// Quality-focused search: finds entities matching a theme, then returns only
/// "strong" relations between those entities (typed relations, not Wikipedia's
/// generic `related`). Ranks by a strength score.
async fn topic_search(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(params): axum::extract::Query<SmartSearchQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let q = params.q.trim().to_string();
    if q.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "q is required".into()));
    }

    // Split q into terms for coverage scoring
    let terms: Vec<String> = q.to_lowercase().split_whitespace().map(|s| s.to_string()).collect();
    let pattern = format!("%{}%", terms.join("%"));

    // 1) Relevant entities: name or properties contain ALL terms (ranked by coverage)
    let rows = sqlx::query(
        r#"SELECT id, name, entity_type, properties,
                  (LENGTH(name) - LENGTH(REPLACE(LOWER(name), $2, '')))::float / GREATEST(LENGTH($2),1) as coverage
           FROM entities
           WHERE LOWER(name) LIKE $1 OR LOWER(properties::text) LIKE $1
           ORDER BY coverage DESC, name
           LIMIT 100"#
    )
    .bind(&pattern)
    .bind(&q.to_lowercase())
    .fetch_all(&state.postgres)
    .await
    .map_err(|e| {
        tracing::error!("topic entities DB error: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, "DB error".to_string())
    })?;

    let entity_ids: Vec<String> = rows.iter().map(|r| r.get::<String, _>("id")).collect();
    if entity_ids.is_empty() {
        return Ok(Json(serde_json::json!({
            "theme": q,
            "entities": [],
            "relations": [],
            "count": 0,
        })));
    }

    // 2) Relations between relevant entities ONLY (both endpoints in the set)
    let rel_rows = sqlx::query(
        r#"SELECT id, from_entity, to_entity, relationship_type, properties, confidence, sources
           FROM relationships
           WHERE from_entity = ANY($1) AND to_entity = ANY($1)"#
    )
    .bind(&entity_ids)
    .fetch_all(&state.postgres)
    .await
    .map_err(|e| {
        tracing::error!("topic relations DB error: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, "DB error".to_string())
    })?;

    // 3) Score relation strength
    //    +3 if NOT a generic weak type (related/mentions/etc)
    //    +confidence (0..1)
    //    +1 if either endpoint is a stub (penalize stubs)
    let weak_types = ["related", "related_to", "related topic", "mentions"];
    let mut scored: Vec<serde_json::Value> = rel_rows
        .iter()
        .map(|r| {
            let rel_type: String = r.get("relationship_type");
            let conf: f32 = r.get("confidence");
            let is_weak = weak_types.contains(&rel_type.to_lowercase().as_str());
            let strength = if is_weak { 0.0 } else { 3.0 } + conf;
            serde_json::json!({
                "id": r.get::<uuid::Uuid, _>("id").to_string(),
                "from": r.get::<String, _>("from_entity"),
                "to": r.get::<String, _>("to_entity"),
                "type": rel_type,
                "strength": strength,
                "confidence": conf,
                "properties": r.get::<serde_json::Value, _>("properties"),
            })
        })
        .collect();

    // Sort by strength desc
    scored.sort_by(|a, b| {
        let sa = a.get("strength").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let sb = b.get("strength").and_then(|v| v.as_f64()).unwrap_or(0.0);
        sb.partial_cmp(&sa).unwrap_or(std::cmp::Ordering::Equal)
    });

    let entities: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| serde_json::json!({
            "id": r.get::<String, _>("id"),
            "name": r.get::<String, _>("name"),
            "entity_type": r.get::<String, _>("entity_type"),
            "properties": r.get::<serde_json::Value, _>("properties"),
        }))
        .collect();

    Ok(Json(serde_json::json!({
        "theme": q,
        "entities": entities,
        "relations": scored,
        "count": entities.len(),
        "relation_count": scored.len(),
    })))
}

async fn search(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<SearchRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let q = &payload.query;

    // Build the SQL query with optional filters
    let mut sql = "SELECT id, name, entity_type, properties, 
                   ts_rank(to_tsvector('english', name), plainto_tsquery('english', $1)) as rank
         FROM entities 
         WHERE to_tsvector('english', name) @@ plainto_tsquery('english', $1)".to_string();

    let mut binds = vec![q.clone()];
    let mut param_index = 2; // $1 is used for the search query

    // Add filters if provided
    if let Some(filters) = &payload.filters {
        // Try to parse filters as JSON object
        if let Some(obj) = filters.as_object() {
            // Add entity_type filter
            if let Some(entity_type) = obj.get("type").and_then(|v| v.as_str()) {
                sql.push_str(&format!(" AND entity_type = ${}", param_index));
                binds.push(entity_type.to_string());
                param_index += 1;
            }

            // Add confidence filter
            if let Some(confidence) = obj.get("confidence").and_then(|v| v.as_f64()) {
                sql.push_str(&format!(" AND (properties->>'confidence')::float >= ${}", param_index));
                binds.push(confidence.to_string());
                param_index += 1;
            }

            // Add date range filter
            if let Some(date_range) = obj.get("date_range").and_then(|v| v.as_object()) {
                if let Some(start) = date_range.get("start").and_then(|v| v.as_str()) {
                    sql.push_str(&format!(" AND created_at >= ${}", param_index));
                    binds.push(start.to_string());
                    param_index += 1;
                }
                if let Some(end) = date_range.get("end").and_then(|v| v.as_str()) {
                    sql.push_str(&format!(" AND created_at <= ${}", param_index));
                    binds.push(end.to_string());
                    // Use param_index but don't increment since it's the last one in this block
                    let _ = param_index;
                }
            }
        }

        tracing::info!("Filters applied: {:?}", filters);
    }

    sql.push_str(" ORDER BY rank DESC LIMIT 50");

    // Bind parameters dynamically
    let query = sqlx::query(&sql).bind(binds[0].clone());
    let query = if binds.len() > 1 { query.bind(binds[1].clone()) } else { query };
    let query = if binds.len() > 2 { query.bind(binds[2].clone()) } else { query };
    let query = if binds.len() > 3 { query.bind(binds[3].clone()) } else { query };
    let query = if binds.len() > 4 { query.bind(binds[4].clone()) } else { query };

    let rows = query
        .fetch_all(&state.postgres)
        .await
        .map_err(|e| {
            tracing::error!("Search DB error: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "DB error".to_string())
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

/// GET /api/v1/plugins/health — loaded plugins + import failures
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

/// POST /api/v1/plugins/reload — hot-reload plugin code from disk
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
