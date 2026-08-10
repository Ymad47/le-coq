//! Le-Chasseur Poller - Standalone binary
//! Scans Le-Chasseur output/ directory and ingests JSON files into PostgreSQL

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::time::sleep;
use tokio::fs;
use sqlx::{PgPool, Row};

#[derive(Debug, Deserialize)]
struct LeChasseurOutput {
    source: String,
    collected_at: String,
    #[serde(default)]
    entities: Vec<Entity>,
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

struct PollerConfig {
    le_chasseur_output_dir: PathBuf,
    poll_interval_secs: u64,
    database_url: String,
}

impl PollerConfig {
    fn from_env() -> Result<Self> {
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

struct Poller {
    config: PollerConfig,
    last_processed_time: Option<DateTime<Utc>>,
    processed_files: HashSet<String>,
}

impl Poller {
    fn new(config: PollerConfig) -> Self {
        Self {
            config,
            last_processed_time: None,
            processed_files: HashSet::new(),
        }
    }

    async fn run(&mut self) -> Result<()> {
        println!(
            "🔍 Starting Le-Chasseur poller - scanning {}",
            self.config.le_chasseur_output_dir.display()
        );
        println!("Poll interval: {}s", self.config.poll_interval_secs);

        loop {
            match self.poll_and_ingest().await {
                Ok(ingested) => {
                    if ingested > 0 {
                        println!("✓ Ingested {} entities", ingested);
                    }
                }
                Err(e) => {
                    eprintln!("✗ Poll error: {}", e);
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
        let mut entries = fs::read_dir(&entities_dir).await?;
        let mut new_files = Vec::new();

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }

            let filename = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();

            if self.processed_files.contains(&filename) {
                continue;
            }

            if let Some(timestamp) = self.extract_timestamp(&filename) {
                if self.last_processed_time.is_none() || timestamp > self.last_processed_time.unwrap() {
                    new_files.push((path, filename, timestamp));
                }
            }
        }

        new_files.sort_by(|a, b| a.2.cmp(&b.2));

        for (path, filename, timestamp) in new_files {
            match self.ingest_file(&path).await {
                Ok(count) => {
                    println!("  → Ingested {} entities from {}", count, filename);
                    total_ingested += count;
                    self.processed_files.insert(filename.clone());
                    self.last_processed_time = Some(timestamp);
                }
                Err(e) => {
                    eprintln!("  ✗ Failed to ingest {}: {}", filename, e);
                }
            }
        }

        Ok(total_ingested)
    }

    fn extract_timestamp(&self, filename: &str) -> Option<DateTime<Utc>> {
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
                    .map(|dt| dt.with_timezone(&Utc));
            }
        }
        None
    }

    async fn ingest_file(&self, path: &Path) -> Result<usize> {
        let content = fs::read_to_string(path).await?;
        let output: LeChasseurOutput = serde_json::from_str(&content)?;

        println!("Processing {} from {} with {} entities", 
                 path.display(), output.source, output.entities.len());

        if output.entities.is_empty() {
            return Ok(0);
        }

        let pool = PgPool::connect(&self.config.database_url).await?;
        let mut ingested = 0;

        for entity in &output.entities {
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

            let properties = entity.properties.clone().unwrap_or(Value::Object(serde_json::Map::new()));
            let sources = vec![output.source.clone()];
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
                Ok(_) => ingested += 1,
                Err(e) => eprintln!("  ✗ Failed to insert {}: {}", entity.id, e),
            }
        }

        Ok(ingested)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let config = PollerConfig::from_env()?;
    let mut poller = Poller::new(config);
    poller.run().await
}
