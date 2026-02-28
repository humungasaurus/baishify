use crate::error::{AppError, Result};
use std::io::{self, IsTerminal, Read, Write};

pub fn resolve_prompt(prompt_from_args: Option<&str>) -> Result<String> {
    if let Some(p) = prompt_from_args {
        let trimmed = p.trim();
        if !trimmed.is_empty() {
            return Ok(trimmed.to_string());
        }
    }

    if io::stdin().is_terminal() {
        print!("What command do you want? ");
        io::stdout().flush()?;
        let mut line = String::new();
        io::stdin().read_line(&mut line)?;
        let trimmed = line.trim().to_string();
        if trimmed.is_empty() {
            return Err(AppError::from("missing prompt"));
        }
        Ok(trimmed)
    } else {
        let mut buf = String::new();
        io::stdin().read_to_string(&mut buf)?;
        let trimmed = buf.trim().to_string();
        if trimmed.is_empty() {
            return Err(AppError::from("missing prompt from stdin"));
        }
        Ok(trimmed)
    }
}
