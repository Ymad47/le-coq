//! Neo4j connection pool and graph queries

use anyhow::Result;
use neo4rs::Graph;

pub async fn create_graph(neo4j_url: &str, user: &str, password: &str) -> Result<Graph> {
    let config = neo4rs::ConfigBuilder::default()
        .uri(neo4j_url)
        .user(user)
        .password(password)
        .build()?;
    
    let graph = Graph::connect(config).await?;
    Ok(graph)
}
