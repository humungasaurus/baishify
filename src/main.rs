mod config;
mod error;
mod onboarding;
mod prompt;
mod providers;
mod shell_integration;
mod types;
mod ui;

use crate::config::{config_file_path, load_file_config, merge_cli_with_setup, parse_cli};
use crate::error::{AppError, Result};
use crate::onboarding::run_onboarding;
use crate::prompt::resolve_prompt;
use crate::providers::generate_once;
use crate::shell_integration::{detect_shell_from_env, install as install_shell, parse_shell_name};
use crate::ui::{emit_non_interactive, run_interactive};
use std::io::IsTerminal;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Err(err) = run(args) {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}

fn run(args: Vec<String>) -> Result<()> {
    if let Some(first) = args.first() {
        if first == "init" {
            let shell = args
                .get(1)
                .and_then(|s| parse_shell_name(s))
                .or_else(detect_shell_from_env)
                .ok_or_else(|| {
                    AppError::from("could not detect shell. Run `b init zsh` or `b init bash`.")
                })?;
            let result = install_shell(shell)?;
            if result.updated {
                println!(
                    "Installed shell integration for {} at {}",
                    result.shell.as_str(),
                    result.rc_path.display()
                );
            } else {
                println!(
                    "Shell integration already up to date for {} at {}",
                    result.shell.as_str(),
                    result.rc_path.display()
                );
            }
            println!("Restart shell or run: source {}", result.rc_path.display());
            return Ok(());
        }
    }

    let config_path = config_file_path()?;
    let file_config = load_file_config(&config_path)?;
    let mut config = parse_cli(args, file_config.clone())?;

    let agent = ureq::AgentBuilder::new().build();

    if config.setup {
        let _saved = run_onboarding(&config_path, file_config, &agent)?;
        return Ok(());
    }

    if config.provider_api_key_missing() {
        if std::io::stdin().is_terminal() && std::io::stdout().is_terminal() {
            eprintln!("No provider key found. Launching onboarding...");
            let saved = run_onboarding(&config_path, file_config, &agent)?;
            config = merge_cli_with_setup(config, saved)?;
        } else {
            return Err(AppError::from(
                "missing API key. Run `b setup` or set provider env key (OPENAI_API_KEY / ANTHROPIC_API_KEY / OPENROUTER_API_KEY / VERCEL_AI_GATEWAY_API_KEY)",
            ));
        }
    }

    let prompt = resolve_prompt(config.prompt.as_deref())?;
    let interactive = std::io::stdout().is_terminal() && !config.json && !config.plain;

    if interactive {
        run_interactive(&agent, &config, &prompt)?;
    } else {
        let output = generate_once(&agent, &config, &prompt)?;
        emit_non_interactive(&config, output)?;
    }
    Ok(())
}
