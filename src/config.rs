use crate::error::{AppError, Result};
use crate::types::{AppConfig, FileConfig, Provider};
use std::env;
use std::fs;
use std::path::PathBuf;

pub fn config_file_path() -> Result<PathBuf> {
    let mut dir =
        dirs::config_dir().ok_or_else(|| AppError::from("unable to locate config directory"))?;
    dir.push("baishify");
    Ok(dir.join("config.toml"))
}

pub fn load_file_config(path: &PathBuf) -> Result<Option<FileConfig>> {
    if !path.exists() {
        return Ok(None);
    }
    let content = fs::read_to_string(path)?;
    let cfg: FileConfig = toml::from_str(&content)?;
    Ok(Some(cfg))
}

pub fn save_file_config(path: &PathBuf, cfg: &FileConfig) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let content = toml::to_string_pretty(cfg)?;
    fs::write(path, content)?;
    Ok(())
}

pub fn parse_cli(args: Vec<String>, file_config: Option<FileConfig>) -> Result<AppConfig> {
    let mut explain = false;
    let mut json = false;
    let mut plain = false;
    let mut no_fun = false;
    let mut setup = false;
    let mut provider_override: Option<Provider> = None;
    let mut model_override: Option<String> = None;
    let mut base_url_override: Option<String> = None;
    let mut api_key_override: Option<String> = None;
    let mut output_file: Option<String> = None;
    let mut prompt_parts: Vec<String> = Vec::new();

    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                print_usage();
                std::process::exit(0);
            }
            "setup" => setup = true,
            "-e" | "--explain" => explain = true,
            "--json" => json = true,
            "--plain" => plain = true,
            "--no-fun" => no_fun = true,
            "--provider" => {
                let value = iter
                    .next()
                    .ok_or_else(|| AppError::from("--provider requires a value"))?;
                provider_override = Provider::parse(&value);
                if provider_override.is_none() {
                    return Err(AppError::from(format!(
                        "unsupported provider `{value}` (use: openai, anthropic, openrouter, vercel)"
                    )));
                }
            }
            "--model" => {
                let value = iter
                    .next()
                    .ok_or_else(|| AppError::from("--model requires a value"))?;
                model_override = Some(value);
            }
            "--base-url" => {
                let value = iter
                    .next()
                    .ok_or_else(|| AppError::from("--base-url requires a value"))?;
                base_url_override = Some(value);
            }
            "--api-key" => {
                let value = iter
                    .next()
                    .ok_or_else(|| AppError::from("--api-key requires a value"))?;
                api_key_override = Some(value);
            }
            "--output-file" => {
                let value = iter
                    .next()
                    .ok_or_else(|| AppError::from("--output-file requires a value"))?;
                output_file = Some(value);
            }
            _ => prompt_parts.push(arg),
        }
    }

    let cfg_provider = file_config.as_ref().and_then(|c| c.provider);
    let provider = provider_override
        .or_else(provider_from_env)
        .or(cfg_provider)
        .unwrap_or(Provider::Openai);

    let model = model_override
        .or_else(|| env_model_for(provider))
        .or_else(|| file_config.as_ref().and_then(|c| c.model.clone()))
        .unwrap_or_else(|| provider.default_model().to_string());

    let base_url = base_url_override
        .or_else(|| env_base_url_for(provider))
        .or_else(|| file_config.as_ref().and_then(|c| c.base_url.clone()))
        .unwrap_or_else(|| provider.default_base_url().to_string());

    let no_fun = no_fun
        || env::var("B_FUN").ok().as_deref() == Some("0")
        || file_config.as_ref().and_then(|c| c.no_fun).unwrap_or(false);

    let api_key = api_key_override
        .or_else(|| env_api_key_for(provider))
        .or_else(|| file_config.as_ref().and_then(|c| c.api_key.clone()))
        .unwrap_or_default();

    let prompt = if prompt_parts.is_empty() {
        None
    } else {
        Some(prompt_parts.join(" "))
    };

    Ok(AppConfig {
        provider,
        model,
        base_url,
        api_key,
        explain,
        json,
        plain,
        no_fun,
        setup,
        prompt,
        output_file,
    })
}

pub fn merge_cli_with_setup(mut config: AppConfig, setup: FileConfig) -> Result<AppConfig> {
    if config.api_key.is_empty() {
        config.api_key = setup
            .api_key
            .ok_or_else(|| AppError::from("setup did not return api key"))?;
    }
    if config.model == config.provider.default_model() {
        if let Some(model) = setup.model {
            config.model = model;
        }
    }
    if config.base_url == config.provider.default_base_url() {
        if let Some(base_url) = setup.base_url {
            config.base_url = base_url;
        }
    }
    if let Some(provider) = setup.provider {
        config.provider = provider;
    }
    Ok(config)
}

pub fn provider_from_env() -> Option<Provider> {
    env::var("BAISHIFY_PROVIDER")
        .ok()
        .and_then(|v| Provider::parse(&v))
}

pub fn env_model_for(provider: Provider) -> Option<String> {
    env::var("BAISHIFY_MODEL")
        .ok()
        .or_else(|| match provider {
            Provider::Openai => env::var("OPENAI_MODEL").ok(),
            Provider::Anthropic => env::var("ANTHROPIC_MODEL").ok(),
            Provider::Openrouter => env::var("OPENROUTER_MODEL").ok(),
            Provider::Vercel => env::var("VERCEL_AI_GATEWAY_MODEL").ok(),
        })
}

pub fn env_api_key_for(provider: Provider) -> Option<String> {
    match provider {
        Provider::Openai => env::var("OPENAI_API_KEY").ok(),
        Provider::Anthropic => env::var("ANTHROPIC_API_KEY").ok(),
        Provider::Openrouter => env::var("OPENROUTER_API_KEY").ok(),
        Provider::Vercel => env::var("VERCEL_AI_GATEWAY_API_KEY")
            .ok()
            .or_else(|| env::var("AI_GATEWAY_API_KEY").ok()),
    }
}

pub fn env_base_url_for(provider: Provider) -> Option<String> {
    env::var("BAISHIFY_BASE_URL").ok().or_else(|| match provider {
        Provider::Openai => env::var("OPENAI_BASE_URL").ok(),
        Provider::Anthropic => env::var("ANTHROPIC_BASE_URL").ok(),
        Provider::Openrouter => env::var("OPENROUTER_BASE_URL").ok(),
        Provider::Vercel => env::var("VERCEL_AI_GATEWAY_BASE_URL")
            .ok()
            .or_else(|| env::var("AI_GATEWAY_BASE_URL").ok()),
    })
}

pub fn detected_provider_keys() -> Vec<(Provider, String)> {
    let mut out = Vec::new();
    if let Ok(v) = env::var("OPENAI_API_KEY") {
        if !v.trim().is_empty() {
            out.push((Provider::Openai, v));
        }
    }
    if let Ok(v) = env::var("ANTHROPIC_API_KEY") {
        if !v.trim().is_empty() {
            out.push((Provider::Anthropic, v));
        }
    }
    if let Ok(v) = env::var("OPENROUTER_API_KEY") {
        if !v.trim().is_empty() {
            out.push((Provider::Openrouter, v));
        }
    }
    if let Ok(v) = env::var("VERCEL_AI_GATEWAY_API_KEY") {
        if !v.trim().is_empty() {
            out.push((Provider::Vercel, v));
        }
    } else if let Ok(v) = env::var("AI_GATEWAY_API_KEY") {
        if !v.trim().is_empty() {
            out.push((Provider::Vercel, v));
        }
    }
    out
}

pub fn print_usage() {
    println!(
        "b - prompt to bash command\n\
         \n\
         Usage:\n\
           b [options] <prompt>\n\
           echo \"<prompt>\" | b [options]\n\
           b setup\n\
           b init [zsh|bash]\n\
         \n\
         Options:\n\
           --provider <name>    openai | anthropic | openrouter | vercel\n\
           --model <name>       Override model\n\
           --base-url <url>     Override API base URL\n\
           --api-key <key>      Override API key\n\
           -e, --explain        Include explanation in output\n\
           --json               JSON output mode\n\
           --plain              Disable interactive rendering\n\
           --no-fun             Disable playful copy\n\
           -h, --help           Show help\n\
         \n\
         Interactive mode is default on TTY. Non-TTY prints command only."
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, OnceLock};

    fn env_lock() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(())).lock().expect("lock poisoned")
    }

    fn clear_env(keys: &[&str]) {
        for key in keys {
            std::env::remove_var(key);
        }
    }

    #[test]
    fn parse_cli_uses_provider_scoped_base_url_env() {
        let _guard = env_lock();
        clear_env(&[
            "BAISHIFY_PROVIDER",
            "BAISHIFY_BASE_URL",
            "OPENAI_BASE_URL",
            "OPENROUTER_BASE_URL",
            "OPENAI_API_KEY",
            "OPENROUTER_API_KEY",
        ]);
        std::env::set_var("BAISHIFY_PROVIDER", "openrouter");
        std::env::set_var("OPENAI_BASE_URL", "https://wrong.example/v1");
        std::env::set_var("OPENROUTER_BASE_URL", "https://right.example/v1");
        std::env::set_var("OPENROUTER_API_KEY", "k");

        let cfg = parse_cli(vec!["hello".to_string()], None).expect("parse failed");
        assert_eq!(cfg.provider, Provider::Openrouter);
        assert_eq!(cfg.base_url, "https://right.example/v1");
    }

    #[test]
    fn parse_cli_accepts_output_file_flag() {
        let _guard = env_lock();
        clear_env(&[
            "BAISHIFY_PROVIDER",
            "OPENAI_API_KEY",
            "OPENAI_BASE_URL",
            "BAISHIFY_BASE_URL",
        ]);
        std::env::set_var("OPENAI_API_KEY", "k");

        let cfg = parse_cli(
            vec![
                "--output-file".to_string(),
                "/tmp/cmd.out".to_string(),
                "list".to_string(),
                "files".to_string(),
            ],
            None,
        )
        .expect("parse failed");
        assert_eq!(cfg.output_file.as_deref(), Some("/tmp/cmd.out"));
        assert_eq!(cfg.prompt.as_deref(), Some("list files"));
    }
}
