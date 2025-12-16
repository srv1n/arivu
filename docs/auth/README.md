# MCP-Compliant Authentication in Arivu

This repo runs MCP servers (stdio today; HTTP optional later). We align with the Model Context Protocol (MCP) authorization guidance:

- HTTP servers use the MCP Authorization protocol: return `401` with `WWW-Authenticate` challenges (Bearer/OAuth), and accept `Authorization: Bearer <token>` on subsequent requests.
- stdio/stdio+pipes servers don't use HTTP challenges; credentials are provided via tools/config/env. We expose first-class MCP tools to set and test credentials.

## What's implemented now (stdio)

- Generic auth tools per provider (added by the MCP server):
  - `auth/<provider>/get_schema` → JSON schema for the provider's credentials
  - `auth/<provider>/set` → set credentials (tokens, OAuth results, username/password or API token)
  - `auth/<provider>/test` → smoke test
- Each connector also retains `config_schema` for UI generation and `test_auth`.

Example (Slack token):

```bash
# Interactive setup (recommended)
arivu setup slack

# Or set directly
arivu config set slack --value "xoxb-..."

# Test authentication
arivu config test slack
```

## Supported methods by provider (MVP)

- Slack: token (xoxb/xoxp)
- GitHub: token (fine-grained PAT)
- Atlassian: basic (email + API token)
- Microsoft Graph: device code OAuth flow (tools planned to be migrated under `auth/microsoft/...`)

LLM provider web search (built-in tools):

- OpenAI search: `OPENAI_API_KEY` (Bearer). Optional: `OPENAI_ORG_ID`, `OPENAI_PROJECT_ID`.
- Anthropic search: `ANTHROPIC_API_KEY` (`x-api-key`), requires `anthropic-version` header (we set `2023-06-01`).
- Gemini search: `GEMINI_API_KEY` or `GOOGLE_API_KEY` (query param or `x-goog-api-key`).
- Perplexity search: `PPLX_API_KEY` (Bearer).
- xAI search: `XAI_API_KEY` (Bearer).

Third-party SERP/crawl providers:

- Exa search: `EXA_API_KEY` (`x-api-key`).
- Firecrawl search: `FIRECRAWL_API_KEY` (Bearer).
- Serper search: `SERPER_API_KEY` (`X-API-KEY`).
- Tavily search: `TAVILY_API_KEY` (sent in request body as `api_key`).
- SerpAPI search: `SERPAPI_API_KEY` (sent as `api_key` query param).

Each connector accepts an optional `model` default in config.

## HTTP transport (future)

If you enable an HTTP MCP transport, implement:

- `401 Unauthorized` with `WWW-Authenticate` challenge indicating supported methods (e.g., `Bearer realm="arivu", scope="connector:slack"`).
- Accept `Authorization: Bearer <token>` on the JSON-RPC endpoint.
- Optionally implement an OAuth redirect flow for app-hosted deployments.

## Security notes

- Secrets never logged; stored via `FileAuthStore` only when explicitly set (future toggle to enforce in-memory-only).
- Fields marked `Secret` in schemas use `format: password` for UIs.

## Next

- Move Microsoft Graph device code tools under `auth/microsoft/*`.
- Optional: add backoff-aware OAuth exchanges and token refresh helpers.
