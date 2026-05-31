use lazy_static::lazy_static;
use regex::Regex;
use tracing::info;

pub mod ai;

lazy_static! {
    // Basic Zalgo detection (detects excessive combining characters)
    static ref ZALGO_REGEX: Regex = Regex::new(r"[\x{0300}-\x{036F}\x{1AB0}-\x{1AFF}\x{1DC0}-\x{1DFF}\x{20D0}-\x{20FF}\x{FE20}-\x{FE2F}]{4,}").unwrap();
    
    // Discord Invite Links (including variations)
    static ref DISCORD_INVITE_REGEX: Regex = Regex::new(r"(?i)(discord\.(gg|io|me|li)|discordapp\.com/invite)/[a-zA-Z0-9]+").unwrap();
}

pub struct AutomodEngine;

impl AutomodEngine {
    /// Layer 1: Fast deterministic checks.
    /// Returns `true` if the message should be BLOCKED (deleted/warned).
    pub fn check_fast_path(content: &str) -> bool {
        // 1. Zalgo Check
        if ZALGO_REGEX.is_match(content) {
            info!("Automod: Blocked Zalgo text.");
            return true;
        }

        // 2. Unauthorized Invites
        if DISCORD_INVITE_REGEX.is_match(content) {
            info!("Automod: Blocked Discord Invite link.");
            return true;
        }

        // 3. Mass Mentions (Simple count)
        let mention_count = content.matches("<@").count();
        if mention_count > 5 {
            info!("Automod: Blocked Mass Mention ({} mentions).", mention_count);
            return true;
        }

        // Passed Layer 1
        false
    }
}
