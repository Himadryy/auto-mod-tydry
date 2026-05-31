# Project Aegis (Auto-Mod-Tydry) 🛡️

> **Privacy-Preserving Edge AI & High-Concurrency Threat Detection for Real-Time Platforms**

A hyper-optimized, Rust-based Discord Antinuke & Automod system featuring Redis-backed microsecond rate limiting and a hybrid local AI defense layer for zero-day phishing detection.

## 📖 Overview

Traditional security bots rely purely on deterministic rules (Regex/Blacklists). Malicious actors easily bypass these using zero-day phishing links, obfuscated text, and social engineering. Furthermore, standard Node.js or Python bots frequently get rate-limited or freeze during the event loop when processing massive parallel API events during a "Nuke" attack (rogue admins deleting hundreds of channels/banning users).

**Project Aegis** solves these problems using a two-tier hybrid moderation architecture built on Rust and the Twilight ecosystem:

1. **Layer 1 (The Shield):** A strictly non-blocking, multi-threaded engine utilizing a Redis "Token-Bucket" algorithm via atomic pipelines. It tracks and thresholds thousands of state-changes per second across distributed shards, mitigating rogue permissions and blocking known bad regex patterns in $<1ms$.
2. **Layer 2 (The Brain):** Complex, borderline, and novel payloads are routed asynchronously to an Edge AI Microservice (Local Deepseek/Ollama over a Tailscale tunnel). This allows for deep semantic evaluation of zero-day phishing and social engineering without exposing user data to third-party corporate APIs (OpenAI/Cloud).

## 🚀 Key Features

- **Microsecond Rate Limiting:** Redis Pipeline-powered Token-Bucket limits for all admin and user actions.
- **Asynchronous AI Integration:** Non-blocking Tokio tasks ferry message context to a local LLM to prevent zero-day attacks without slowing down the primary gateway shard.
- **Privacy-First (Local LLM):** 100% of user data stays on your hardware. Inference is done using edge LLMs.
- **Memory Safe & Memory Fast:** Built entirely in Rust, maximizing CPU utilization and memory safety.

## 🛠️ Technology Stack

- **Core Engine:** Rust (Edition 2021)
- **Async Runtime:** Tokio
- **Discord Gateway & HTTP:** Twilight (`twilight-rs`)
- **State & Rate Limiting:** Redis (`redis-rs`)
- **Long-term Storage:** PostgreSQL (Supabase / `sqlx`)
- **AI Microservice:** Local Ollama (`deepseek-r1`)

## ⚙️ Installation & Setup

### Prerequisites
1. [Rust](https://www.rust-lang.org/tools/install) installed (`cargo`).
2. A running [Redis](https://redis.io/) instance.
3. A [Supabase](https://supabase.com/) PostgreSQL database.
4. [Ollama](https://ollama.com/) running locally or over a secure tunnel.

### Configuration
1. Clone the repository:
   ```bash
   git clone https://github.com/Himadryy/auto-mod-tydry.git
   cd auto-mod-tydry
   ```
2. Create a `.env` file in the root directory based on `.env.example`:
   ```env
   DISCORD_TOKEN=your_bot_token_here
   REDIS_URL=redis://...
   DATABASE_URL=postgresql://...
   OLLAMA_URL=http://localhost:11434
   OLLAMA_MODEL=deepseek-r1:8b
   ```
3. Run the bot:
   ```bash
   cargo run --release
   ```

## 📜 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
