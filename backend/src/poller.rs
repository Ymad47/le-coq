//! Le-Chasseur JSON poller
//! Scans output/ directory for new JSON files and ingests them into PostgreSQL

use crate::db::postgres::create_pool;
use anyhow::{Context, Result};
use chrono::DateTime;
use serde::Deserialize;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::time::sleep;
use tracing::{info, error};

#[derive(Debug, Deserialize)]
struct LeChasseurOutput {
    source: String,
    collected_at: String,
    #[serde(default)]
    entities: Vec<Entity>,
    #[serde(default)]
    events: Vec<Value>,
    #[serde(default)]
    relationships: Vec<Value>,
}

#[derive(Debug, Deserialize)]
struct Entity {
    id: String,
    #[serde(rename = "type")]
    entity_type: String,
    name: String,
    #[serde(default)]
    aliases: Vec<String>,
    #[serde(default)]
    properties: Option<Value>,
}

pub struct PollerConfig {
    pub le_chasseur_output_dir: PathBuf,
    pub poll_interval_secs: u64,
    pub database_url: String,
}

impl PollerConfig {
    pub fn from_env() -> Result<Self> {
        let le_chasseur_output_dir = std::env::var("LE_CHASSEUR_OUTPUT_DIR")
            .unwrap_or_else(|_| "/home/yaya/le-chasseur/output".to_string());
        let poll_interval_secs = std::env::var("POLL_INTERVAL_SECS")
            .unwrap_or_else(|_| "300".to_string())
            .parse::<u64>()
            .context("Invalid POLL_INTERVAL_SECS")?;
        let database_url = std::env::var("DATABASE_URL")
            .context("DATABASE_URL not set")?;

        Ok(Self {
            le_chasseur_output_dir: PathBuf::from(le_chasseur_output_dir),
            poll_interval_secs,
            database_url,
        })
    }
}

pub struct Poller {
    config: PollerConfig,
    last_processed_time: Option<DateTime<chrono::Utc>>,
    processed_files: std::collections::HashSet<String>,
}

impl Poller {
    pub fn new(config: PollerConfig) -> Self {
        Self {
            config,
            last_processed_time: None,
            processed_files: std::collections::HashSet::new(),
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        info!(
            "🔍 Starting Le-Chasseur poller - scanning {}",
            self.config.le_chasseur_output_dir.display()
        );
        info!("Poll interval: {}s", self.config.poll_interval_secs);

        loop {
            match self.poll_and_ingest().await {
                Ok(ingested) => {
                    if ingested > 0 {
                        info!("✓ Ingested {} entities", ingested);
                    }
                }
                Err(e) => {
                    error!("Poll error: {}", e);
                }
            }

            sleep(Duration::from_secs(self.config.poll_interval_secs)).await;
        }
    }

    async fn poll_and_ingest(&mut self) -> Result<usize> {
        let entities_dir = self.config.le_chasseur_output_dir.join("entities");
        
        if !entities_dir.exists() {
            return Ok(0);
        }

        let mut total_ingested = 0;

        // Scan for JSON files
        let mut entries = tokio::fs::read_dir(&entities_dir).await?;
        let mut new_files = Vec::new();

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            
            // Skip non-JSON files
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }

            let filename = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();

            // Skip already processed files
            if self.processed_files.contains(&filename) {
                continue;
            }

            // Parse timestamp from filename: YYYYMMDD_HHMMSS_source.json
            if let Some(timestamp) = self.extract_timestamp(&filename) {
                // Only process files newer than last poll
                if self.last_processed_time.is_none() || timestamp > self.last_processed_time.unwrap() {
                    new_files.push((path, filename, timestamp));
                }
            }
        }

        // Sort by timestamp (oldest first)
        new_files.sort_by(|a, b| a.2.cmp(&b.2));

        // Process new files
        for (path, filename, timestamp) in new_files {
            match self.ingest_file(&path).await {
                Ok(count) => {
                    info!("Ingested {} entities from {}", count, filename);
                    total_ingested += count;
                    self.processed_files.insert(filename.clone());
                    self.last_processed_time = Some(timestamp);
                }
                Err(e) => {
                    error!("Failed to ingest {}: {}", filename, e);
                }
            }
        }

        Ok(total_ingested)
    }

    fn extract_timestamp(&self, filename: &str) -> Option<DateTime<chrono::Utc>> {
        // Filename format: YYYYMMDD_HHMMSS_source.json
        let parts: Vec<&str> = filename.split('_').collect();
        if parts.len() >= 2 {
            let date_str = parts[0];
            let time_str = parts[1];
            
            if date_str.len() == 8 && time_str.len() == 6 {
                let datetime_str = format!(
                    "{}-{}-{}T{}:{}:{}Z",
                    &date_str[0..4],
                    &date_str[4..6],
                    &date_str[6..8],
                    &time_str[0..2],
                    &time_str[2..4],
                    &time_str[4..6]
                );
                
                return DateTime::parse_from_rfc3339(&datetime_str)
                    .ok()
                    .map(|dt| dt.with_timezone(&chrono::Utc));
            }
        }
        None
    }

    async fn ingest_file(&self, path: &Path) -> Result<usize> {
        let content = tokio::fs::read_to_string(path).await?;
        let output: LeChasseurOutput = serde_json::from_str(&content)?;

        info!(
            "Processing {} file from {} with {} entities",
            path.display(),
            output.source,
            output.entities.len()
        );

        if output.entities.is_empty() {
            return Ok(0);
        }

        // Create DB pool
        let pool = create_pool(&self.config.database_url).await?;

        let mut ingested = 0;

        for entity in output.entities {
            let insert_sql = r#"
                INSERT INTO entities (id, name, entity_type, aliases, properties, sources, created_at)
                VALUES ($1, $2, $3, $4, $5, $6, COALESCE($7::timestamp, NOW()))
                ON CONFLICT (id) DO UPDATE SET
                    name = EXCLUDED.name,
                    entity_type = EXCLUDED.entity_type,
                    aliases = EXCLUDED.aliases,
                    properties = EXCLUDED.properties,
                    sources = EXCLUDED.sources,
                    updated_at = NOW()
            "#;

            let properties = entity.properties.unwrap_or(Value::Object(serde_json::Map::new()));
            let sources = vec![output.source.clone()];
            
            // Parse created_at timestamp
            let created_at = DateTime::parse_from_rfc3339(&output.collected_at)
                .ok()
                .map(|dt| dt.naive_utc());

            match sqlx::query(insert_sql)
                .bind(&entity.id)
                .bind(&entity.name)
                .bind(&entity.entity_type)
                .bind(&entity.aliases)
                .bind(sqlx::types::Json(&properties))
                .bind(&sources)
                .bind(created_at)
                .execute(&pool)
                .await
            {
                Ok(_) => {
                    ingested += 1;
                }
                Err(e) => {
                    error!("Failed to insert entity {}: {}", entity.id, e);
                }
            }
        }

        Ok(ingested)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Datelike;

    #[test]
    fn test_extract_timestamp() {
        let poller = Poller::new(PollerConfig {
            le_chasseur_output_dir: PathBuf::from("/tmp"),
            poll_interval_secs: 300,
            database_url: "postgres://test".to_string(),
        });

        let result = poller.extract_timestamp("20260804_145125_wikipedia.json");
        assert!(result.is_some());
        
        let dt = result.unwrap();
        assert_eq!(dt.year(), 2026);
        assert_eq!(dt.month(), 8);
        assert_eq!(dt.day(), 4);
    }
}
