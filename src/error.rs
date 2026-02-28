use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("{0}")]
    Message(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("toml decode error: {0}")]
    TomlDe(#[from] toml::de::Error),
    #[error("toml encode error: {0}")]
    TomlSer(#[from] toml::ser::Error),
    #[error("request failed: {0}")]
    Request(#[from] ureq::Error),
    #[error("prompt failed: {0}")]
    Dialoguer(#[from] dialoguer::Error),
}

impl From<&str> for AppError {
    fn from(value: &str) -> Self {
        Self::Message(value.to_string())
    }
}

impl From<String> for AppError {
    fn from(value: String) -> Self {
        Self::Message(value)
    }
}

pub type Result<T> = std::result::Result<T, AppError>;
