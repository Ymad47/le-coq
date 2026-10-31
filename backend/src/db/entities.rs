//! Entity queries — all SQL for the `entities` table lives here.

use serde_json::Value;
use sqlx::{PgPool, Row};

/// Row shape returned from entity queries.
#[derive(Debug, Clone)]
pub struct EntityRow {
    pub id: String,
    pub name: String,
    pub entity_type: String,
    pub properties: Value,
}

impl EntityRow {
    fn from_row(row: &sqlx::postgres::PgRow) -> Self {
        Self {
            id: row.get::<String, _>("id"),
            name: row.get("name"),
            entity_type: row.get("entity_type"),
            properties: row.get("properties"),
        }
    }
}

/// Filters for listing entities.
#[derive(Debug, Default, Clone)]
pub struct EntityFilters {
    pub entity_type: Option<String>,
    pub search: Option<String>,
    pub limit: i32,
}

/// List entities with optional filters.
pub async fn list(pool: &PgPool, filters: EntityFilters) -> Result<Vec<EntityRow>, sqlx::Error> {
    let mut sql = String::from(
        "SELECT id, name, entity_type, properties FROM entities WHERE 1=1",
    );

    let rows = match (filters.entity_type.as_deref(), filters.search.as_deref()) {
        (Some(t), Some(q)) => {
            sql.push_str(" AND entity_type = $1 AND (name ILIKE $2 OR properties::text ILIKE $2) ORDER BY created_at DESC LIMIT $3");
            let pat = format!("%{}%", q);
            sqlx::query(&sql).bind(t).bind(pat).bind(filters.limit)
                .fetch_all(pool).await?
        }
        (Some(t), None) => {
            sql.push_str(" AND entity_type = $1 ORDER BY created_at DESC LIMIT $2");
            sqlx::query(&sql).bind(t).bind(filters.limit)
                .fetch_all(pool).await?
        }
        (None, Some(q)) => {
            sql.push_str(" AND (name ILIKE $1 OR properties::text ILIKE $1) ORDER BY created_at DESC LIMIT $2");
            let pat = format!("%{}%", q);
            sqlx::query(&sql).bind(pat).bind(filters.limit)
                .fetch_all(pool).await?
        }
        (None, None) => {
            sql.push_str(" ORDER BY created_at DESC LIMIT $1");
            sqlx::query(&sql).bind(filters.limit)
                .fetch_all(pool).await?
        }
    };

    Ok(rows.iter().map(EntityRow::from_row).collect())
}

/// Fetch single entity by id.
pub async fn get_by_id(pool: &PgPool, id: &str) -> Result<Option<EntityRow>, sqlx::Error> {
    let row = sqlx::query(
        "SELECT id, name, entity_type, properties FROM entities WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    Ok(row.as_ref().map(EntityRow::from_row))
}

/// Fetch several entities by id in one query. Unknown ids are skipped.
pub async fn get_many(pool: &PgPool, ids: &[String]) -> Result<Vec<EntityRow>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT id, name, entity_type, properties FROM entities WHERE id = ANY($1)",
    )
    .bind(ids)
    .fetch_all(pool)
    .await?;

    Ok(rows.iter().map(EntityRow::from_row).collect())
}

/// Insert a new entity. Returns error on duplicate id.
#[allow(clippy::too_many_arguments)]
pub async fn insert(
    pool: &PgPool,
    id: &str,
    name: &str,
    entity_type: &str,
    aliases: &[String],
    properties: &Value,
    sources: &[String],
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"INSERT INTO entities (id, name, entity_type, aliases, properties, sources)
           VALUES ($1, $2, $3, $4, $5, $6)"#,
    )
    .bind(id)
    .bind(name)
    .bind(entity_type)
    .bind(aliases)
    .bind(properties)
    .bind(sources)
    .execute(pool)
    .await?;
    Ok(())
}

/// Upsert entity — used by ingestion pipelines. Returns true if a NEW row was inserted.
#[allow(clippy::too_many_arguments)]
pub async fn upsert(
    pool: &PgPool,
    id: &str,
    name: &str,
    entity_type: &str,
    aliases: &[String],
    properties: &Value,
    sources: &[String],
    created_at: Option<chrono::NaiveDateTime>,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        r#"INSERT INTO entities (id, name, entity_type, aliases, properties, sources, created_at)
           VALUES ($1, $2, $3, $4, $5, $6, COALESCE($7::timestamp, NOW()))
           ON CONFLICT (id) DO UPDATE SET
               name = EXCLUDED.name,
               entity_type = EXCLUDED.entity_type,
               aliases = EXCLUDED.aliases,
               properties = EXCLUDED.properties,
               sources = EXCLUDED.sources,
               updated_at = NOW()
           RETURNING (xmax = 0) AS inserted"#,
    )
    .bind(id)
    .bind(name)
    .bind(entity_type)
    .bind(aliases)
    .bind(sqlx::types::Json(properties))
    .bind(sources)
    .bind(created_at)
    .fetch_one(pool)
    .await?;

    Ok(result.get::<bool, _>("inserted"))
}

/// Insert a stub entity (placeholder for FK constraint satisfaction).
/// No-op if id exists.
pub async fn insert_stub(pool: &PgPool, id: &str, name: &str, sources: &[String]) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"INSERT INTO entities (id, name, entity_type, properties, sources, created_at)
           VALUES ($1, $2, 'stub', $3, $4, NOW())
           ON CONFLICT (id) DO NOTHING"#,
    )
    .bind(id)
    .bind(name)
    .bind(sqlx::types::Json(serde_json::json!({"stub": true})))
    .bind(sources)
    .execute(pool)
    .await?;
    Ok(())
}
