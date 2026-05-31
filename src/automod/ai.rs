use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;
use tracing::{error, info};

#[derive(Serialize)]
struct OllamaRequest {
    model: String,
    prompt: String,
    stream: bool,
}

#[derive(Deserialize)]
struct OllamaResponse {
    response: String,
}

#[derive(Deserialize, Debug)]
pub struct AiDecision {
    pub is_malicious: bool,
    pub reason: String,
}

pub struct OllamaClient {
    client: Client,
    url: String,
    model: String,
}

impl OllamaClient {
    pub fn new() -> Result<Self> {
        let url = env::var("OLLAMA_URL").context("OLLAMA_URL must be set")?;
        let model = env::var("OLLAMA_MODEL").context("OLLAMA_MODEL must be set")?;
        
        Ok(Self {
            client: Client::new(),
            url,
            model,
        })
    }

    /// Evaluates a message using the local Ollama (Deepseek) instance.
    /// Returns an `AiDecision` containing whether it's malicious and why.
    pub async fn evaluate_message(&self, content: &str) -> Result<AiDecision> {
        // We instruct Deepseek to return strictly JSON.
        let prompt = format!(
            "You are a strict Discord Auto-Moderation AI. Analyze the following message for spam, phishing, intense toxicity, or malicious intent.\n\n\
            Message: \"{}\"\n\n\
            Respond ONLY with a valid JSON object in this exact format, with no markdown formatting or extra text:\n\
            {{\"is_malicious\": true/false, \"reason\": \"short explanation\"}}",
            content
        );

        let req_body = OllamaRequest {
            model: self.model.clone(),
            prompt,
            stream: false,
        };

        let res = self.client
            .post(format!("{}/api/generate", self.url))
            .json(&req_body)
            .send()
            .await
            .context("Failed to send request to Ollama")?;

        if !res.status().is_success() {
            error!("Ollama API returned an error: {}", res.status());
            anyhow::bail!("Ollama API error");
        }

        let ollama_res: OllamaResponse = res.json().await.context("Failed to parse Ollama response")?;
        
        info!("Raw Ollama Response: {}", ollama_res.response);

        // Deepseek might still wrap it in ```json ... ``` despite instructions, so we clean it.
        let cleaned_json = ollama_res.response
            .replace("```json", "")
            .replace("```", "")
            .trim()
            .to_string();

        let decision: AiDecision = serde_json::from_str(&cleaned_json)
            .context(format!("Failed to parse AI response into JSON. Raw: {}", cleaned_json))?;

        Ok(decision)
    }
}
