use anyhow::Result;
use std::sync::Arc;
use std::time::Duration;
use twilight_gateway::Event;
use twilight_http::Client as HttpClient;
use tracing::{error, info, warn};
use crate::database::Database;
use crate::automod::AutomodEngine;
use crate::automod::ai::OllamaClient;
use crate::core::ratelimit::RateLimiter;

pub async fn handle_event(
    event: Event,
    db: Arc<Database>,
    http: Arc<HttpClient>,
) -> Result<()> {
    match event {
        Event::Ready(ready) => {
            info!("Gateway connected! Logged in as: {}", ready.user.name);
        }
        Event::MessageCreate(msg) => {
            // Ignore bot messages
            if msg.author.bot {
                return Ok(());
            }

            // 1. Antispam Rate Limit (Max 5 messages per 3 seconds)
            let mut rate_limiter = RateLimiter::new(db.redis.clone());
            let is_allowed = rate_limiter
                .check_limit(&msg.author.id.to_string(), "message", 5, Duration::from_secs(3))
                .await?;

            if !is_allowed {
                warn!("User {} is spamming messages! Taking action...", msg.author.id);
                // TODO: Apply timeout/mute via HTTP
                return Ok(()); // Stop processing this message
            }

            // 2. Automod Fast-Path (Regex/Rules)
            if AutomodEngine::check_fast_path(&msg.content) {
                // If it hits Layer 1, immediately delete the message
                info!("Layer 1 Automod triggered. Deleting message {} from user {}", msg.id, msg.author.id);
                
                let _ = http.delete_message(msg.channel_id, msg.id).await;
                // TODO: Log infraction to Database
                
                return Ok(()); // Message is dead, stop processing
            }

            // 3. Layer 2: AI Microservice (Ollama Deepseek)
            // If the message is long enough to warrant checking, and passed fast path
            if msg.content.len() > 10 {
                // Spawn the AI check in the background so it doesn't block the event handler
                let http_clone = http.clone();
                let msg_id = msg.id;
                let channel_id = msg.channel_id;
                let author_id = msg.author.id;
                let content = msg.content.clone();

                info!("Forwarding message {} to Layer 2 AI for evaluation...", msg_id);

                tokio::spawn(async move {
                    match OllamaClient::new() {
                        Ok(ollama) => {
                            match ollama.evaluate_message(&content).await {
                                Ok(decision) => {
                                    if decision.is_malicious {
                                        warn!("Layer 2 AI Automod triggered! Reason: {}. Deleting message {} from user {}", decision.reason, msg_id, author_id);
                                        let _ = http_clone.delete_message(channel_id, msg_id).await;
                                        // TODO: Log to DB and potentially timeout user
                                    } else {
                                        info!("Layer 2 AI Automod passed for message {}", msg_id);
                                    }
                                }
                                Err(e) => error!("Ollama evaluation failed: {}", e),
                            }
                        }
                        Err(e) => error!("Failed to initialize Ollama Client: {}", e),
                    }
                });
            }
        }
        Event::ChannelDelete(channel) => {
            // ANTINUKE LOGIC Example: Detect mass channel deletion
            info!("Channel deleted: {}", channel.id);
            // We would check rate limits here: "Did this user delete > 3 channels in 10s?"
        }
        _ => {}
    }
    Ok(())
}
