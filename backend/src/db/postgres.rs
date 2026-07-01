//! PostgreSQL connection pool and queries

use anyhow::Result;
use sqlx::postgres::{PgPool, PgPoolOptions};

pub async fn create_pool(database_url: &str) -> Result<PgPool> {
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(std::time::Duration::from_secs(30))
        .connect(database_url)
        .await?;
    
    // Run migrations
    sqlx::migrate!("./migrations").run(&pool).await?;
    
    Ok(pool)
}

// TODO: Add entity CRUD functions
// TODO: Add relationship queries
// TODO: Add geospatial queries with PostGIS
