use anyhow::{Context, Result};
use redis::aio::ConnectionManager;
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::env;
use tracing::info;

pub struct Database {
    pub redis: ConnectionManager,
    #[allow(dead_code)]
    pub pg: PgPool,
}

impl Database {
    pub async fn connect() -> Result<Self> {
        // Connect to PostgreSQL (Supabase)
        let database_url = env::var("DATABASE_URL").context("DATABASE_URL must be set")?;
        info!("Connecting to PostgreSQL...");
        let pg = PgPoolOptions::new()
            .max_connections(5)
            .connect(&database_url)
            .await
            .context("Failed to connect to PostgreSQL")?;
        info!("Connected to PostgreSQL!");

        // Connect to Redis
        let redis_url = env::var("REDIS_URL").context("REDIS_URL must be set")?;
        info!("Connecting to Redis...");
        let redis_client = redis::Client::open(redis_url).context("Invalid Redis URL")?;
        let redis = redis_client
            .get_connection_manager()
            .await
            .context("Failed to create Redis connection manager")?;
        info!("Connected to Redis!");

        Ok(Self { redis, pg })
    }
}
