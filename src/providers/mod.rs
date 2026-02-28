use crate::error::{AppError, Result};
use crate::types::{AppConfig, GenerationOutput, Provider};
use serde::Deserialize;
use serde_json::json;

pub trait ProviderClient {
    fn generate(&self, agent: &ureq::Agent, config: &AppConfig, prompt: &str) -> Result<GenerationOutput>;
}

struct OpenAIClient;
struct OpenRouterClient;
struct VercelClient;
struct AnthropicClient;

pub fn generate_once(agent: &ureq::Agent, config: &AppConfig, prompt: &str) -> Result<GenerationOutput> {
    let client: Box<dyn ProviderClient> = match config.provider {
        Provider::Openai => Box::new(OpenAIClient),
        Provider::Openrouter => Box::new(OpenRouterClient),
        Provider::Vercel => Box::new(VercelClient),
        Provider::Anthropic => Box::new(AnthropicClient),
    };
    client.generate(agent, config, prompt)
}

impl ProviderClient for OpenAIClient {
    fn generate(&self, agent: &ureq::Agent, config: &AppConfig, prompt: &str) -> Result<GenerationOutput> {
        openai_like(agent, config, prompt, OpenAILikeMode::OpenAI)
    }
}

impl ProviderClient for OpenRouterClient {
    fn generate(&self, agent: &ureq::Agent, config: &AppConfig, prompt: &str) -> Result<GenerationOutput> {
        openai_like(agent, config, prompt, OpenAILikeMode::OpenRouter)
    }
}

impl ProviderClient for VercelClient {
    fn generate(&self, agent: &ureq::Agent, config: &AppConfig, prompt: &str) -> Result<GenerationOutput> {
        openai_like(agent, config, prompt, OpenAILikeMode::Vercel)
    }
}

impl ProviderClient for AnthropicClient {
    fn generate(&self, agent: &ureq::Agent, config: &AppConfig, prompt: &str) -> Result<GenerationOutput> {
        let url = format!("{}/v1/messages", config.base_url.trim_end_matches('/'));
        let body = json!({
            "model": config.model,
            "max_tokens": 300,
            "temperature": 0,
            "system": system_prompt(),
            "messages": [
                {"role": "user", "content": format!("User request: {}", prompt)}
            ]
        });

        let response: AnthropicResponse = agent
            .post(&url)
            .set("Content-Type", "application/json")
            .set("x-api-key", &config.api_key)
            .set("anthropic-version", "2023-06-01")
            .send_json(body)?
            .into_json()?;

        let content = response
            .content
            .into_iter()
            .find(|c| c.type_name == "text")
            .and_then(|c| c.text)
            .ok_or_else(|| AppError::from("no text content returned"))?;

        parse_model_output(&content)
    }
}

enum OpenAILikeMode {
    OpenAI,
    OpenRouter,
    Vercel,
}

fn openai_like(
    agent: &ureq::Agent,
    config: &AppConfig,
    prompt: &str,
    mode: OpenAILikeMode,
) -> Result<GenerationOutput> {
    let url = format!("{}/chat/completions", config.base_url.trim_end_matches('/'));
    let body = json!({
        "model": config.model,
        "temperature": 0,
        "messages": [
            {"role": "system", "content": system_prompt()},
            {"role": "user", "content": format!("User request: {}", prompt)}
        ]
    });

    let mut req = agent
        .post(&url)
        .set("Content-Type", "application/json")
        .set("Authorization", &format!("Bearer {}", config.api_key));

    match mode {
        OpenAILikeMode::OpenAI => {}
        OpenAILikeMode::OpenRouter => {
            req = req
                .set("HTTP-Referer", "https://github.com/danielhostetler/baishify")
                .set("X-Title", "baishify");
        }
        OpenAILikeMode::Vercel => {
            req = req.set("X-Vercel-AI-Gateway-Api-Key", &config.api_key);
        }
    }

    let response: OpenAIResponse = req.send_json(body)?.into_json()?;
    let content = response
        .choices
        .into_iter()
        .next()
        .ok_or_else(|| AppError::from("no choices returned"))?
        .message
        .content;

    parse_model_output(&content)
}

fn system_prompt() -> &'static str {
    "You convert natural language intent into exactly one bash command. Return JSON only with keys: command, explanation, safety. safety must be one of safe|caution|risky. command must be plain bash (no backticks, no markdown, no leading $). Keep commands concise and practical for macOS/Linux."
}

fn parse_model_output(content: &str) -> Result<GenerationOutput> {
    if let Ok(mut parsed) = serde_json::from_str::<GenerationOutput>(content) {
        parsed.safety = normalize_safety(&parsed.safety, &parsed.command);
        return Ok(parsed);
    }

    let cleaned = content.trim().trim_matches('`').trim().to_string();
    let command = cleaned
        .lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or("")
        .trim()
        .to_string();
    if command.is_empty() {
        return Err(AppError::from("model returned empty output"));
    }

    Ok(GenerationOutput {
        command: command.clone(),
        explanation: "Model did not provide structured explanation.".to_string(),
        safety: normalize_safety("caution", &command),
    })
}

fn normalize_safety(raw: &str, command: &str) -> String {
    let norm = raw.trim().to_ascii_lowercase();
    if norm == "safe" || norm == "caution" || norm == "risky" {
        return norm;
    }

    let lower = command.to_ascii_lowercase();
    let risky = ["rm -rf", "mkfs", "dd if=", "shutdown", "reboot"];
    if risky.iter().any(|p| lower.contains(p)) {
        "risky".to_string()
    } else if lower.contains("sudo") || lower.contains("chmod 777") {
        "caution".to_string()
    } else {
        "safe".to_string()
    }
}

#[derive(Debug, Deserialize)]
struct OpenAIResponse {
    choices: Vec<OpenAIChoice>,
}

#[derive(Debug, Deserialize)]
struct OpenAIChoice {
    message: OpenAIMessage,
}

#[derive(Debug, Deserialize)]
struct OpenAIMessage {
    content: String,
}

#[derive(Debug, Deserialize)]
struct AnthropicResponse {
    content: Vec<AnthropicContent>,
}

#[derive(Debug, Deserialize)]
struct AnthropicContent {
    #[serde(rename = "type")]
    type_name: String,
    text: Option<String>,
}
