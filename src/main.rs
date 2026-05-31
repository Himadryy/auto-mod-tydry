mod automod;
mod core;
mod database;
mod events;

use anyhow::Result;
use std::sync::Arc;
use tracing::{error, info, warn};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables from .env file
    if let Err(e) = dotenvy::dotenv() {
        warn!("Failed to load .env file: {}", e);
    }

    // Initialize the logger
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("automod_bot=info".parse()?))
        .init();

    info!("Initializing Automod Bot engine...");

    // 1. Connect to Database (Redis & Postgres)
    let db = database::Database::connect().await?;
    let db = Arc::new(db);

    // 2. Initialize Discord Core (HTTP & Gateway)
    let mut discord = core::DiscordCore::connect().await?;

    info!("Setup complete. Connecting to Gateway...");

    // 3. Event Loop
    loop {
        let event = match discord.shard.next_event().await {
            Ok(event) => event,
            Err(source) => {
                warn!(?source, "Error receiving event");
                // If the error is fatal, break. Otherwise, continue.
                if source.is_fatal() {
                    break;
                }
                continue;
            }
        };

        // Spawn a new task to handle the event without blocking the main loop
        let _db_clone = Arc::clone(&db);
        let _http_clone = Arc::clone(&discord.http);
        
        tokio::spawn(async move {
            if let Err(e) = events::handle_event(event, _db_clone, _http_clone).await {
                error!("Error handling event: {:?}", e);
            }
        });
    }

    Ok(())
}
