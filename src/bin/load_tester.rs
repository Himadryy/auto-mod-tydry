use anyhow::{Context, Result};
use dotenvy::dotenv;
use std::env;
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use tracing::{error, info, warn};
use twilight_http::Client;
use twilight_model::id::Id;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Load environment variables
    dotenv().ok();

    let token = env::var("ATTACKER_BOT_TOKEN")
        .context("ATTACKER_BOT_TOKEN not found in environment")?;
    let channel_id_str = env::var("TARGET_CHANNEL_ID")
        .context("TARGET_CHANNEL_ID not found in environment")?;
    
    let channel_id = Id::new(channel_id_str.parse::<u64>().context("Invalid TARGET_CHANNEL_ID")?);

    info!("Starting Load Tester (Attacker Bot)...");
    info!("Targeting Channel: {}", channel_id);

    let client = Arc::new(Client::new(token));

    // Array of suspicious/bad words to trigger the automod
    let messages = [
        "THIS IS A SPAM ATTACK!",
        "CURSE WORD DETECTED: [censored]",
        "Nuke attempt in progress...",
        "Rapid fire message 1",
        "Rapid fire message 2",
        "I am going to spam this channel until I get banned!",
    ];

    // Concurrency control: Spawn 10 parallel tasks to blast messages
    let concurrency = 10;
    let mut handles = vec![];

    for i in 0..concurrency {
        let client = Arc::clone(&client);
        let messages = messages.clone();
        
        let handle = tokio::spawn(async move {
            let mut local_count = 0;
            loop {
                let content = messages[local_count % messages.len()];
                let create_msg_res = client.create_message(channel_id)
                    .content(content);

                let res = match create_msg_res {
                    Ok(req) => req.await,
                    Err(e) => {
                        error!("Task {}: Validation error: {}", i, e);
                        sleep(Duration::from_secs(1)).await;
                        continue;
                    }
                };

                match res {
                    Ok(_) => {
                        local_count += 1;
                        if local_count % 10 == 0 {
                            info!("Task {}: Sent {} messages", i, local_count);
                        }
                    }
                    Err(e) => {
                        if e.to_string().contains("429") {
                            warn!("Task {}: Rate limited by Discord. Backing off...", i);
                            sleep(Duration::from_secs(2)).await;
                        } else {
                            error!("Task {}: Error sending message: {}", i, e);
                            sleep(Duration::from_secs(1)).await;
                        }
                    }
                }
                
                // Extremely short sleep to avoid immediate 429 but stay aggressive
                sleep(Duration::from_millis(100)).await;
            }
        });
        handles.push(handle);
    }

    // Keep the main task alive
    info!("Load tester running with {} concurrent tasks.", concurrency);
    
    // We'll let it run for a while or indefinitely
    futures::future::join_all(handles).await;

    Ok(())
}
