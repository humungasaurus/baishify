use crate::config::{detected_provider_keys, save_file_config};
use crate::error::{AppError, Result};
use crate::providers::generate_once;
use crate::shell_integration::{detect_shell_from_env, install as install_shell};
use crate::types::{AppConfig, FileConfig, Provider};
use dialoguer::{theme::ColorfulTheme, Confirm, FuzzySelect, Input, Password, Select};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::io::{self, IsTerminal, Write};
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub fn run_onboarding(
    config_path: &PathBuf,
    existing: Option<FileConfig>,
    agent: &ureq::Agent,
) -> Result<FileConfig> {
    render_intro();

    let theme = ColorfulTheme::default();
    let detected = detected_provider_keys();
    if !detected.is_empty() {
        let names = detected
            .iter()
            .map(|(p, _)| p.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        println!("{} {names}", paint("Found keys:", Ansi::Green));
    } else {
        println!("{}", paint("No keys found in env. We can paste one in.", Ansi::Yellow));
    }
    divider();

    step("1/3", "Provider");
    let provider = select_provider(&theme, existing.as_ref().and_then(|c| c.provider), &detected)?;
    println!(
        "{} {}",
        paint("Selected:", Ansi::Dim),
        paint(provider.as_str(), Ansi::Bold)
    );
    divider();

    step("2/3", "Credentials");
    let key = select_api_key(&theme, provider, &detected, existing.as_ref())?;
    let base_url = provider.default_base_url().to_string();
    divider();

    step("3/3", "Model");
    let existing_model = existing.as_ref().and_then(|c| c.model.clone());
    let model = select_model(
        &theme,
        agent,
        provider,
        &base_url,
        &key,
        existing_model,
    )?;
    println!("{} {}", paint("Base URL:", Ansi::Dim), paint(&base_url, Ansi::Dim));
    divider();

    let staged = AppConfig {
        provider,
        model: model.clone(),
        base_url: base_url.clone(),
        api_key: key.clone(),
        explain: false,
        json: false,
        plain: true,
        no_fun: false,
        setup: false,
        prompt: None,
        output_file: None,
    };

    print!("{} ", paint("Running a tiny test prompt...", Ansi::Cyan));
    io::stdout().flush()?;
    let test = generate_once(agent, &staged, "print current directory");
    match test {
        Ok(_) => println!("{}", paint("nice, connection looks good.", Ansi::Green)),
        Err(e) => {
            println!("{}", paint("nope, that didn't work.", Ansi::Red));
            return Err(AppError::from(format!("provider test failed: {e}")));
        }
    }

    let saved = FileConfig {
        provider: Some(provider),
        model: Some(model),
        base_url: Some(base_url),
        api_key: Some(key),
        no_fun: existing.as_ref().and_then(|c| c.no_fun).or(Some(false)),
    };
    save_file_config(config_path, &saved)?;
    println!();
    println!("{}", paint("Setup complete.", Ansi::Green));
    println!("{}", paint("Saved config: ~/.config/baishify/config.toml", Ansi::Dim));
    maybe_install_shell_integration(&theme)?;
    Ok(saved)
}

fn select_provider(
    theme: &ColorfulTheme,
    default: Option<Provider>,
    detected: &[(Provider, String)],
) -> Result<Provider> {
    let items = vec![
        "openai      OpenAI",
        "anthropic   Anthropic",
        "openrouter  OpenRouter",
        "vercel      Vercel AI Gateway",
    ];

    let suggested = default
        .or_else(|| detected.first().map(|(p, _)| *p))
        .unwrap_or(Provider::Openai);
    let default_idx = match suggested {
        Provider::Openai => 0,
        Provider::Anthropic => 1,
        Provider::Openrouter => 2,
        Provider::Vercel => 3,
    };

    let idx = Select::with_theme(theme)
        .with_prompt("Pick your model provider")
        .items(&items)
        .default(default_idx)
        .interact()?;

    let provider = match idx {
        0 => Provider::Openai,
        1 => Provider::Anthropic,
        2 => Provider::Openrouter,
        3 => Provider::Vercel,
        _ => return Err(AppError::from("invalid provider selection")),
    };
    Ok(provider)
}

fn select_model(
    theme: &ColorfulTheme,
    agent: &ureq::Agent,
    provider: Provider,
    base_url: &str,
    api_key: &str,
    existing_model: Option<String>,
) -> Result<String> {
    println!("{}", paint("Loading models...", Ansi::Dim));
    let mut items: Vec<String> = resolve_model_candidates(agent, provider, base_url, api_key)?;

    let default_model = existing_model.unwrap_or_else(|| provider.default_model().to_string());
    if !items.iter().any(|m| m == &default_model) {
        items.insert(0, default_model.clone());
    }
    items.push("Custom model id...".to_string());

    let default_idx = items.iter().position(|m| m == &default_model).unwrap_or(0);
    let idx = FuzzySelect::with_theme(theme)
        .with_prompt("Select model (type to search)")
        .items(&items)
        .default(default_idx)
        .interact()?;

    if items[idx] == "Custom model id..." {
        loop {
            let value: String = Input::with_theme(theme)
                .with_prompt("Enter model id")
                .interact_text()?;
            if !value.trim().is_empty() {
                return Ok(value.trim().to_string());
            }
            println!("{}", paint("Model id cannot be empty.", Ansi::Yellow));
        }
    }

    Ok(items[idx].clone())
}

fn resolve_model_candidates(
    agent: &ureq::Agent,
    provider: Provider,
    base_url: &str,
    api_key: &str,
) -> Result<Vec<String>> {
    match fetch_live_models(agent, provider, base_url, api_key) {
        Ok(mut models) if !models.is_empty() => {
            models.sort();
            models.dedup();
            save_models_cache(provider, &models);
            println!(
                "{} {}",
                paint("Loaded models from API:", Ansi::Green),
                paint(&models.len().to_string(), Ansi::Bold)
            );
            Ok(models)
        }
        Ok(_) | Err(_) => {
            if let Some(cached) = load_models_cache(provider) {
                println!("{}", paint("Using cached model list.", Ansi::Yellow));
                return Ok(cached);
            }
            println!("{}", paint("Using built-in model list.", Ansi::Yellow));
            Ok(model_candidates(provider)
                .into_iter()
                .map(str::to_string)
                .collect())
        }
    }
}

fn fetch_live_models(
    agent: &ureq::Agent,
    provider: Provider,
    base_url: &str,
    api_key: &str,
) -> Result<Vec<String>> {
    let url = match provider {
        Provider::Anthropic => format!("{}/v1/models", base_url.trim_end_matches('/')),
        _ => format!("{}/models", base_url.trim_end_matches('/')),
    };

    let mut req = agent.get(&url).timeout(Duration::from_secs(4));
    match provider {
        Provider::Openai => {
            req = req.set("Authorization", &format!("Bearer {api_key}"));
        }
        Provider::Openrouter => {
            req = req
                .set("Authorization", &format!("Bearer {api_key}"))
                .set("HTTP-Referer", "https://github.com/danielhostetler/baishify")
                .set("X-Title", "baishify");
        }
        Provider::Vercel => {
            req = req
                .set("Authorization", &format!("Bearer {api_key}"))
                .set("X-Vercel-AI-Gateway-Api-Key", api_key);
        }
        Provider::Anthropic => {
            req = req
                .set("x-api-key", api_key)
                .set("anthropic-version", "2023-06-01");
        }
    }

    let value: Value = req.call()?.into_json()?;
    Ok(extract_model_ids(value))
}

fn extract_model_ids(value: Value) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(array) = value.get("data").and_then(|v| v.as_array()) {
        for item in array {
            if let Some(id) = item.get("id").and_then(|v| v.as_str()) {
                out.push(id.to_string());
            }
        }
        return out;
    }
    if let Some(array) = value.as_array() {
        for item in array {
            if let Some(id) = item.get("id").and_then(|v| v.as_str()) {
                out.push(id.to_string());
            }
        }
    }
    out
}

#[derive(Debug, Serialize, Deserialize)]
struct ModelCache {
    fetched_at_epoch: u64,
    models: Vec<String>,
}

fn models_cache_path(provider: Provider) -> Option<PathBuf> {
    let mut dir = dirs::config_dir()?;
    dir.push("baishify");
    dir.push(format!("models-{}.json", provider.as_str()));
    Some(dir)
}

fn load_models_cache(provider: Provider) -> Option<Vec<String>> {
    let path = models_cache_path(provider)?;
    let raw = fs::read_to_string(path).ok()?;
    let cache: ModelCache = serde_json::from_str(&raw).ok()?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()?
        .as_secs();
    if now.saturating_sub(cache.fetched_at_epoch) > 86_400 {
        return None;
    }
    if cache.models.is_empty() {
        return None;
    }
    Some(cache.models)
}

fn save_models_cache(provider: Provider, models: &[String]) {
    let Some(path) = models_cache_path(provider) else {
        return;
    };
    if let Some(parent) = path.parent() {
        if fs::create_dir_all(parent).is_err() {
            return;
        }
    }
    let now = match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(d) => d.as_secs(),
        Err(_) => 0,
    };
    let payload = ModelCache {
        fetched_at_epoch: now,
        models: models.to_vec(),
    };
    if let Ok(raw) = serde_json::to_string_pretty(&payload) {
        let _ = fs::write(path, raw);
    }
}

fn select_api_key(
    theme: &ColorfulTheme,
    provider: Provider,
    detected: &[(Provider, String)],
    existing: Option<&FileConfig>,
) -> Result<String> {
    if let Some((_, key)) = detected.iter().find(|(p, _)| *p == provider) {
        let use_detected = Confirm::with_theme(theme)
            .with_prompt("Use detected env key?")
            .default(true)
            .interact()?;
        if use_detected {
            return Ok(key.clone());
        }
    }

    if let Some(saved_key) = existing.and_then(|c| c.api_key.clone()) {
        let use_saved = Confirm::with_theme(theme)
            .with_prompt("Use existing saved key?")
            .default(true)
            .interact()?;
        if use_saved {
            return Ok(saved_key);
        }
    }

    loop {
        let value = Password::with_theme(theme)
            .with_prompt("API key")
            .allow_empty_password(true)
            .interact()?;
        if !value.trim().is_empty() {
            return Ok(value);
        }
        println!(
            "{}",
            paint("Key was empty. Paste one in, or Ctrl+C to bail out.", Ansi::Yellow)
        );
    }
}

fn model_candidates(provider: Provider) -> Vec<&'static str> {
    match provider {
        Provider::Openai => vec![
            "openai-codex/gpt-5.3-codex",
            "openai-codex/gpt-5.1-codex",
            "gpt-5-mini-2025-08-07",
            "gpt-5",
            "gpt-5-mini",
            "gpt-5-nano",
            "gpt-4o",
            "gpt-4o-mini",
        ],
        Provider::Anthropic => vec![
            "claude-3-7-sonnet-latest",
            "claude-3-5-sonnet-latest",
            "claude-3-5-haiku-latest",
        ],
        Provider::Openrouter => vec![
            "openai-codex/gpt-5.3-codex",
            "openai-codex/gpt-5.1-codex",
            "openai/gpt-5-mini-2025-08-07",
            "openai/gpt-5",
            "openai/gpt-5-nano",
            "openai/gpt-4o-mini",
            "anthropic/claude-3.5-sonnet",
            "google/gemini-2.5-flash",
        ],
        Provider::Vercel => vec![
            "openai-codex/gpt-5.3-codex",
            "openai-codex/gpt-5.1-codex",
            "openai/gpt-5-mini-2025-08-07",
            "openai/gpt-5",
            "openai/gpt-5-nano",
            "openai/gpt-4o-mini",
            "anthropic/claude-3-5-sonnet-latest",
        ],
    }
}

fn render_intro() {
    println!();
    println!("{}", paint("┌─────────────────────────────────────────────┐", Ansi::Dim));
    println!(
        "{}",
        paint("│ b setup                                     │", Ansi::Cyan)
    );
    println!(
        "{}",
        paint("│ Prompt -> command in under a minute.        │", Ansi::Dim)
    );
    println!("{}", paint("└─────────────────────────────────────────────┘", Ansi::Dim));
    println!();
}

fn maybe_install_shell_integration(theme: &ColorfulTheme) -> Result<()> {
    let Some(shell) = detect_shell_from_env() else {
        println!(
            "{}",
            paint(
                "Tip: run `b init zsh` or `b init bash` for parent-shell execution + history.",
                Ansi::Dim
            )
        );
        return Ok(());
    };

    let should_install = Confirm::with_theme(theme)
        .with_prompt(format!(
            "Install shell integration for {}? (recommended)",
            shell.as_str()
        ))
        .default(true)
        .interact()?;
    if !should_install {
        println!(
            "{}",
            paint(
                "Skipped. You can run `b init` anytime to enable parent-shell execution + history.",
                Ansi::Dim
            )
        );
        return Ok(());
    }

    let installed = install_shell(shell)?;
    if installed.updated {
        println!(
            "{} {}",
            paint("Installed shell integration:", Ansi::Green),
            installed.rc_path.display()
        );
    } else {
        println!(
            "{} {}",
            paint("Shell integration already up to date:", Ansi::Green),
            installed.rc_path.display()
        );
    }
    println!(
        "{} {}",
        paint("Reload shell config:", Ansi::Dim),
        paint(&format!("source {}", installed.rc_path.display()), Ansi::Bold)
    );
    Ok(())
}

fn step(id: &str, name: &str) {
    println!(
        "{} {} {}",
        paint("[", Ansi::Dim),
        paint(id, Ansi::Cyan),
        paint("]", Ansi::Dim)
    );
    println!("{}", paint(name, Ansi::Bold));
    println!();
}

fn divider() {
    println!();
    println!("{}", paint("──────────────────────────────────────────────", Ansi::Dim));
    println!();
}

#[derive(Clone, Copy)]
enum Ansi {
    Bold,
    Dim,
    Red,
    Green,
    Yellow,
    Cyan,
}

fn paint(text: &str, color: Ansi) -> String {
    if !io::stdout().is_terminal() || std::env::var("NO_COLOR").is_ok() {
        return text.to_string();
    }
    let code = match color {
        Ansi::Bold => "1",
        Ansi::Dim => "2",
        Ansi::Red => "31",
        Ansi::Green => "32",
        Ansi::Yellow => "33",
        Ansi::Cyan => "36",
    };
    format!("\x1b[{code}m{text}\x1b[0m")
}
