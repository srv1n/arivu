# Arivu

[![CI](https://github.com/srv1n/arivu/actions/workflows/ci.yml/badge.svg)](https://github.com/srv1n/arivu/actions/workflows/ci.yml)
[![Release](https://github.com/srv1n/arivu/actions/workflows/release.yml/badge.svg)](https://github.com/srv1n/arivu/releases)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE-MIT)

A Rust library and CLI for querying external data sources through a unified interface. Access academic papers, social platforms, enterprise tools, and web content from one tool.

Arivu wraps existing, well-maintained Rust crates and APIs into a consistent connector interface. Rather than reinventing the wheel, it provides a unified way to access data from sources that already have excellent libraries or public APIs.

## Connectors

Arivu provides connectors for 25+ data sources. Each connector exposes a consistent interface for searching, fetching, and listing content.

### No Authentication Required

These connectors work immediately after installation.

| Connector | Description |
|-----------|-------------|
| <img src="https://www.google.com/s2/favicons?domain=arxiv.org&sz=16" width="16" height="16" /> ArXiv | Search and retrieve academic preprints |
| <img src="https://www.google.com/s2/favicons?domain=biorxiv.org&sz=16" width="16" height="16" /> bioRxiv/medRxiv | Biology and medicine preprints |
| <img src="https://www.google.com/s2/favicons?domain=pubmed.ncbi.nlm.nih.gov&sz=16" width="16" height="16" /> PubMed | Search biomedical and life sciences literature |
| <img src="https://www.google.com/s2/favicons?domain=semanticscholar.org&sz=16" width="16" height="16" /> Semantic Scholar | Academic paper search, citations, references |
| <img src="https://www.google.com/s2/favicons?domain=scholar.google.com&sz=16" width="16" height="16" /> Google Scholar | Academic paper search |
| <img src="https://www.google.com/s2/favicons?domain=wikipedia.org&sz=16" width="16" height="16" /> Wikipedia | Article content and search |
| <img src="https://www.google.com/s2/favicons?domain=news.ycombinator.com&sz=16" width="16" height="16" /> Hacker News | Stories, comments, user profiles |
| <img src="https://www.google.com/s2/favicons?domain=youtube.com&sz=16" width="16" height="16" /> YouTube | Video metadata, transcripts, search |
| <img src="https://www.google.com/s2/favicons?domain=rss.com&sz=16" width="16" height="16" /> RSS | Fetch and parse RSS/Atom feeds |
| <img src="https://www.google.com/s2/favicons?domain=w3.org&sz=16" width="16" height="16" /> Web Scraper | HTML content extraction with CSS selectors |

### Optional Authentication

These connectors work without credentials but offer additional functionality when authenticated.

| Connector | Without Auth | With Auth |
|-----------|--------------|-----------|
| <img src="https://www.google.com/s2/favicons?domain=reddit.com&sz=16" width="16" height="16" /> Reddit | Public subreddit browsing | Post to subreddits, access private content |
| <img src="https://www.google.com/s2/favicons?domain=github.com&sz=16" width="16" height="16" /> GitHub | Public repo search | Private repos, higher rate limits |
| <img src="https://www.google.com/s2/favicons?domain=semanticscholar.org&sz=16" width="16" height="16" /> Semantic Scholar | Basic search | Higher rate limits |

### Authentication Required

| Connector | Auth Type | Description |
|-----------|-----------|-------------|
| <img src="https://www.google.com/s2/favicons?domain=slack.com&sz=16" width="16" height="16" /> Slack | Bot token | Channels, messages, users |
| <img src="https://www.google.com/s2/favicons?domain=discord.com&sz=16" width="16" height="16" /> Discord | Bot token | Servers, channels, messages |
| <img src="https://www.google.com/s2/favicons?domain=atlassian.com&sz=16" width="16" height="16" /> Atlassian | API token | Jira issues, Confluence pages |
| <img src="https://www.google.com/s2/favicons?domain=drive.google.com&sz=16" width="16" height="16" /> Google Drive | OAuth2 | Files and folders |
| <img src="https://www.google.com/s2/favicons?domain=gmail.com&sz=16" width="16" height="16" /> Gmail | OAuth2 | Email access |
| <img src="https://www.google.com/s2/favicons?domain=calendar.google.com&sz=16" width="16" height="16" /> Google Calendar | OAuth2 | Calendar events |
| <img src="https://www.google.com/s2/favicons?domain=contacts.google.com&sz=16" width="16" height="16" /> Google Contacts | OAuth2 | People/contacts |
| <img src="https://www.google.com/s2/favicons?domain=microsoft.com&sz=16" width="16" height="16" /> Microsoft Graph | OAuth2 | OneDrive, Outlook, Calendar |
| <img src="https://www.google.com/s2/favicons?domain=gmail.com&sz=16" width="16" height="16" /> IMAP | Server credentials | Email retrieval |
| <img src="https://www.google.com/s2/favicons?domain=x.com&sz=16" width="16" height="16" /> X (Twitter) | Credentials/Cookies | Tweets, profiles, search |

### Search Providers

These connectors query AI-powered or traditional search APIs.

| Connector | Auth Type |
|-----------|-----------|
| <img src="https://www.google.com/s2/favicons?domain=perplexity.ai&sz=16" width="16" height="16" /> Perplexity | API Key |
| <img src="https://www.google.com/s2/favicons?domain=exa.ai&sz=16" width="16" height="16" /> Exa | API Key |
| <img src="https://www.google.com/s2/favicons?domain=tavily.com&sz=16" width="16" height="16" /> Tavily | API Key |
| <img src="https://www.google.com/s2/favicons?domain=serpapi.com&sz=16" width="16" height="16" /> SerpApi | API Key |
| <img src="https://www.google.com/s2/favicons?domain=serper.dev&sz=16" width="16" height="16" /> Serper | API Key |
| <img src="https://www.google.com/s2/favicons?domain=firecrawl.dev&sz=16" width="16" height="16" /> Firecrawl | API Key |
| <img src="https://www.google.com/s2/favicons?domain=anthropic.com&sz=16" width="16" height="16" /> Anthropic | API Key |
| <img src="https://www.google.com/s2/favicons?domain=openai.com&sz=16" width="16" height="16" /> OpenAI | API Key |
| <img src="https://www.google.com/s2/favicons?domain=deepmind.google&sz=16" width="16" height="16" /> Gemini | API Key |
| <img src="https://www.google.com/s2/favicons?domain=parallel.ai&sz=16" width="16" height="16" /> Parallel AI | API Key |
| <img src="https://www.google.com/s2/favicons?domain=x.ai&sz=16" width="16" height="16" /> xAI | API Key |

### macOS Native

| Connector | Description |
|-----------|-------------|
| <img src="https://www.google.com/s2/favicons?domain=apple.com&sz=16" width="16" height="16" /> macOS Automation | Control Mail, Calendar, Safari via JXA (requires permissions) |
| <img src="https://www.google.com/s2/favicons?domain=apple.com&sz=16" width="16" height="16" /> Spotlight | Search files by content, name, type, or metadata (macOS only) |

## Quick Start

```bash
# Just paste any URL or ID - Arivu figures out what to do
arivu fetch https://arxiv.org/abs/2301.07041
arivu fetch https://news.ycombinator.com/item?id=38500000
arivu fetch https://github.com/rust-lang/rust
arivu fetch PMID:12345678
arivu fetch r/rust

# Or use explicit commands
arivu search pubmed "CRISPR gene therapy"
arivu get hackernews 38500000
```

## Smart Resolver

Arivu includes a pattern-matching system that automatically detects URLs, IDs, and identifiers and routes them to the appropriate connector.

```bash
# YouTube - any URL format or video ID
arivu fetch https://www.youtube.com/watch?v=dQw4w9WgXcQ
arivu fetch https://youtu.be/dQw4w9WgXcQ
arivu fetch dQw4w9WgXcQ

# Academic papers
arivu fetch https://arxiv.org/abs/2301.07041
arivu fetch arXiv:2301.07041
arivu fetch PMID:12345678
arivu fetch 10.1038/nature12373

# GitHub - repos, issues, PRs
arivu fetch https://github.com/rust-lang/rust
arivu fetch https://github.com/rust-lang/rust/issues/12345
arivu fetch rust-lang/rust

# Social platforms
arivu fetch https://news.ycombinator.com/item?id=38500000
arivu fetch hn:38500000
arivu fetch r/rust
arivu fetch @elonmusk

# Local files (macOS Spotlight)
arivu fetch ~/Documents/report.pdf
arivu fetch /Users/me/Downloads/data.csv
arivu fetch "spotlight:machine learning"

# Any URL falls back to web scraper
arivu fetch https://example.com/some/page
```

**Ambiguous inputs** are handled interactively. For example, an 8-digit number could be a Hacker News ID or PubMed ID:

```
$ arivu fetch 12345678

Ambiguous: Input '12345678' matches multiple patterns:

  [1] hackernews → get_post (Hacker News item ID)
  [2] pubmed → get_abstract (PubMed ID)

Select option [1-2]:
```

Use prefixes to avoid ambiguity: `hn:12345678`, `PMID:12345678`, `arXiv:2301.07041`.

**Shell quoting:** URLs containing `?` (like YouTube watch URLs) need to be quoted:

```bash
# This will fail in zsh/bash - the ? is interpreted as a glob
arivu fetch https://www.youtube.com/watch?v=dQw4w9WgXcQ  # Error!

# Quote the URL to make it work
arivu fetch "https://www.youtube.com/watch?v=dQw4w9WgXcQ"  # Works!
```

```bash
# View all supported patterns
arivu formats
```

See [Smart Resolver Documentation](docs/SMART_RESOLVER.md) for the complete list of patterns and library usage.

## Installation

### Pre-built Binaries

Ready-made binaries are available from the [GitHub Releases](https://github.com/srv1n/arivu/releases) page. Choose one of the following methods:

#### Install Script (Recommended)

The install script automatically downloads the correct binary for your platform:

```bash
curl -fsSL https://raw.githubusercontent.com/srv1n/arivu/main/packaging/scripts/install.sh | bash

# Uninstall
rm -f ~/.local/bin/arivu
rm -rf ~/.config/arivu
```

#### Homebrew (macOS/Linux)

```bash
brew tap srv1n/tap
brew install arivu

# Uninstall
brew uninstall arivu
brew untap srv1n/tap
```

### Build from Source

If you prefer to build from source or need to customize which connectors are included:

```bash
git clone https://github.com/srv1n/arivu.git
cd arivu

# Build with all connectors
cargo build --release -p arivu_cli --features full

# Build with specific connectors (smaller binary)
cargo build --release -p arivu_cli --features "arxiv,pubmed,hackernews"

cp target/release/arivu ~/.local/bin/
```

## Federated Search

Search multiple data sources simultaneously with a single command using built-in profiles or custom connector lists.

### Built-in Profiles

| Profile | Connectors | Description |
|---------|------------|-------------|
| `research` | pubmed, arxiv, semantic-scholar, google-scholar | Academic papers |
| `enterprise` | slack, atlassian, github | Work documents and code |
| `social` | reddit, hackernews | Community discussions |
| `code` | github | Code search |
| `web` | perplexity, exa, tavily | AI-powered web search |
| `media` | youtube, wikipedia | Video and reference content |

### Usage

```bash
# Search using a profile
arivu search "CRISPR gene therapy" --profile research
arivu search "kubernetes deployment" -p enterprise
arivu search "rust async" --profile social

# Custom connector list
arivu search "machine learning" -s arxiv,pubmed,hackernews

# Merge modes
arivu search "attention mechanisms" -p research --merge grouped    # Group by source (default)
arivu search "attention mechanisms" -p research --merge interleaved # Interleave results

# Modify profiles on the fly
arivu search "CRISPR" -p research --add wikipedia --exclude pubmed
```

### Output

Results are displayed grouped by source with timing information:

```
Federated Search: CRISPR gene therapy
Profile: research

━━ pubmed (10 results)
   1. CRISPR/Cas9 Immune System as a Tool for Genome Engineering
      https://pubmed.ncbi.nlm.nih.gov/12345678
   2. Advances in therapeutic CRISPR/Cas9 genome editing
      ...

━━ arxiv (10 results)
   1. Investigating the genomic background of CRISPR-Cas genomes
      CRISPR-Cas systems are an adaptive immunity that protects prokaryotes...
   ...

Completed in 1234ms
```

## CLI Usage

### Connector Subcommands (Recommended)

Each connector has its own subcommand with proper CLI flags:

```bash
# Local filesystem - text extraction from PDF, EPUB, DOCX, HTML, code
arivu localfs list-files --path ~/Documents --recursive --extensions pdf,md
arivu localfs extract-text --path ~/paper.pdf
arivu localfs structure --path ~/book.epub
arivu localfs section --path ~/doc.pdf --section page:5
arivu localfs search --path ~/code.rs --query "async fn"

# YouTube
arivu youtube search --query "rust programming" --limit 10
arivu youtube video --id dQw4w9WgXcQ
arivu youtube transcript --id dQw4w9WgXcQ

# Hacker News
arivu hackernews top --limit 20
arivu hackernews search --query "rust" --limit 10
arivu hackernews story --id 38500000

# arXiv
arivu arxiv search --query "transformer architecture" --limit 10
arivu arxiv paper --id 2301.07041

# GitHub
arivu github search-repos --query "rust cli"
arivu github search-code --query "async fn" --repo tokio-rs/tokio
arivu github issues --repo rust-lang/rust --state open

# Reddit
arivu reddit search --query "rust" --subreddit programming
arivu reddit hot --subreddit rust --limit 20

# AI-powered search
arivu perplexity-search search --query "best practices for rust async"
arivu exa search --query "rust async programming" --num-results 10
arivu openai-search search --query "machine learning"
arivu anthropic-search search --query "AI safety"

# Google services (requires OAuth setup)
arivu google-calendar list-events
arivu google-drive list-files --query "project report"
arivu google-gmail search --query "from:boss@company.com"

# Microsoft 365 (requires OAuth setup)
arivu microsoft-graph list-drive-items
arivu microsoft-graph list-mail --filter "isRead eq false"

# Academic research
arivu pubmed search --query "CRISPR gene therapy" --limit 10
arivu semantic-scholar search --query "attention mechanism"
arivu biorxiv search --query "protein folding"

# Use --help on any subcommand for all options
arivu localfs list-files --help
arivu hackernews --help
```

### Generic Commands

```bash
# List available connectors
arivu list

# Show tools for a connector
arivu tools youtube
arivu tools pubmed

# Smart fetch - auto-detects URL/ID type
arivu fetch https://arxiv.org/abs/2301.07041
arivu fetch https://news.ycombinator.com/item?id=38500000
arivu fetch hn:38500000

# Search (single connector)
arivu search arxiv "attention mechanism"
arivu search hackernews "rust" --limit 20

# Get specific content
arivu get hackernews 12345678
arivu get youtube dQw4w9WgXcQ

# Call tools directly (JSON args - for advanced use)
arivu call github search_repos --args '{"query":"language:rust stars:>1000"}'
arivu call slack list_channels

# Output formats
arivu --output json arxiv search --query "llm" | jq '.results[0]'

# Copy output to clipboard
arivu --copy fetch hn:38500000
```

### All Connector Subcommands

| Connector | Aliases | Description |
|-----------|---------|-------------|
| `localfs` | `fs`, `file` | Local filesystem text extraction |
| `youtube` | `yt` | Video metadata, transcripts, search |
| `hackernews` | `hn` | Stories, comments, search |
| `arxiv` | | Academic preprints |
| `github` | `gh` | Repositories, issues, PRs, code |
| `reddit` | | Posts, comments, subreddits |
| `web` | | Web page scraping |
| `wikipedia` | `wiki` | Article search and retrieval |
| `pubmed` | | Medical literature |
| `semantic-scholar` | `scholar` | Academic paper search |
| `slack` | | Workspace messages, channels |
| `discord` | | Servers, channels, messages |
| `x` | `twitter` | Tweets, profiles, search |
| `rss` | | RSS/Atom feed reader |
| `biorxiv` | | Biology/medicine preprints |
| `scihub` | | Paper access |
| `google-calendar` | | Calendar events |
| `google-drive` | | File management |
| `google-gmail` | | Email access |
| `google-people` | | Contacts |
| `google-scholar` | | Academic search |
| `microsoft-graph` | | Microsoft 365 services |
| `atlassian` | | Jira + Confluence |
| `imap` | | Email retrieval |
| `macos` | | macOS automation |
| `spotlight` | | File search (macOS) |
| `openai-search` | | OpenAI web search |
| `anthropic-search` | | Anthropic web search |
| `gemini-search` | | Gemini web search |
| `perplexity-search` | | Perplexity search |
| `xai-search` | | xAI web search |
| `exa` | | Neural search |
| `tavily-search` | | Tavily search |
| `serper-search` | | Serper search |
| `serpapi-search` | | SerpAPI search |
| `firecrawl-search` | | Firecrawl scraping |
| `parallel-search` | | Parallel AI search |

### Response Format

Most connectors support a `response_format` parameter to control output verbosity:

- **`concise`** (default): Returns only essential fields for token efficiency
- **`detailed`**: Returns full metadata including all available fields

```bash
# Concise output (default) - minimal fields, fewer tokens
arivu hackernews top --limit 5

# With JSON args for detailed output
arivu call hackernews get_stories --args '{"story_type":"top","limit":5,"response_format":"detailed"}'
```

This is particularly useful when integrating with AI agents where token usage matters. The concise format reduces response size while preserving the most important information.

### Global Flags

| Flag | Short | Description |
|------|-------|-------------|
| `--copy` | `-c` | Copy output to system clipboard |
| `--output <format>` | | Output format: `pretty`, `json`, `yaml`, `text`, `markdown` |
| `--no-color` | | Disable colored output |
| `--verbose` | `-v` | Verbose output (can be repeated: `-vv`, `-vvv`) |

Note: Global flags must be placed **before** the subcommand:
```bash
arivu --copy fetch https://arxiv.org/abs/2301.07041  # Correct
arivu fetch --copy https://arxiv.org/abs/2301.07041  # Won't work
```

## Configuration

Arivu provides an interactive setup wizard that guides you through configuring each connector.

### Interactive Setup

```bash
# Launch the setup wizard
arivu setup

# Or configure a specific connector
arivu setup slack
```

The wizard will:
1. Show you where to obtain credentials (with clickable URLs)
2. Walk you through each step
3. Securely prompt for tokens (hidden input)
4. Test the connection automatically
5. Save credentials to `~/.config/arivu/auth.json`

Example session:

```
$ arivu setup slack

Setting up Slack
Workspace messages and channels

How to get credentials:
  https://api.slack.com/apps

  1. Create a new app or select an existing one
  2. Go to 'OAuth & Permissions' in the sidebar
  3. Add required scopes: channels:read, channels:history, users:read
  4. Install the app to your workspace
  5. Copy the 'Bot User OAuth Token' (starts with xoxb-)

Configuration options:

  Option 1: Set environment variables:
    export SLACK_BOT_TOKEN="<Bot Token>"

  Option 2: Enter credentials now (stored in ~/.config/arivu/auth.json):

Enter credentials now? [y/N] y
  Bot Token (starts with xoxb-): ****

Saved! Credentials saved for Slack

Testing connection... Success!

You're all set! Try:
  arivu search slack "test query"
```

### Managing Credentials

```bash
# View configured connectors
arivu config show

# Test authentication
arivu config test github

# Remove credentials
arivu config remove slack

# Set credentials directly
arivu config set github --value "ghp_xxxx"
```

### Environment Variables

You can also configure connectors via environment variables:

```bash
export GITHUB_TOKEN="ghp_..."
export SLACK_BOT_TOKEN="xoxb-..."
export REDDIT_CLIENT_ID="..."
export REDDIT_CLIENT_SECRET="..."
export OPENAI_API_KEY="sk-..."
export ANTHROPIC_API_KEY="sk-ant-..."
```

### Browser Cookie Extraction

For services like X (Twitter), Arivu can extract session cookies directly from your browser:

```bash
arivu setup x
```

The wizard will prompt you to select your browser (Chrome, Firefox, Safari, or Brave) and automatically extract cookies after you confirm you're logged in.

### OAuth Setup

For Google and Microsoft services, Arivu supports the OAuth device authorization flow:

```bash
arivu setup google-drive
```

You'll receive a code to enter at a URL in your browser. Once authorized, tokens are saved and refreshed automatically.

## Library Usage

```toml
[dependencies]
arivu_core = { version = "0.1", features = ["arxiv", "pubmed"] }
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
serde_json = "1"
```

```rust
use arivu_core::{build_registry_enabled_only, CallToolRequestParam};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let registry = build_registry_enabled_only().await;
    let pubmed = registry.get_provider("pubmed").unwrap();
    let connector = pubmed.lock().await;

    let request = CallToolRequestParam {
        name: "search_articles".into(),
        arguments: Some(json!({"query": "CRISPR"}).as_object().unwrap().clone()),
    };

    let result = connector.call_tool(request).await?;
    println!("{:?}", result);
    Ok(())
}
```

## MCP Server

Arivu includes a [Model Context Protocol](https://modelcontextprotocol.io/) server for integration with MCP-compatible clients like Claude Desktop.

```bash
cargo build --release -p arivu_mcp --features full
./target/release/arivu_mcp
```

Claude Desktop configuration (`~/Library/Application Support/Claude/claude_desktop_config.json`):

```json
{
  "mcpServers": {
    "arivu": {
      "command": "/path/to/arivu_mcp",
      "env": {
        "GITHUB_TOKEN": "...",
        "SLACK_BOT_TOKEN": "..."
      }
    }
  }
}
```

## Feature Flags

Enable only the connectors you need to reduce binary size:

```toml
# Research
arivu_core = { version = "0.1", features = ["arxiv", "pubmed", "semantic-scholar"] }

# Social
arivu_core = { version = "0.1", features = ["reddit", "hackernews", "youtube"] }

# Enterprise
arivu_core = { version = "0.1", features = ["slack", "github", "atlassian"] }

# Everything
arivu_core = { version = "0.1", features = ["full"] }
```

## Architecture

```
arivu/
├── arivu_core/       # Core library with Connector trait and registry
├── arivu_cli/        # CLI binary
├── arivu_mcp/        # MCP server binary
└── scrapable_derive/ # Proc-macro for HTML parsing
```

All connectors implement a common trait:

```rust
#[async_trait]
pub trait Connector: Send + Sync {
    fn name(&self) -> &'static str;
    async fn list_tools(&self, request: Option<PaginatedRequestParam>) -> Result<ListToolsResult, ConnectorError>;
    async fn call_tool(&self, request: CallToolRequestParam) -> Result<CallToolResult, ConnectorError>;
    // ...
}
```

## Adding a Connector

See the **[Connector Development Guide](docs/CONNECTOR_DEVELOPMENT.md)** for a complete walkthrough including:

- Step-by-step implementation with code examples
- Tool design guidelines and naming conventions
- Authentication patterns
- Smart Resolver and Federated Search integration
- Testing and documentation

Quick overview:
1. Create a module in `arivu_core/src/connectors/`
2. Implement the `Connector` trait
3. Add a feature flag to `Cargo.toml`
4. Register in `build_registry_enabled_only()`

See [CONTRIBUTING.md](CONTRIBUTING.md) for contribution guidelines.

## Documentation

- [Connector Development Guide](docs/CONNECTOR_DEVELOPMENT.md) - Complete guide for adding connectors
- [CLI Usage Guide](arivu_cli/README.md)
- [Smart Resolver](docs/SMART_RESOLVER.md) - URL/ID auto-detection and routing
- [Federated Search](docs/FEDERATED_SEARCH.md) - Multi-source search architecture
- [Authentication Design](docs/AUTH_DESIGN.md) - Auth patterns for downstream apps
- [Installation Options](INSTALLATION.md)
- [Connector Documentation](docs/CONNECTORS.md)
- [Authentication Setup](docs/auth/README.md)

## Acknowledgements

Arivu is built on the shoulders of excellent open-source crates:

| Crate | Used For |
|-------|----------|
| [roux](https://crates.io/crates/roux) | Reddit API client |
| [octocrab](https://crates.io/crates/octocrab) | GitHub API client |
| [wikipedia](https://crates.io/crates/wikipedia) | Wikipedia API client |
| [yt-transcript-rs](https://crates.io/crates/yt-transcript-rs) | YouTube transcript extraction |
| [rusty_ytdl](https://crates.io/crates/rusty_ytdl) | YouTube video metadata |
| [agent-twitter-client](https://crates.io/crates/agent-twitter-client) | X (Twitter) client |
| [rookie](https://crates.io/crates/rookie) | Browser cookie extraction |
| [graph-rs-sdk](https://crates.io/crates/graph-rs-sdk) | Microsoft Graph API |
| [google-drive3](https://crates.io/crates/google-drive3) | Google Drive API |
| [google-gmail1](https://crates.io/crates/google-gmail1) | Gmail API |
| [google-calendar3](https://crates.io/crates/google-calendar3) | Google Calendar API |
| [scraper](https://crates.io/crates/scraper) | HTML parsing |
| [htmd](https://crates.io/crates/htmd) | HTML to Markdown conversion |
| [quick-xml](https://crates.io/crates/quick-xml) | XML parsing for ArXiv/PubMed |
| [rmcp](https://crates.io/crates/rmcp) | Model Context Protocol |
| [reqwest](https://crates.io/crates/reqwest) | HTTP client |
| [tokio](https://crates.io/crates/tokio) | Async runtime |

Thank you to all the maintainers and contributors of these projects.

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))
