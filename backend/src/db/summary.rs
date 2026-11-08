//! Aggregation queries for entity summaries (used by /api/v1/entities/:id/summary).

use sqlx::{PgPool, Row};

/// Aggregated type breakdown: "5 person, 3 organization" for all neighbors.
#[derive(Debug)]
pub struct TypeBreakdown {
    pub entity_type: String,
    pub count: i64,
}

/// Strongest connected neighbor by summed confidence.
#[derive(Debug)]
pub struct TopNeighbor {
    pub id: String,
    pub name: String,
    pub weight: f64,
    pub rels: i64,
}

/// Relationship type frequency for an entity.
#[derive(Debug)]
pub struct RelTypeCount {
    pub relationship_type: String,
    pub count: i64,
}

/// Count neighbor entities grouped by type, both directions.
pub async fn type_breakdown(
    pool: &PgPool,
    entity_id: &str,
) -> Result<Vec<TypeBreakdown>, sqlx::Error> {
    let rows = sqlx::query(
        r#"SELECT neighbor.entity_type, COUNT(*) as n
           FROM relationships r
           JOIN entities neighbor ON (
             neighbor.id = CASE WHEN r.from_entity = $1 THEN r.to_entity ELSE r.from_entity END
           )
           WHERE r.from_entity = $1 OR r.to_entity = $1
           GROUP BY neighbor.entity_type
           ORDER BY n DESC"#,
    )
    .bind(entity_id)
    .fetch_all(pool)
    .await?;

    rows.iter()
        .map(|r| {
            Ok(TypeBreakdown {
                entity_type: r.try_get("entity_type")?,
                count: r.try_get("n")?,
            })
        })
        .collect()
}

/// Top 3 typed relation kinds (excludes generic types like "related"/"mentions").
pub async fn top_rel_types(
    pool: &PgPool,
    entity_id: &str,
) -> Result<Vec<RelTypeCount>, sqlx::Error> {
    let rows = sqlx::query(
        r#"SELECT relationship_type, COUNT(*) as n
           FROM relationships
           WHERE (from_entity = $1 OR to_entity = $1)
             AND relationship_type NOT IN ('related', 'related_to', 'related topic', 'mentions')
           GROUP BY relationship_type
           ORDER BY n DESC
           LIMIT 3"#,
    )
    .bind(entity_id)
    .fetch_all(pool)
    .await?;

    rows.iter()
        .map(|r| {
            Ok(RelTypeCount {
                relationship_type: r.try_get("relationship_type")?,
                count: r.try_get("n")?,
            })
        })
        .collect()
}

/// Strongest neighbor by SUM(confidence) across shared typed relations.
pub async fn top_neighbor(
    pool: &PgPool,
    entity_id: &str,
) -> Result<Option<TopNeighbor>, sqlx::Error> {
    let row = sqlx::query(
        r#"SELECT neighbor.id, neighbor.name, COALESCE(SUM(r.confidence), 0)::float8 as weight, COUNT(*) as rels
           FROM relationships r
           JOIN entities neighbor ON (
             neighbor.id = CASE WHEN r.from_entity = $1 THEN r.to_entity ELSE r.from_entity END
           )
           WHERE (r.from_entity = $1 OR r.to_entity = $1)
             AND r.relationship_type NOT IN ('related', 'related_to', 'mentions')
           GROUP BY neighbor.id, neighbor.name
           ORDER BY weight DESC
           LIMIT 1"#,
    )
    .bind(entity_id)
    .fetch_optional(pool)
    .await?;

    row.map(|r| {
        Ok(TopNeighbor {
            id: r.try_get("id")?,
            name: r.try_get("name")?,
            weight: r.try_get("weight")?,
            rels: r.try_get("rels")?,
        })
    })
    .transpose()
}
