# baishify

`baishify` is a terminal-native CLI that turns a text prompt into a single bash command.

It is designed for one job: fast prompt-to-command generation with a fun but transparent interactive UX.

## Install

```bash
cargo install --path .
```

This installs the executable as `b`.

## Quick Start

```bash
b setup
b "find all jpg files larger than 5MB"
```

Pipe mode also works:

```bash
echo "show top 10 biggest files recursively" | b
```

## Onboarding

Run `b setup` to configure provider, model, base URL, and key.

The setup flow:
1. Detects existing provider keys from your environment.
2. Lets you choose a provider.
3. Prompts for key only if needed.
4. Lets you pick a model from a searchable list.
5. Tests the provider.
6. Saves config in `~/.config/baishify/config.toml`.
7. Offers one-click shell integration install (recommended).

## Shell Integration (recommended)

To execute commands in your current shell session and get history-up-arrow behavior:

```bash
b init
```

Or explicitly:

```bash
b init zsh
b init bash
```

This installs a small shell function wrapper into your rc file (`~/.zshrc` or `~/.bashrc`) and is idempotent.

## Providers

Supported providers:
- OpenAI
- Anthropic
- OpenRouter
- Vercel AI Gateway

Recognized env vars:
- `OPENAI_API_KEY`
- `ANTHROPIC_API_KEY`
- `OPENROUTER_API_KEY`
- `VERCEL_AI_GATEWAY_API_KEY` (or `AI_GATEWAY_API_KEY`)

Optional:
- `BAISHIFY_PROVIDER`
- `BAISHIFY_MODEL`
- `BAISHIFY_BASE_URL`

Config precedence:
1. CLI flags
2. Environment variables
3. Config file defaults

## UX Modes

`b` defaults to interactive mode on a TTY:
- Immediate loading feedback (phase-based states)
- Command preview
- Actions: accept, regenerate, explain, copy, quit

In non-TTY/script mode, it prints only the command by default.

## Flags

```text
--provider <name>    openai | anthropic | openrouter | vercel
--model <name>       Override model
--base-url <url>     Override API base URL
--api-key <key>      Override API key
-e, --explain        Include explanation
--json               JSON output
--plain              Disable interactive rendering
--no-fun             Disable playful copy
```

## Safety

- `b` does not auto-execute commands.
- It returns one generated command and a safety label (`safe`, `caution`, `risky`).
- You choose whether to run the command.
