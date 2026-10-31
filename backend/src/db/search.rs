//! Full-text search logic for entities.

use serde_json::Value;
use sqlx::{PgPool, Row};

use super::entities::EntityRow;

/// Search result row: entity + FTS rank.
#[derive(Debug, Clone)]
pub struct SearchRow {
    pub entity: EntityRow,
    pub rank: f32,
}

/// FTS-first search with ILIKE fallback (multi-word AND semantics).
pub async fn search_entities(pool: &PgPool, q: &str) -> Result<Vec<EntityRow>, sqlx::Error> {
    // 1) PostgreSQL FTS — handles stemming, stopwords, ranking
    let sql = "SELECT id, name, entity_type, properties, \
               ts_rank(to_tsvector('english', name), plainto_tsquery('english', $1)) as rank \
               FROM entities \
               WHERE to_tsvector('english', name) @@ plainto_tsquery('english', $1) \
               ORDER BY rank DESC LIMIT 50";
    let rows = sqlx::query(sql).bind(q).fetch_all(pool).await?;
    if !rows.is_empty() {
        return Ok(rows.iter().map(entity_from_row).collect());
    }

    // 2) Fallback: ILIKE every word (AND between words, OR per field)
    let words: Vec<&str> = q.split_whitespace().collect();
    if words.is_empty() {
        return Ok(Vec::new());
    }

    if words.len() == 1 {
        let pat = format!("%{}%", words[0]);
        let rows = sqlx::query(
            "SELECT id, name, entity_type, properties \
             FROM entities WHERE name ILIKE $1 OR properties::text ILIKE $1 \
             ORDER BY created_at DESC LIMIT 50",
        )
        .bind(&pat)
        .fetch_all(pool)
        .await?;
        return Ok(rows.iter().map(entity_from_row).collect());
    }

    // Multi-word: build dynamic AND chain
    let mut sql_like = String::from(
        "SELECT id, name, entity_type, properties FROM entities WHERE ",
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
    let rows = qb.fetch_all(pool).await?;
    Ok(rows.iter().map(entity_from_row).collect())
}

/// FTS search with structured filters (POST /api/v1/search).
pub async fn search_filtered(
    pool: &PgPool,
    query: &str,
    entity_type: Option<&str>,
    confidence: Option<f64>,
    date_start: Option<&str>,
    date_end: Option<&str>,
) -> Result<Vec<SearchRow>, sqlx::Error> {
    let mut sql = "SELECT id, name, entity_type, properties,
                   ts_rank(to_tsvector('english', name), plainto_tsquery('english', $1)) as rank
         FROM entities
         WHERE to_tsvector('english', name) @@ plainto_tsquery('english', $1)".to_string();

    let mut binds: Vec<String> = vec![query.to_string()];
    let mut param_index = 2;

    if let Some(t) = entity_type {
        sql.push_str(&format!(" AND entity_type = ${}", param_index));
        binds.push(t.to_string());
        param_index += 1;
    }

    if let Some(c) = confidence {
        sql.push_str(&format!(" AND (properties->>'confidence')::float >= ${}", param_index));
        binds.push(c.to_string());
        param_index += 1;
    }

    if let Some(start) = date_start {
        sql.push_str(&format!(" AND created_at >= ${}", param_index));
        binds.push(start.to_string());
        param_index += 1;
    }

    if let Some(end) = date_end {
        sql.push_str(&format!(" AND created_at <= ${}", param_index));
        binds.push(end.to_string());
    }

    sql.push_str(" ORDER BY rank DESC LIMIT 50");

    let mut qb = sqlx::query(&sql).bind(&binds[0]);
    for b in binds.iter().skip(1) {
        qb = qb.bind(b);
    }

    let rows = qb.fetch_all(pool).await?;
    Ok(rows
        .iter()
        .map(|r| SearchRow {
            entity: entity_from_row(r),
            rank: r.get::<f32, _>("rank"),
        })
        .collect())
}

/// Topic search: find entities whose name OR properties contain ALL terms (ranked by coverage).
pub async fn topic_entities(pool: &PgPool, q: &str) -> Result<Vec<EntityRow>, sqlx::Error> {
    let terms: Vec<String> = q.to_lowercase().split_whitespace().map(|s| s.to_string()).collect();
    let pattern = format!("%{}%", terms.join("%"));

    let sql = r#"SELECT id, name, entity_type, properties
           FROM entities
           WHERE LOWER(name) LIKE $1 OR LOWER(properties::text) LIKE $1
           ORDER BY
               (LENGTH(name) - LENGTH(REPLACE(LOWER(name), $2, '')))::float
                   / GREATEST(LENGTH($2), 1) DESC,
               name
           LIMIT 100"#;

    let rows = sqlx::query(sql)
        .bind(&pattern)
        .bind(q.to_lowercase())
        .fetch_all(pool)
        .await?;

    Ok(rows.iter().map(entity_from_row).collect())
}

/// Helper: build EntityRow from a PgRow with id/name/entity_type/properties columns.
fn entity_from_row(row: &sqlx::postgres::PgRow) -> EntityRow {
    EntityRow {
        id: row.get::<String, _>("id"),
        name: row.get("name"),
        entity_type: row.get("entity_type"),
        properties: row.get::<Value, _>("properties"),
    }
}
