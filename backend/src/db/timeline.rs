//! Timeline queries: dated articles about a topic, in chronological order.

use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::{PgPool, Row};

/// One dated article on the timeline.
#[derive(Debug, Clone)]
pub struct TimelineRow {
    pub id: String,
    pub title: String,
    pub properties: Value,
    pub published_at: DateTime<Utc>,
}

/// Escapes LIKE wildcards so user input is matched literally.
fn escape_like(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if matches!(c, '\\' | '%' | '_') {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// `%word%` patterns (lowercase, escaped) for every word of the query.
pub fn word_patterns(q: &str) -> Vec<String> {
    q.split_whitespace()
        .map(|w| w.trim_matches('"').to_lowercase())
        .filter(|w| !w.is_empty())
        .map(|w| format!("%{}%", escape_like(&w)))
        .collect()
}

/// Articles with a publication date in [from, to), matching the topic either
/// because they were fetched for this exact query, or because their title
/// contains every word of it. Oldest first. Fetches `limit + 1` rows so the
/// caller can tell whether the result was truncated.
pub async fn articles(
    pool: &PgPool,
    q: &str,
    from: Option<DateTime<Utc>>,
    to: Option<DateTime<Utc>>,
    limit: i64,
) -> Result<Vec<TimelineRow>, sqlx::Error> {
    let patterns = word_patterns(q);
    let rows = sqlx::query(
        r#"SELECT id, name, properties, published_at
           FROM entities
           WHERE entity_type = 'Article'
             AND published_at IS NOT NULL
             AND ($3::timestamptz IS NULL OR published_at >= $3)
             AND ($4::timestamptz IS NULL OR published_at < $4)
             AND (
                   LOWER(properties->>'query') = LOWER($1)
                   OR (cardinality($2::text[]) > 0 AND LOWER(name) LIKE ALL($2))
                 )
           ORDER BY published_at ASC, id
           LIMIT $5"#,
    )
    .bind(q)
    .bind(&patterns)
    .bind(from)
    .bind(to)
    .bind(limit + 1)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .iter()
        .map(|r| TimelineRow {
            id: r.get("id"),
            title: r.get("name"),
            properties: r
                .get::<Option<Value>, _>("properties")
                .unwrap_or(Value::Null),
            published_at: r.get("published_at"),
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn patterns_are_lowercase_escaped_and_skip_quotes() {
        assert_eq!(
            word_patterns(r#""South Africa" 50%_off"#),
            vec!["%south%", "%africa%", r"%50\%\_off%"]
        );
    }

    #[test]
    fn empty_query_gives_no_patterns() {
        assert!(word_patterns("   ").is_empty());
        assert!(word_patterns(r#""""#).is_empty());
    }
}
