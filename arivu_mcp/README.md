# RZN DataSourcer MCP Server

This project is now fully MCP (Model Context Protocol) compliant! The MCP server exposes all connectors through a standardized protocol that can be used by any MCP-compatible client.

## Quick Start

### Running the MCP Server

```bash
# Run the MCP server with stdio transport
cargo run --bin mcp_server

# Or build and run
cargo build --release
./target/release/mcp_server
```

The server will listen on stdin/stdout using JSON-RPC over stdio transport.

### Environment Variables

Configure connectors by setting environment variables:

```bash
# Google Search
export GOOGLE_API_KEY="your_api_key"
export GOOGLE_CSE_ID="your_cse_id"

# Reddit
export REDDIT_CLIENT_ID="your_client_id"
export REDDIT_CLIENT_SECRET="your_client_secret"

# Brave Search
export BRAVE_API_KEY="your_api_key"

# Then run the server
cargo run --bin mcp_server
```

## MCP Protocol Compliance

### Supported Capabilities

The server exposes the following MCP capabilities:

- **Tools**: Execute actions through connectors (search, fetch data, etc.)
- **Resources**: Access structured data from various sources
- **Prompts**: Use predefined prompt templates

### Available Connectors

**No Authentication Required:**
- `hackernews` - Search and fetch Hacker News stories
- `wikipedia` - Search and fetch Wikipedia articles
- `arxiv` - Search academic papers on arXiv
- `pubmed` - Search medical literature on PubMed
- `semantic_scholar` - Search academic papers on Semantic Scholar
- `web` - Basic web scraping

**Authentication Required:**
- `reddit` - Reddit API (Client ID/Secret)
- `x` - X (Twitter) API (Credentials or browser cookies)
- `slack` - Slack API (Bot token)
- `github` - GitHub API (Personal access token)
- LLM search connectors (`openai-search`, `anthropic-search`, etc.) - API keys

### MCP Client Usage

#### 1. Initialize Connection

```json
{
  "jsonrpc": "2.0",
  "method": "initialize",
  "params": {
    "protocol_version": "0.1.0",
    "capabilities": {},
    "client_info": {
      "name": "your_client",
      "version": "1.0.0"
    }
  },
  "id": 1
}
```

#### 2. List Available Tools

```json
{
  "jsonrpc": "2.0",
  "method": "tools/list",
  "params": {},
  "id": 2
}
```

#### 3. Call a Tool

```json
{
  "jsonrpc": "2.0",
  "method": "tools/call",
  "params": {
    "name": "wikipedia/search",
    "arguments": {
      "query": "rust programming language",
      "limit": 5
    }
  },
  "id": 3
}
```

#### 4. List Resources

```json
{
  "jsonrpc": "2.0",
  "method": "resources/list",
  "params": {},
  "id": 4
}
```

#### 5. Read Resource

```json
{
  "jsonrpc": "2.0",
  "method": "resources/read",
  "params": {
    "uri": "wikipedia://article/Rust_(programming_language)"
  },
  "id": 5
}
```

### Tool Naming Convention

Tools are prefixed with their connector name to avoid conflicts:
- `hackernews/search` - Search Hacker News
- `wikipedia/search` - Search Wikipedia
- `wikipedia/get_article` - Get specific Wikipedia article
- `reddit/search_posts` - Search Reddit posts
- `youtube/get_video_details` - Get YouTube video with transcript

## Example Client

See `examples/mcp_client_example.rs` for a complete example of how to interact with the MCP server programmatically.

```bash
# Run the example client
cargo run --example mcp_client_example
```

## Integration with MCP Clients

### Claude Desktop

Add to your Claude Desktop configuration:

```json
{
  "mcpServers": {
    "rzn_datasourcer": {
      "command": "/path/to/rzn_datasourcer/target/release/mcp_server",
      "args": [],
      "env": {
        "GOOGLE_API_KEY": "your_api_key",
        "GOOGLE_CSE_ID": "your_cse_id"
      }
    }
  }
}
```

### Other MCP Clients

The server implements the standard MCP protocol and should work with any compliant client. The transport layer uses JSON-RPC over stdio.

## Server Architecture

```
┌─────────────────┐
│   MCP Client    │
└─────────┬───────┘
          │ JSON-RPC/stdio
┌─────────▼───────┐
│  JsonRpcHandler │
└─────────┬───────┘
          │
┌─────────▼───────┐
│   McpServer     │
└─────────┬───────┘
          │
┌─────────▼───────┐
│ProviderRegistry │
└─────────┬───────┘
          │
┌─────────▼───────┐
│   Connectors    │
│ (hackernews,    │
│  wikipedia,     │
│  google, etc.)  │
└─────────────────┘
```

## Error Handling

The server properly handles and reports errors according to the JSON-RPC specification:

- Parse errors (-32700)
- Invalid params (-32602)
- Method not found (-32601)
- Internal errors (-32603)
- Connector-specific errors (mapped to appropriate codes)

## Logging

Set log level via environment variable:

```bash
export RUST_LOG=rzn_datasourcer=debug
cargo run --bin mcp_server
```

## Development

### Adding New Connectors

1. Implement the `Connector` trait in `src/connectors/`
2. Register the connector in `src/bin/mcp_server.rs`
3. The connector will automatically be exposed via MCP

### Testing

```bash
# Test the library
cargo test

# Test MCP server
cargo run --example mcp_client_example
```

## Standards Compliance

This implementation follows the [Model Context Protocol specification](https://modelcontextprotocol.io/) and is compatible with MCP clients and tools.