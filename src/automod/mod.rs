use lazy_static::lazy_static;
use regex::Regex;
use tracing::info;

pub mod ai;

lazy_static! {
    // Basic Zalgo detection (detects excessive combining characters)
    static ref ZALGO_REGEX: Regex = Regex::new(r"[\x{0300}-\x{036F}\x{1AB0}-\x{1AFF}\x{1DC0}-\x{1DFF}\x{20D0}-\x{20FF}\x{FE20}-\x{FE2F}]{4,}").unwrap();

    // Discord Invite Links (including variations)
    static ref DISCORD_INVITE_REGEX: Regex = Regex::new(r"(?i)(discord\.(gg|io|me|li)|discordapp\.com/invite)/[a-zA-Z0-9]+").unwrap();

    // Basic bad words regex
    static ref BAD_WORDS_REGEX: Regex = Regex::new(r"(?i)\b(fuck|shit|bitch|asshole|cunt)\b").unwrap();
}

pub enum AutomodAction {
    Block(&'static str),
    BadWord,
    None,
}

pub struct AutomodEngine;

impl AutomodEngine {
    /// Layer 1: Fast deterministic checks.
    pub fn check_fast_path(content: &str) -> AutomodAction {
        // 1. Zalgo Check
        if ZALGO_REGEX.is_match(content) {
            info!("Automod: Blocked Zalgo text.");
            return AutomodAction::Block("Zalgo");
        }

        // 2. Unauthorized Invites
        if DISCORD_INVITE_REGEX.is_match(content) {
            info!("Automod: Blocked Discord Invite link.");
            return AutomodAction::Block("Invite Link");
        }

        // 3. Bad Words Check
        if BAD_WORDS_REGEX.is_match(content) {
            info!("Automod: Blocked Bad Word.");
            return AutomodAction::BadWord;
        }

        // 4. Mass Mentions (Simple count)
        let mention_count = content.matches("<@").count();
        if mention_count > 5 {
            info!(
                "Automod: Blocked Mass Mention ({} mentions).",
                mention_count
            );
            return AutomodAction::Block("Mass Mentions");
        }

        // Passed Layer 1
        AutomodAction::None
    }
}
