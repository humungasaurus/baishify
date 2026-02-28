# Distribution Plan

## Recommended Channels

1. Homebrew tap (primary for macOS users)
2. `cargo install baishify` from crates.io (Rust users)
3. Direct binaries from GitHub Releases (Linux/macOS)

## Homebrew Strategy

Use a dedicated tap repo (`humungasaurus/homebrew-tap`) with formula name `baishify`.

Install command for users:

```bash
brew tap humungasaurus/tap
brew install baishify
```

The formula should install the binary as `b`.

### Formula template

The formula uses prebuilt binaries from GitHub Releases (no Rust toolchain required on the user's machine):

```ruby
class Baishify < Formula
  desc "Prompt-to-bash CLI"
  homepage "https://github.com/humungasaurus/baishify"
  license "MIT"
  version "VERSION"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/humungasaurus/baishify/releases/download/VERSION/baishify-VERSION-aarch64-apple-darwin.tar.gz"
      sha256 "REPLACE_WITH_SHA256"
    else
      url "https://github.com/humungasaurus/baishify/releases/download/VERSION/baishify-VERSION-x86_64-apple-darwin.tar.gz"
      sha256 "REPLACE_WITH_SHA256"
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "https://github.com/humungasaurus/baishify/releases/download/VERSION/baishify-VERSION-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "REPLACE_WITH_SHA256"
    else
      url "https://github.com/humungasaurus/baishify/releases/download/VERSION/baishify-VERSION-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "REPLACE_WITH_SHA256"
    end
  end

  def install
    bin.install "b"
  end

  test do
    assert_match "baishify", shell_output("#{bin}/b --help")
  end
end
```

## Release Automation

Handled by `.github/workflows/release.yml`, triggered on `v*` tag pushes:

1. **Build job** — cross-compiles for 4 targets:
   - `aarch64-apple-darwin` (macOS ARM, native on `macos-latest`)
   - `x86_64-apple-darwin` (macOS Intel, native on `macos-13`)
   - `x86_64-unknown-linux-gnu` (Linux x86, native on `ubuntu-latest`)
   - `aarch64-unknown-linux-gnu` (Linux ARM, via `cross` on `ubuntu-latest`)
2. **Release job** — creates a GitHub Release with all 4 tarballs attached.
3. **Update-tap job** — downloads assets, computes SHA256s, updates the Homebrew formula in `humungasaurus/homebrew-tap`, and pushes.

### Required secrets

- `TAP_GITHUB_TOKEN` — fine-grained PAT scoped to `humungasaurus/homebrew-tap` with Contents read/write permission.
