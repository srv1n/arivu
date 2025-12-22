# Changelog

All notable changes to Arivu will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

### Changed

## [0.2.12] - 2025-12-22

### Changed
- Fixed `arivu_core` examples to compile again after MCP type refactors (removed stale `async_mcp` usage and aligned examples with `rmcp` request/response types).
- Cleaned up example-only warnings so `cargo test --workspace --all-features` stays green.

## [0.2.11] - 2025-12-22

### Added
- YouTube: `youtube/list` to list recent uploads from a channel/playlist (with `published_within_days` / `published_after`) and `youtube/resolve_channel` to reduce ambiguity when selecting an “official” channel.
- Docs: downstream integration + migration guide (`docs/integrations/DOWNSTREAM_UPGRADE.md`) and updated connector docs to match canonical tool surfaces.

### Changed
- Standardized “tools for agents” surfaces across multiple connectors (canonical `search`/`get`/`list` where applicable) while keeping legacy tool names callable for compatibility.
- Fixed YouTube CLI regressions and aligned YouTube/Reddit/arXiv/PubMed resolver routing to canonical tools.
- Reduced Reddit tool ambiguity by consolidating into `reddit/list`, `reddit/search`, `reddit/get` (with explicit `sort`/`time` parameters on search).

## [0.2.10] - 2025-12-21

### Changed
- Hacker News `top` now uses the official Firebase `topstories` ordering for front-page parity.
- Removed the confusing `arivu call` CLI subcommand; use connector subcommands (e.g., `arivu reddit top ...`) and `arivu tools <connector>`.
- Fixed Reddit CLI wrappers to call the correct underlying tools; `reddit top` now supports `--time` (hour/day/week/month/year/all).

## [0.2.9] - 2025-12-21

### Changed
- Bumped toml dependency to 0.9.10 for wider downstream compatibility

## [0.2.8] - 2025-12-20

### Added
- LLM quick sheet for tool selection (`docs/llms.txt`)
- Task → Tool mappings across connector docs for MCP usage
- Documentation sections for additional connectors (bioRxiv/medRxiv, Google Scholar, RSS, LocalFS, Spotlight, Discord)

### Changed
- Tightened MCP tool descriptions for LLM-friendly selection across connectors
- Updated MCP README tool naming guidance and auth notes
- Clarified explicit user-permission requirements for personal-data connectors

### Added
- Interactive setup wizard (`arivu setup`)
- Comprehensive connector documentation
- GitHub Actions release workflow for all platforms
- Homebrew formula and install script
- Cross-platform binary releases (macOS, Linux, Windows)

### Changed
- Improved CLI help messages and examples
- Updated installation documentation

## [0.1.0] - 2024-XX-XX

### Added

#### Core
- Model Context Protocol (MCP) compliant connector architecture
- Unified `Connector` trait for standardized data source integration
- Thread-safe `ProviderRegistry` for connector management
- Schema-driven authentication system
- Structured error handling with `ConnectorError`

#### CLI (`arivu`)
- `list` - List available connectors
- `search` - Search across connectors
- `get` - Fetch specific content by ID
- `tools` - Show connector tools and parameters
- `config` - Manage authentication
- `setup` - Interactive configuration wizard
- `call` - Call connector tools directly
- Multiple output formats: pretty, JSON, YAML, Markdown

#### MCP Server
- Full MCP protocol compliance
- JSON-RPC over stdio transport
- Tool aggregation across all connectors

#### Connectors

**Media & Social**
- YouTube - Video details, transcripts, chapters, search
- Reddit - Posts, comments, subreddits, user profiles
- X (Twitter) - Tweets, profiles, timelines, DMs
- Hacker News - Stories, comments, search

**Academic & Research**
- arXiv - Paper search and PDF retrieval
- PubMed - Medical literature
- Semantic Scholar - Academic papers with citations
- SciHub - Research paper access

**AI-Powered Search**
- OpenAI Web Search (Responses API)
- Anthropic/Claude Web Search
- Gemini Search (Google)
- Perplexity Search
- Tavily, Exa, Firecrawl
- X.AI Grok Search

**Productivity**
- Slack - Channels, messages, files, search
- GitHub - Issues, PRs, code search, files
- Atlassian - Jira issues, Confluence pages

**Google Workspace**
- Gmail - Messages and search
- Calendar - Events and scheduling
- Drive - Files and folders
- People/Contacts

**Microsoft 365**
- Outlook, Teams, OneDrive via Microsoft Graph

**Web Scraping**
- Generic web scraper

**Reference**
- Wikipedia - Articles, search, geo-search

### Security
- Secure credential storage in user config directory
- Browser cookie extraction for authenticated services
- No credentials stored in code or logs
- Environment variable support for all secrets

---

[Unreleased]: https://github.com/srv1n/arivu/compare/v0.2.12...HEAD
[0.2.12]: https://github.com/srv1n/arivu/releases/tag/v0.2.12
[0.2.11]: https://github.com/srv1n/arivu/releases/tag/v0.2.11
[0.2.10]: https://github.com/srv1n/arivu/releases/tag/v0.2.10
[0.2.9]: https://github.com/srv1n/arivu/releases/tag/v0.2.9
[0.2.8]: https://github.com/srv1n/arivu/releases/tag/v0.2.8
[0.1.0]: https://github.com/srv1n/arivu/releases/tag/v0.1.0
