use anyhow::{Context, Result};
use std::env;
use std::sync::Arc;
use tracing::info;
use twilight_gateway::{Intents, Shard, ShardId};
use twilight_http::Client as HttpClient;
use twilight_model::id::{marker::ApplicationMarker, Id};

pub mod ratelimit;

pub struct DiscordCore {
    pub http: Arc<HttpClient>,
    #[allow(dead_code)]
    pub application_id: Id<ApplicationMarker>,
    pub shard: Shard,
}

impl DiscordCore {
    pub async fn connect() -> Result<Self> {
        let token = env::var("DISCORD_TOKEN").context("DISCORD_TOKEN must be set")?;

        info!("Initializing Discord HTTP Client...");
        let http = Arc::new(HttpClient::new(token.clone()));

        // Fetch application info (needed for slash commands later)
        let application_info = http.current_user_application().await?.model().await?;
        let application_id = application_info.id;
        info!("Logged in as Application ID: {}", application_id);

        // Calculate intents: We need Guilds, Guild Members, Guild Bans, Guild Messages, Message Content
        let intents = Intents::GUILDS
            | Intents::GUILD_MEMBERS
            | Intents::GUILD_MODERATION
            | Intents::GUILD_MESSAGES
            | Intents::MESSAGE_CONTENT;

        info!("Initializing Gateway Shard...");
        let shard = Shard::new(ShardId::ONE, token, intents);

        Ok(Self {
            http,
            application_id,
            shard,
        })
    }
}
