# Feature Flags and Platform Support

This crate compiles with a minimal core by default. You opt‑in to connectors and extras via Cargo features. This keeps builds small and enables cross‑platform targets (Windows, Linux, macOS, iOS, Android) without platform‑specific breakage.

## Core Feature Sets

- all-connectors: Convenience bundle that enables most connectors (use only when size isn’t a concern).
- examples: Enable example binaries under `rzn_datasourcer_core/examples/*`.
- logging (suggested): Use tracing/tracing-subscriber in CLI/MCP (planned umbrella; currently enabled by default in those crates).

## Connectors (enable individually)

Academic & web:
- arxiv, pubmed, semantic-scholar, wikipedia, web, reddit, hackernews, youtube, x (x-twitter), scihub, imap, github, slack, atlassian

Productivity:
- microsoft-graph, google-drive, google-gmail, google-calendar, google-people

LLM provider web search:
- openai-search, anthropic-search, gemini-search, perplexity-search, xai-search

SERP / crawl APIs:
- exa-search, firecrawl-search, serper-search, serpapi-search, tavily-search

Platform specific:
- macos-automation (macOS only): Adds AppleScript/JXA via osakit. Safe on non‑mac targets (dependency and code are target‑gated), but functionality is a no‑op.
- browser-cookies: Enables reading cookies from installed browsers (uses rookie + publicsuffix).

## Build Recipes

- Minimal core:
  - `cargo build -p rzn_datasourcer_core`
- CLI with a couple of connectors:
  - `cargo build -p rzn_datasourcer_cli --features "openai-search,serpapi-search"`
- MCP server with productivity only:
  - `cargo build -p rzn_datasourcer_mcp --features "microsoft-graph,google-drive"`
- macOS automation (macOS target):
  - `cargo build -p rzn_datasourcer_core --features macos-automation`

## Cross‑Platform Notes (Tauri)

- macOS‑specific code is behind `#[cfg(target_os = "macos")]` and a feature flag; non‑mac targets compile clean.
- Avoid `all-connectors` in mobile builds; pick only what you need.
- Network/HTTP features (reqwest with `rustls-tls`) are already set for portable TLS.

## Auth and Environment Variables

- Provider auth is documented in `docs/auth/README.md` (OpenAI/Anthropic/Gemini/Perplexity/xAI, Exa/Firecrawl/Serper/Tavily/SerpAPI, etc.).
