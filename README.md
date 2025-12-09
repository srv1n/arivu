# Arivu

[![CI](https://github.com/srv1n/arivu/actions/workflows/ci.yml/badge.svg)](https://github.com/srv1n/arivu/actions/workflows/ci.yml)
[![Release](https://github.com/srv1n/arivu/actions/workflows/release.yml/badge.svg)](https://github.com/srv1n/arivu/releases)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE-MIT)

A Rust library and CLI for accessing external data sources through a unified interface. Query YouTube transcripts, Reddit threads, academic papers, HackerNews, and more from the command line or embed it in your application.

## CLI

```bash
# List available connectors
arivu list

# Search (Shortcuts)
arivu search youtube "rust async runtime"
arivu search hackernews "rust" --limit 20
arivu search arxiv "attention mechanism"

# Get specific content (Shortcuts)
arivu get youtube dQw4w9WgXcQ          # Video with transcript
arivu get hackernews 12345678          # Story with comments

# Show available tools for a connector
arivu youtube                          # Lists all tools (formerly 'arivu tools youtube')

# Call a tool directly (Smart Syntax)
# Maps arguments to tool parameters automatically
arivu slack list_channels
arivu youtube search_videos huberman
arivu hackernews search_stories rust 5

# Advanced / Scripting
arivu github search_repos --args '{"query":"language:rust stars:>1000"}'
arivu search arxiv "transformer" --output json
```

## Installation

### From Source

```bash
git clone https://github.com/srv1n/arivu.git
cd arivu

# Build with all connectors
cargo build --release -p arivu_cli --features full

# Build with specific connectors only (smaller binary)
cargo build --release -p arivu_cli --features "youtube,hackernews,arxiv"

# Install to PATH
cp target/release/arivu ~/.local/bin/
```

### Homebrew (macOS/Linux)

```bash
brew tap srv1n/tap
brew install arivu
```

### Install Script

```bash
curl -fsSL https://raw.githubusercontent.com/srv1n/arivu/main/packaging/scripts/install.sh | bash
```

## Available Connectors

All connectors are behind feature flags. Enable only what you need.

| Feature | Connector | Auth Required | Description |
|---------|-----------|---------------|-------------|
| `youtube` | YouTube | No | Video metadata, transcripts, search |
| `reddit` | Reddit | Client ID/Secret | Posts, comments, subreddit data |
| `hackernews` | HackerNews | No | Stories, comments, user profiles |
| `wikipedia` | Wikipedia | No | Article content, search |
| `arxiv` | ArXiv | No | Academic preprints |
| `pubmed` | PubMed | No | Medical literature |
| `semantic-scholar` | Semantic Scholar | Optional API key | Academic papers, citations |
| `web` | Web Scraper | No | HTML scraping with CSS selectors |
| `x-twitter` | X (Twitter) | Credentials/Cookies | Tweets, profiles, search |
| `scihub` | SciHub | No | Research paper access |
| `imap` | IMAP Email | Server credentials | Email retrieval |
| `slack` | Slack | Bot token | Channels, messages, users |
| `github` | GitHub | Personal access token | Repos, issues, PRs |
| `atlassian` | Atlassian | API token | Jira, Confluence |
| `microsoft-graph` | Microsoft Graph | OAuth2 | OneDrive, Outlook, Calendar |
| `google-drive` | Google Drive | OAuth2 | Files, folders |
| `google-gmail` | Gmail | OAuth2 | Email |
| `google-calendar` | Google Calendar | OAuth2 | Events |

## Library Usage

Add to `Cargo.toml`:

```toml
[dependencies]
arivu_core = { version = "0.1", features = ["youtube", "hackernews"] }
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
serde_json = "1"
```

```rust
use arivu_core::{build_registry_enabled_only, CallToolRequestParam};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let registry = build_registry_enabled_only().await;

    let youtube = registry.get_provider("youtube").unwrap();
    let connector = youtube.lock().await;

    let request = CallToolRequestParam {
        name: "get_video_details".into(),
        arguments: Some(json!({"video_id": "dQw4w9WgXcQ"}).as_object().unwrap().clone()),
    };

    let result = connector.call_tool(request).await?;
    println!("{:?}", result);

    Ok(())
}
```

## Architecture

```
arivu/
├── arivu_core/       # Core library: Connector trait, registry
├── arivu_cli/        # CLI binary (arivu)
├── arivu_mcp/        # Optional MCP server binary
└── scrapable_derive/ # Proc-macro for HTML parsing
```

### Connector Trait

All data sources implement a common async trait:

```rust
#[async_trait]
pub trait Connector: Send + Sync {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;

    async fn list_tools(&self, request: Option<PaginatedRequestParam>) -> Result<ListToolsResult, ConnectorError>;
    async fn call_tool(&self, request: CallToolRequestParam) -> Result<CallToolResult, ConnectorError>;
    async fn list_resources(&self, request: Option<PaginatedRequestParam>) -> Result<ListResourcesResult, ConnectorError>;
    async fn read_resource(&self, request: ReadResourceRequestParam) -> Result<Vec<ResourceContents>, ConnectorError>;

    fn config_schema(&self) -> ConnectorConfigSchema;
    async fn set_auth_details(&mut self, details: AuthDetails) -> Result<(), ConnectorError>;
    async fn test_auth(&self) -> Result<(), ConnectorError>;
}
```

### ProviderRegistry

Thread-safe connector management using `Arc<Mutex<Box<dyn Connector>>>`. Connectors are registered at startup based on enabled Cargo features.

```rust
let registry = build_registry_enabled_only().await;

// Get a specific connector
if let Some(connector) = registry.get_provider("hackernews") {
    let c = connector.lock().await;
    let tools = c.list_tools(None).await?;
}

// List all available connectors
let providers = registry.list_providers();
```

## Authentication

Connectors define their auth requirements via `config_schema()`:

```rust
pub struct ConnectorConfigSchema {
    pub fields: Vec<Field>,
}

pub struct Field {
    pub name: String,
    pub label: String,
    pub field_type: FieldType,  // Text, Secret, Number, Boolean, Select
    pub required: bool,
    pub description: Option<String>,
}
```

Credentials are passed as `HashMap<String, String>` to `set_auth_details()`. The library does not persist credentials.

### CLI Configuration

```bash
# Interactive setup
arivu setup slack

# Set credentials directly
arivu config set github --value "ghp_xxxx"

# Test authentication
arivu config test github
```

### Environment Variables

```bash
export REDDIT_CLIENT_ID="..."
export REDDIT_CLIENT_SECRET="..."
export GITHUB_TOKEN="..."
export SLACK_BOT_TOKEN="xoxb-..."
```

### Browser Cookie Extraction

For connectors like X (Twitter), cookies can be extracted from installed browsers:

```rust
let mut auth = AuthDetails::new();
auth.insert("browser".to_string(), "chrome".to_string());
connector.set_auth_details(auth).await?;
```

Supported: Chrome, Firefox, Safari, Brave.

## Error Handling

All methods return `Result<T, ConnectorError>`:

```rust
pub enum ConnectorError {
    Authentication(String),
    InvalidInput(String),
    InvalidParams(String),
    ResourceNotFound,
    ToolNotFound,
    Timeout(String),
    HttpRequest(reqwest::Error),
    InternalError(String),
}
```

## Feature Flags

Enable only the connectors you need:

```toml
# Minimal
arivu_core = { version = "0.1", features = ["hackernews"] }

# Academic research
arivu_core = { version = "0.1", features = ["arxiv", "pubmed", "semantic-scholar"] }

# Social media
arivu_core = { version = "0.1", features = ["reddit", "x-twitter", "youtube"] }

# Everything
arivu_core = { version = "0.1", features = ["full"] }
```

## Adding a Connector

1. Create module in `arivu_core/src/connectors/`
2. Implement `Connector` trait
3. Add feature flag to `Cargo.toml`
4. Register in `build_registry_enabled_only()`

Key requirements:
- Return data as JSON in `CallToolResult`
- Define tools with JSON Schema for input validation
- Return empty collections for no results (not errors)
- Map external errors to `ConnectorError` variants

## MCP Server

The library includes an optional [Model Context Protocol](https://modelcontextprotocol.io/) server for integration with MCP-compatible clients.

```bash
cargo build --release -p arivu_mcp --features full
./target/release/arivu_mcp
```

The server exposes all enabled connectors via JSON-RPC over stdio. Tool names are prefixed with the connector name (e.g., `youtube/get_video_details`).

Configuration for Claude Desktop (`~/Library/Application Support/Claude/claude_desktop_config.json`):

```json
{
  "mcpServers": {
    "arivu": {
      "command": "/path/to/arivu_mcp",
      "env": {
        "REDDIT_CLIENT_ID": "...",
        "REDDIT_CLIENT_SECRET": "..."
      }
    }
  }
}
```

## Development

```bash
cargo build
cargo test
cargo clippy
cargo fmt
```

## Documentation

- [CLI Usage Guide](arivu_cli/README.md)
- [Installation Options](INSTALLATION.md)
- [Connector Documentation](docs/CONNECTORS.md)
- [Authentication Setup](docs/auth/README.md)

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).
