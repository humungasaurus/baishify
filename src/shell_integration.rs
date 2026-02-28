use crate::error::{AppError, Result};
use std::fs;
use std::path::{Path, PathBuf};

const BEGIN_MARKER: &str = "# >>> baishify integration >>>";
const END_MARKER: &str = "# <<< baishify integration <<<";

#[derive(Debug, Clone, Copy)]
pub enum ShellKind {
    Bash,
    Zsh,
}

impl ShellKind {
    pub fn as_str(self) -> &'static str {
        match self {
            ShellKind::Bash => "bash",
            ShellKind::Zsh => "zsh",
        }
    }

    fn rc_filename(self) -> &'static str {
        match self {
            ShellKind::Bash => ".bashrc",
            ShellKind::Zsh => ".zshrc",
        }
    }

    fn wrapper_block(self) -> String {
        let body = match self {
            ShellKind::Bash => r#"b() {
  if [[ ! -t 0 || ! -t 1 ]]; then
    command b "$@"
    return $?
  fi
  for arg in "$@"; do
    case "$arg" in
      setup|init|-h|--help|--json|--plain)
        command b "$@"
        return $?
        ;;
    esac
  done
  local __b_tmp
  __b_tmp="$(mktemp)" || return 1
  command b --output-file "$__b_tmp" "$@" || {
    local __b_status=$?
    rm -f "$__b_tmp"
    return $__b_status
  }
  local cmd
  cmd="$(cat "$__b_tmp")"
  rm -f "$__b_tmp"
  [[ -z "$cmd" ]] && return 1
  printf '%s\n' "$cmd"
  history -s "$cmd"
  eval "$cmd"
}"#,
            ShellKind::Zsh => r#"b() {
  if [[ ! -t 0 || ! -t 1 ]]; then
    command b "$@"
    return $?
  fi
  for arg in "$@"; do
    case "$arg" in
      setup|init|-h|--help|--json|--plain)
        command b "$@"
        return $?
        ;;
    esac
  done
  local __b_tmp
  __b_tmp="$(mktemp)" || return 1
  command b --output-file "$__b_tmp" "$@" || {
    local __b_status=$?
    rm -f "$__b_tmp"
    return $__b_status
  }
  local cmd
  cmd="$(cat "$__b_tmp")"
  rm -f "$__b_tmp"
  [[ -z "$cmd" ]] && return 1
  printf '%s\n' "$cmd"
  print -s -- "$cmd"
  eval "$cmd"
}"#,
        };
        format!("{BEGIN_MARKER}\n{body}\n{END_MARKER}\n")
    }
}

pub struct InstallResult {
    pub shell: ShellKind,
    pub rc_path: PathBuf,
    pub updated: bool,
}

pub fn detect_shell_from_env() -> Option<ShellKind> {
    let shell = std::env::var("SHELL").ok()?;
    let name = Path::new(&shell).file_name()?.to_string_lossy();
    match name.as_ref() {
        "zsh" => Some(ShellKind::Zsh),
        "bash" => Some(ShellKind::Bash),
        _ => None,
    }
}

pub fn parse_shell_name(input: &str) -> Option<ShellKind> {
    match input.trim().to_ascii_lowercase().as_str() {
        "zsh" => Some(ShellKind::Zsh),
        "bash" => Some(ShellKind::Bash),
        _ => None,
    }
}

pub fn install(shell: ShellKind) -> Result<InstallResult> {
    let home =
        dirs::home_dir().ok_or_else(|| AppError::from("unable to locate home directory"))?;
    let rc_path = home.join(shell.rc_filename());
    let block = shell.wrapper_block();

    let existing = fs::read_to_string(&rc_path).unwrap_or_default();
    let (new_content, updated) = upsert_block(&existing, &block);
    if updated {
        fs::write(&rc_path, new_content)?;
    }

    Ok(InstallResult {
        shell,
        rc_path,
        updated,
    })
}

fn upsert_block(existing: &str, block: &str) -> (String, bool) {
    if let Some(start) = existing.find(BEGIN_MARKER) {
        if let Some(end_rel) = existing[start..].find(END_MARKER) {
            let end = start + end_rel + END_MARKER.len();
            let mut out = String::new();
            out.push_str(&existing[..start]);
            if !out.ends_with('\n') && !out.is_empty() {
                out.push('\n');
            }
            out.push_str(block);
            let trailing = existing[end..].trim_start_matches('\n');
            if !trailing.is_empty() {
                out.push('\n');
                out.push_str(trailing);
                if !out.ends_with('\n') {
                    out.push('\n');
                }
            }
            let changed = out != existing;
            return (out, changed);
        }
    }

    let mut out = existing.to_string();
    if !out.is_empty() && !out.ends_with('\n') {
        out.push('\n');
    }
    if !out.is_empty() {
        out.push('\n');
    }
    out.push_str(block);
    (out, true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrapper_block_passes_through_control_flags() {
        let z = ShellKind::Zsh.wrapper_block();
        assert!(z.contains("setup|init|-h|--help|--json|--plain"));
        assert!(z.contains("command b --output-file"));
        assert!(z.contains("if [[ ! -t 0 || ! -t 1 ]]; then"));
    }

    #[test]
    fn upsert_block_is_idempotent() {
        let block = ShellKind::Bash.wrapper_block();
        let (first, changed1) = upsert_block("", &block);
        assert!(changed1);
        let (second, changed2) = upsert_block(&first, &block);
        assert!(!changed2);
        assert_eq!(first, second);
    }
}
