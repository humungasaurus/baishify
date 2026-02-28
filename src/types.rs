use serde::{Deserialize, Serialize};

pub const DEFAULT_OPENAI_BASE_URL: &str = "https://api.openai.com/v1";
pub const DEFAULT_OPENAI_MODEL: &str = "gpt-4o-mini";
pub const DEFAULT_ANTHROPIC_BASE_URL: &str = "https://api.anthropic.com";
pub const DEFAULT_ANTHROPIC_MODEL: &str = "claude-3-5-haiku-latest";
pub const DEFAULT_OPENROUTER_BASE_URL: &str = "https://openrouter.ai/api/v1";
pub const DEFAULT_OPENROUTER_MODEL: &str = "openai/gpt-4o-mini";
pub const DEFAULT_VERCEL_BASE_URL: &str = "https://ai-gateway.vercel.sh/v1";
pub const DEFAULT_VERCEL_MODEL: &str = "openai/gpt-4o-mini";

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    Openai,
    Anthropic,
    Openrouter,
    Vercel,
}

impl Provider {
    pub fn parse(input: &str) -> Option<Self> {
        match input.to_ascii_lowercase().as_str() {
            "openai" => Some(Self::Openai),
            "anthropic" => Some(Self::Anthropic),
            "openrouter" => Some(Self::Openrouter),
            "vercel" | "vercel-ai-gateway" | "gateway" => Some(Self::Vercel),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Provider::Openai => "openai",
            Provider::Anthropic => "anthropic",
            Provider::Openrouter => "openrouter",
            Provider::Vercel => "vercel",
        }
    }

    pub fn default_base_url(self) -> &'static str {
        match self {
            Provider::Openai => DEFAULT_OPENAI_BASE_URL,
            Provider::Anthropic => DEFAULT_ANTHROPIC_BASE_URL,
            Provider::Openrouter => DEFAULT_OPENROUTER_BASE_URL,
            Provider::Vercel => DEFAULT_VERCEL_BASE_URL,
        }
    }

    pub fn default_model(self) -> &'static str {
        match self {
            Provider::Openai => DEFAULT_OPENAI_MODEL,
            Provider::Anthropic => DEFAULT_ANTHROPIC_MODEL,
            Provider::Openrouter => DEFAULT_OPENROUTER_MODEL,
            Provider::Vercel => DEFAULT_VERCEL_MODEL,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FileConfig {
    pub provider: Option<Provider>,
    pub model: Option<String>,
    pub base_url: Option<String>,
    pub api_key: Option<String>,
    pub no_fun: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub provider: Provider,
    pub model: String,
    pub base_url: String,
    pub api_key: String,
    pub explain: bool,
    pub json: bool,
    pub plain: bool,
    pub no_fun: bool,
    pub setup: bool,
    pub prompt: Option<String>,
    pub output_file: Option<String>,
}

impl AppConfig {
    pub fn provider_api_key_missing(&self) -> bool {
        self.api_key.trim().is_empty()
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct GenerationOutput {
    pub command: String,
    pub explanation: String,
    pub safety: String,
}

#[derive(Debug, Serialize)]
pub struct JsonOutput {
    pub provider: String,
    pub model: String,
    pub command: String,
    pub explanation: String,
    pub safety: String,
}
