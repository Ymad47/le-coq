//! Relationship queries — all SQL for the `relationships` table lives here.

use serde_json::Value;
use sqlx::{PgPool, Row};

/// Row shape returned from relationship queries.
#[derive(Debug, Clone)]
pub struct RelationshipRow {
    pub id: uuid::Uuid,
    pub from_entity: String,
    pub to_entity: String,
    pub relationship_type: String,
    pub properties: Value,
    pub confidence: Option<f32>,
    pub sources: Option<Vec<String>>,
}

impl RelationshipRow {
    fn from_row(row: &sqlx::postgres::PgRow) -> Self {
        Self {
            id: row.get("id"),
            from_entity: row.get("from_entity"),
            to_entity: row.get("to_entity"),
            relationship_type: row.get("relationship_type"),
            properties: row.get("properties"),
            confidence: row.try_get("confidence").ok(),
            sources: row.try_get("sources").ok(),
        }
    }
}

/// List relationships with pagination.
pub async fn list(pool: &PgPool, limit: i32, offset: i32) -> Result<Vec<RelationshipRow>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT id, from_entity, to_entity, relationship_type, properties, confidence, sources
         FROM relationships
         ORDER BY created_at DESC
         LIMIT $1 OFFSET $2",
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    Ok(rows.iter().map(RelationshipRow::from_row).collect())
}

/// Insert a new relationship. Idempotent: returns (uuid, was_inserted).
/// If (from, to, type) triple already exists, returns its id and was_inserted=false.
pub async fn insert(
    pool: &PgPool,
    from_entity: &str,
    to_entity: &str,
    relationship_type: &str,
    properties: &Value,
    confidence: Option<f32>,
    sources: &[String],
) -> Result<(uuid::Uuid, bool), sqlx::Error> {
    let result = sqlx::query(
        r#"INSERT INTO relationships (from_entity, to_entity, relationship_type, properties, confidence, sources)
           VALUES ($1, $2, $3, $4, $5, $6)
           ON CONFLICT (from_entity, to_entity, relationship_type) DO NOTHING
           RETURNING id"#,
    )
    .bind(from_entity)
    .bind(to_entity)
    .bind(relationship_type)
    .bind(sqlx::types::Json(properties))
    .bind(confidence)
    .bind(sources)
    .fetch_optional(pool)
    .await?;

    match result {
        Some(row) => Ok((row.get("id"), true)),
        None => {
            // Was a conflict — fetch the existing id
            let row = sqlx::query(
                "SELECT id FROM relationships
                 WHERE from_entity = $1 AND to_entity = $2 AND relationship_type = $3",
            )
            .bind(from_entity)
            .bind(to_entity)
            .bind(relationship_type)
            .fetch_one(pool)
            .await?;
            Ok((row.get("id"), false))
        }
    }
}

/// Relationships where both endpoints are in `entity_ids`. Used by topic_search.
pub async fn between_entities(
    pool: &PgPool,
    entity_ids: &[String],
) -> Result<Vec<RelationshipRow>, sqlx::Error> {
    let rows = sqlx::query(
        r#"SELECT id, from_entity, to_entity, relationship_type, properties, confidence, sources
           FROM relationships
           WHERE from_entity = ANY($1) AND to_entity = ANY($1)"#,
    )
    .bind(entity_ids)
    .fetch_all(pool)
    .await?;

    Ok(rows.iter().map(RelationshipRow::from_row).collect())
}
