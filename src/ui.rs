use crate::error::Result;
use crate::providers::generate_once;
use crate::types::{AppConfig, GenerationOutput, JsonOutput};
use dialoguer::console::{Key, Term};
use std::fs;
use std::io::{self, IsTerminal, Write};
use std::process::Command;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

pub fn run_interactive(agent: &ureq::Agent, config: &AppConfig, prompt: &str) -> Result<()> {
    let current_prompt = prompt.to_string();
    loop {
        let output = generate_with_loader(agent, config, &current_prompt)?;
        render_result_card(config, prompt, &output);

        loop {
            print!(
                "{}  {}  {}  {}  {}",
                paint("[Enter] use", Ansi::Dim),
                paint("[r] regenerate", Ansi::Dim),
                paint("[e] explain", Ansi::Dim),
                paint("[c] copy", Ansi::Dim),
                paint("[q] quit", Ansi::Dim),
            );
            println!();
            print!("{}", paint("action > ", Ansi::Dim));
            io::stdout().flush()?;

            let key = Term::stdout().read_key()?;
            println!();

            match key {
                Key::Enter => {
                    let cmd = output.command.trim();
                    if cmd.is_empty() {
                        println!("{}", paint("Generated command was empty.", Ansi::Yellow));
                        continue;
                    }
                    if let Some(path) = config.output_file.as_deref() {
                        fs::write(path, format!("{cmd}\n"))?;
                        return Ok(());
                    }
                    run_command(cmd)?;
                    return Ok(());
                }
                Key::Char(c) if c.eq_ignore_ascii_case(&'r') => {
                    if !config.no_fun {
                        println!("Trying a different phrasing path...");
                    }
                    break;
                }
                Key::Char(c) if c.eq_ignore_ascii_case(&'e') => {
                    println!();
                    println!("{}", paint("Explanation", Ansi::Cyan));
                    println!("{}", output.explanation.trim());
                    println!();
                    continue;
                }
                Key::Char(c) if c.eq_ignore_ascii_case(&'c') => {
                    if copy_to_clipboard(output.command.trim()) {
                        println!("{}", paint("Copied to clipboard.", Ansi::Green));
                    } else {
                        println!("{}", paint("Copy not supported on this system.", Ansi::Yellow));
                    }
                    continue;
                }
                Key::Char(c) if c.eq_ignore_ascii_case(&'q') => return Ok(()),
                _ => {
                    println!("{}", paint("Unknown key. Press Enter, r, e, c, or q.", Ansi::Yellow));
                    continue;
                }
            }
        }
    }
}

pub fn emit_non_interactive(config: &AppConfig, output: GenerationOutput) -> Result<()> {
    if config.json {
        let payload = JsonOutput {
            provider: config.provider.as_str().to_string(),
            model: config.model.clone(),
            command: output.command,
            explanation: output.explanation,
            safety: output.safety,
        };
        println!("{}", serde_json::to_string(&payload)?);
        return Ok(());
    }

    if config.explain {
        eprintln!("{}", output.explanation.trim());
    }
    println!("{}", output.command.trim());
    Ok(())
}

fn render_result_card(config: &AppConfig, prompt: &str, output: &GenerationOutput) {
    println!();
    println!("{} {}", paint("Prompt:", Ansi::Bold), prompt.trim());
    println!();
    println!("{}", paint("Command", Ansi::Cyan));
    println!("{}", output.command.trim());
    if config.explain {
        println!();
        println!("{}", paint("Explanation", Ansi::Cyan));
        println!("{}", output.explanation.trim());
    }
    println!();
}

fn generate_with_loader(
    agent: &ureq::Agent,
    config: &AppConfig,
    prompt: &str,
) -> Result<GenerationOutput> {
    let (tx, rx) = mpsc::channel::<Result<GenerationOutput>>();
    let cfg = config.clone();
    let prompt_owned = prompt.to_string();
    let agent = agent.clone();

    thread::spawn(move || {
        let result = generate_once(&agent, &cfg, &prompt_owned);
        let _ = tx.send(result);
    });

    let phases = ["thinking", "drafting", "refining", "finalizing"];
    let spinner = ['|', '/', '-', '\\'];
    let mut phase_idx = 0usize;
    let mut spin_idx = 0usize;
    let mut last_phase_tick = Instant::now();

    // Immediate feedback in same event-loop tick (<=30ms budget).
    draw_loader_line(spinner[spin_idx], phases[phase_idx], config.no_fun)?;

    loop {
        match rx.recv_timeout(Duration::from_millis(90)) {
            Ok(result) => {
                clear_line()?;
                return result;
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                spin_idx = (spin_idx + 1) % spinner.len();
                if last_phase_tick.elapsed() >= Duration::from_millis(850) {
                    phase_idx = (phase_idx + 1) % phases.len();
                    last_phase_tick = Instant::now();
                }
                draw_loader_line(spinner[spin_idx], phases[phase_idx], config.no_fun)?;
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                return Err("worker disconnected".into());
            }
        }
    }
}

fn draw_loader_line(spin: char, phase: &str, no_fun: bool) -> Result<()> {
    clear_line()?;
    let _ = no_fun;
    print!("{spin} {phase}...");
    io::stdout().flush()?;
    Ok(())
}

fn clear_line() -> Result<()> {
    // ANSI clear line + carriage return keeps loader on a single stable row.
    print!("\x1b[2K\r");
    io::stdout().flush()?;
    Ok(())
}

fn copy_to_clipboard(text: &str) -> bool {
    #[cfg(target_os = "macos")]
    {
        if let Ok(mut child) = Command::new("pbcopy")
            .stdin(std::process::Stdio::piped())
            .spawn()
        {
            if let Some(stdin) = child.stdin.as_mut() {
                if stdin.write_all(text.as_bytes()).is_ok() && child.wait().is_ok() {
                    return true;
                }
            }
        }
        false
    }
    #[cfg(target_os = "linux")]
    {
        if let Ok(mut child) = Command::new("xclip")
            .args(["-selection", "clipboard"])
            .stdin(std::process::Stdio::piped())
            .spawn()
        {
            if let Some(stdin) = child.stdin.as_mut() {
                if stdin.write_all(text.as_bytes()).is_ok() && child.wait().is_ok() {
                    return true;
                }
            }
        }
        false
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        let _ = text;
        false
    }
}

fn run_command(command: &str) -> Result<()> {
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string());
    let _status = Command::new(shell)
        .arg("-lc")
        .arg(command)
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()?;
    Ok(())
}

#[derive(Clone, Copy)]
enum Ansi {
    Bold,
    Dim,
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
        Ansi::Green => "32",
        Ansi::Yellow => "33",
        Ansi::Cyan => "36",
    };
    format!("\x1b[{code}m{text}\x1b[0m")
}
