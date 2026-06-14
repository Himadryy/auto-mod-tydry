use crate::automod::ai::OllamaClient;
use crate::automod::{AutomodAction, AutomodEngine};
use crate::core::ratelimit::RateLimiter;
use crate::database::Database;
use anyhow::Result;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::{error, info, warn};
use twilight_gateway::Event;
use twilight_http::Client as HttpClient;
use twilight_model::guild::audit_log::AuditLogEventType;
use twilight_model::util::Timestamp;

pub async fn handle_event(event: Event, db: Arc<Database>, http: Arc<HttpClient>) -> Result<()> {
    match event {
        Event::Ready(ready) => {
            info!("Gateway connected! Logged in as: {}", ready.user.name);
        }
        Event::MessageCreate(msg) => {
            // Ignore bot messages (Temporarily disabled for load testing)
            // if msg.author.bot {
            //     return Ok(());
            // }

            // 1. Antispam Rate Limit (Max 5 messages per 3 seconds)
            let mut rate_limiter = RateLimiter::new(db.redis.clone());
            let is_allowed = rate_limiter
                .check_limit(
                    &msg.author.id.to_string(),
                    "message",
                    5,
                    Duration::from_secs(3),
                )
                .await?;

            if !is_allowed {
                warn!(
                    "User {} is spamming messages! Taking action...",
                    msg.author.id
                );
                
                // 1a. Immediately delete the spam message
                let _ = http.delete_message(msg.channel_id, msg.id).await;

                // 1b. Purge recent messages from the spammer
                let http_clone = http.clone();
                let channel_id = msg.channel_id;
                let author_id = msg.author.id;
                tokio::spawn(async move {
                    let mut last_message_id = None;
                    let mut total_deleted = 0;

                    // Scan up to 10 pages (1000 messages) to thoroughly scrub the spammer
                    for _ in 0..10 {
                        let mut messages_page = Vec::new();

                        if let Some(id) = last_message_id {
                            if let Ok(configured_req) = http_clone.channel_messages(channel_id).limit(100) {
                                let req_before = configured_req.before(id);
                                if let Ok(resp) = req_before.await {
                                    messages_page = resp.model().await.unwrap_or_default();
                                }
                            }
                        } else {
                            if let Ok(configured_req) = http_clone.channel_messages(channel_id).limit(100) {
                                if let Ok(resp) = configured_req.await {
                                    messages_page = resp.model().await.unwrap_or_default();
                                }
                            }
                        }

                        if messages_page.is_empty() {
                            break;
                        }

                        last_message_id = messages_page.last().map(|m| m.id);

                        let to_delete: Vec<_> = messages_page
                            .into_iter()
                            .filter(|m| m.author.id == author_id)
                            .map(|m| m.id)
                            .collect();

                        if to_delete.len() >= 2 {
                            if let Ok(del_req) = http_clone.delete_messages(channel_id, &to_delete) {
                                let _ = del_req.await;
                                total_deleted += to_delete.len();
                            }
                        } else if to_delete.len() == 1 {
                            let _ = http_clone.delete_message(channel_id, to_delete[0]).await;
                            total_deleted += 1;
                        }
                    }
                    
                    if total_deleted > 0 {
                        info!("Purged a total of {} messages from spammer {}", total_deleted, author_id);
                    }
                });

                // 1c. Apply a 1-minute timeout to the spammer
                if let Some(guild_id) = msg.guild_id {
                    let timeout_until = SystemTime::now() + Duration::from_secs(60);
                    let timestamp_secs =
                        timeout_until.duration_since(UNIX_EPOCH).unwrap().as_secs() as i64;
                    if let Ok(ts) = Timestamp::from_secs(timestamp_secs) {
                        if let Ok(req) = http
                            .update_guild_member(guild_id, msg.author.id)
                            .communication_disabled_until(Some(ts))
                        {
                            let _ = req.await;
                            info!("User {} timed out for 1 minute due to spamming.", msg.author.id);
                        }
                    }
                }

                return Ok(()); // Stop processing this message
            }

            // 2. Automod Fast-Path (Regex/Rules)
            match AutomodEngine::check_fast_path(&msg.content) {
                AutomodAction::Block(reason) => {
                    info!(
                        "Layer 1 Automod triggered ({}). Deleting message {} from user {}",
                        reason, msg.id, msg.author.id
                    );
                    let _ = http.delete_message(msg.channel_id, msg.id).await;
                    return Ok(());
                }
                AutomodAction::BadWord => {
                    info!(
                        "Layer 1 Automod triggered (BadWord). Deleting message {} from user {}",
                        msg.id, msg.author.id
                    );
                    let _ = http.delete_message(msg.channel_id, msg.id).await;

                    let warnings = rate_limiter
                        .track_warning(&msg.author.id.to_string())
                        .await
                        .unwrap_or(0);

                    if warnings >= 5 {
                        if let Some(guild_id) = msg.guild_id {
                            let timeout_until = SystemTime::now() + Duration::from_secs(60);
                            let timestamp_secs =
                                timeout_until.duration_since(UNIX_EPOCH).unwrap().as_secs() as i64;
                            if let Ok(ts) = Timestamp::from_secs(timestamp_secs) {
                                if let Ok(req) = http
                                    .update_guild_member(guild_id, msg.author.id)
                                    .communication_disabled_until(Some(ts))
                                {
                                    let _ = req.await;
                                }
                                info!(
                                    "User {} timed out for 1 minute due to 5 bad word warnings.",
                                    msg.author.id
                                );
                            }
                        }
                    } else if let Ok(warning_req) =
                        http.create_message(msg.channel_id).content(&format!(
                            "Hey <@{}>, please watch your language. Warning {}/5",
                            msg.author.id, warnings
                        ))
                    {
                        if let Ok(warning_resp) = warning_req.await {
                            if let Ok(warning_msg) = warning_resp.model().await {
                                let http_clone = http.clone();
                                tokio::spawn(async move {
                                    tokio::time::sleep(Duration::from_secs(5)).await;
                                    let _ = http_clone
                                        .delete_message(warning_msg.channel_id, warning_msg.id)
                                        .await;
                                });
                            }
                        }
                    }
                    return Ok(());
                }
                AutomodAction::None => {}
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

                info!(
                    "Forwarding message {} to Layer 2 AI for evaluation...",
                    msg_id
                );

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
        Event::GuildAuditLogEntryCreate(entry) => {
            if entry.action_type == AuditLogEventType::MemberKick {
                if let (Some(executor_id), Some(guild_id)) = (entry.user_id, entry.guild_id) {
                    let mut rate_limiter = RateLimiter::new(db.redis.clone());
                    let is_allowed = rate_limiter
                        .check_limit(&executor_id.to_string(), "kick", 2, Duration::from_secs(60))
                        .await
                        .unwrap_or(true);

                    if !is_allowed {
                        warn!("ANTI-NUKE TRIGGERED! User {} kicked 2 users within 60s. Engaging lockdown sequence.", executor_id);

                        // 1. Quarantine Role (Example Role ID: 123456789012345678)
                        // In a real bot, we would fetch this from DB. We'll simulate applying a generic Quarantine role.
                        // For safety, you often strip roles first.
                        // Assuming we want to strip roles, we'd need to fetch member and overwrite roles:
                        // let _ = http.update_guild_member(guild_id, executor_id).roles(&[quarantine_role_id]).await;

                        // 2. Kick the rogue admin
                        let _ = http.remove_guild_member(guild_id, executor_id).await;

                        // 3. Ban the rogue admin
                        let _ = http.create_ban(guild_id, executor_id).await;

                        info!("Lockdown complete for user {}", executor_id);
                    }
                }
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
