# Distribution Plan

## Recommended Channels

1. Homebrew tap (primary for macOS users)
2. `cargo install baishify` from crates.io (Rust users)
3. Direct binaries from GitHub Releases (Linux/macOS)

## Homebrew Strategy

Use a dedicated tap repo (for example `danielhostetler/homebrew-tap`) with formula name `baishify`.

Install command for users:

```bash
brew tap danielhostetler/tap
brew install baishify
```

The formula should install the binary as `b`.

### Formula template

```ruby
class Baishify < Formula
  desc "Prompt-to-bash CLI"
  homepage "https://github.com/danielhostetler/baishify"
  url "https://github.com/danielhostetler/baishify/archive/refs/tags/v0.1.0.tar.gz"
  sha256 "REPLACE_WITH_SHA256"
  license "MIT"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args
  end

  test do
    output = shell_output("#{bin}/b --help")
    assert_match "prompt to bash command", output
  end
end
```

Note: many maintainers prefer release artifacts over source builds in formulae for faster installs.

## Release Automation (suggested)

1. Tag release (`vX.Y.Z`).
2. CI builds binaries for:
   - `aarch64-apple-darwin`
   - `x86_64-apple-darwin`
   - `x86_64-unknown-linux-gnu`
   - `aarch64-unknown-linux-gnu`
3. Publish checksums.
4. Open automated PR to tap repo updating formula URL + SHA256.

Tools commonly used:
- `goreleaser` (works for Rust via custom builds)
- `cargo-dist`
- `release-plz`
