# Arivu - Installation Guide

## Quick Install

### One-Line Install (macOS/Linux)

```bash
curl -fsSL https://raw.githubusercontent.com/srv1n/arivu/main/packaging/scripts/install.sh | bash
```

### Homebrew (macOS/Linux)

```bash
brew tap srv1n/tap
brew install arivu
```

### Verify Installation

```bash
arivu --version
arivu setup        # Interactive first-time setup
```

---

## Platform-Specific Installation

### macOS

**Apple Silicon (M1/M2/M3):**
```bash
curl -LO https://github.com/srv1n/arivu/releases/latest/download/arivu-v0.1.0-aarch64-apple-darwin.tar.gz
tar -xzf arivu-v0.1.0-aarch64-apple-darwin.tar.gz
sudo mv arivu /usr/local/bin/
```

**Intel Mac:**
```bash
curl -LO https://github.com/srv1n/arivu/releases/latest/download/arivu-v0.1.0-x86_64-apple-darwin.tar.gz
tar -xzf arivu-v0.1.0-x86_64-apple-darwin.tar.gz
sudo mv arivu /usr/local/bin/
```

> **Note:** If macOS blocks the binary, run: `xattr -d com.apple.quarantine /usr/local/bin/arivu`

### Linux

**x86_64 (glibc):**
```bash
curl -LO https://github.com/srv1n/arivu/releases/latest/download/arivu-v0.1.0-x86_64-unknown-linux-gnu.tar.gz
tar -xzf arivu-v0.1.0-x86_64-unknown-linux-gnu.tar.gz
sudo mv arivu /usr/local/bin/
```

**x86_64 (musl/Alpine):**
```bash
curl -LO https://github.com/srv1n/arivu/releases/latest/download/arivu-v0.1.0-x86_64-unknown-linux-musl.tar.gz
tar -xzf arivu-v0.1.0-x86_64-unknown-linux-musl.tar.gz
sudo mv arivu /usr/local/bin/
```

**ARM64:**
```bash
curl -LO https://github.com/srv1n/arivu/releases/latest/download/arivu-v0.1.0-aarch64-unknown-linux-gnu.tar.gz
tar -xzf arivu-v0.1.0-aarch64-unknown-linux-gnu.tar.gz
sudo mv arivu /usr/local/bin/
```

### Windows

**PowerShell:**
```powershell
Invoke-WebRequest -Uri "https://github.com/srv1n/arivu/releases/latest/download/arivu-v0.1.0-x86_64-pc-windows-msvc.zip" -OutFile "arivu.zip"
Expand-Archive -Path "arivu.zip" -DestinationPath "$env:USERPROFILE\bin"
# Add to PATH: [Environment]::SetEnvironmentVariable("PATH", "$env:PATH;$env:USERPROFILE\bin", "User")
```

---

## Build from Source

### Prerequisites
- Rust 1.70+ ([rustup.rs](https://rustup.rs))

### Build & Install

```bash
git clone https://github.com/srv1n/arivu.git
cd arivu
cargo build --release -p arivu_cli
sudo cp target/release/arivu /usr/local/bin/
```

### Feature Flags

```bash
# Minimal (no connectors)
cargo build --release -p arivu_cli --no-default-features

# All connectors
cargo build --release -p arivu_cli --features full

# Specific connectors only
cargo build --release -p arivu_cli --features "youtube,hackernews,arxiv"

# With LLM search providers
cargo build --release -p arivu_cli --features "openai-search,anthropic-search,gemini-search"
```

---

## First-Time Setup

After installation, run the interactive setup wizard:

```bash
arivu setup
```

This guides you through:
1. Selecting connectors to configure
2. Entering API keys and credentials
3. Testing authentication

### Credential Storage

Credentials are stored securely at:
- **macOS/Linux:** `~/.config/arivu/auth.json`
- **Windows:** `%APPDATA%\arivu\auth.json`

### Manual Configuration

**Via CLI:**
```bash
arivu config set slack --value "xoxb-your-token"
arivu config set x --browser chrome          # Extract cookies from browser
arivu config test slack                       # Verify authentication
```

**Via Environment Variables:**
```bash
export REDDIT_CLIENT_ID="your_id"
export REDDIT_CLIENT_SECRET="your_secret"
export SLACK_TOKEN="xoxb-your-token"
```

---

## Updating

```bash
# Homebrew
brew upgrade arivu

# One-line installer
curl -fsSL https://raw.githubusercontent.com/srv1n/arivu/main/packaging/scripts/install.sh | bash

# From source
cd arivu && git pull && cargo build --release -p arivu_cli
```

## Uninstalling

```bash
# Homebrew
brew uninstall arivu && brew untap srv1n/tap

# Manual
sudo rm /usr/local/bin/arivu
rm -rf ~/.config/arivu  # Remove config (optional)
```

---

## Troubleshooting

### Command not found
```bash
# Add to PATH (bash)
echo 'export PATH="/usr/local/bin:$PATH"' >> ~/.bashrc && source ~/.bashrc

# Add to PATH (zsh)
echo 'export PATH="/usr/local/bin:$PATH"' >> ~/.zshrc && source ~/.zshrc
```

### Permission denied
```bash
chmod +x /usr/local/bin/arivu
```

### macOS Gatekeeper
```bash
xattr -d com.apple.quarantine /usr/local/bin/arivu
```

### Verify binary integrity
```bash
curl -LO https://github.com/srv1n/arivu/releases/latest/download/checksums.txt
sha256sum -c checksums.txt  # Linux
shasum -a 256 -c checksums.txt  # macOS
```

---

## Getting Help

```bash
arivu --help           # General help
arivu <command> --help # Command help
arivu connectors       # List available connectors with details
arivu tools youtube    # Show YouTube connector tools & parameters
```

Report issues: https://github.com/srv1n/arivu/issues
