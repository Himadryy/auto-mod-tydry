use anyhow::{Context, Result};
use redis::aio::ConnectionManager;
use std::time::Duration;
use tracing::warn;

pub struct RateLimiter {
    pub redis: ConnectionManager,
}

impl RateLimiter {
    pub fn new(redis: ConnectionManager) -> Self {
        Self { redis }
    }

    /// Tracks an infraction and returns the new total count.
    pub async fn track_warning(&mut self, user_id: &str) -> Result<i64> {
        let key = format!("warnings:{}", user_id);

        let (count, _): (i64, i64) = redis::pipe()
            .atomic()
            .incr(&key, 1)
            .expire(&key, 86400) // 24 hours
            .query_async(&mut self.redis)
            .await
            .context("Redis warning pipeline failed")?;

        Ok(count)
    }

    /// Checks if an action exceeds the rate limit using a simple counter with expiration.
    /// Returns `true` if the action is ALLOWED, `false` if it is BLOCKED (Rate Limited).
    pub async fn check_limit(
        &mut self,
        user_id: &str,
        action_type: &str,
        max_actions: i64,
        window: Duration,
    ) -> Result<bool> {
        let key = format!("ratelimit:{}:{}", action_type, user_id);

        // Use a pipeline to increment and set expiration atomically
        let (count, _): (i64, i64) = redis::pipe()
            .atomic()
            .incr(&key, 1)
            .expire(&key, window.as_secs() as i64)
            .query_async(&mut self.redis)
            .await
            .context("Redis rate limit pipeline failed")?;

        if count > max_actions {
            warn!(
                "RATE LIMIT EXCEEDED: User {} performed {} more than {} times in {}s",
                user_id,
                action_type,
                max_actions,
                window.as_secs()
            );
            return Ok(false); // Blocked
        }

        Ok(true) // Allowed
    }
}
