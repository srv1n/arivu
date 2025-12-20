# Arivu - Connector Reference

> A unified interface to 30+ data sources through the Model Context Protocol

**MCP tool names are prefixed with connector name**, e.g. `youtube/search_videos`.

---

## Quick Navigation

| Category | Connectors |
|----------|------------|
| [Media & Social](#media--social) | YouTube, Reddit, X (Twitter), Hacker News |
| [Academic & Research](#academic--research) | arXiv, PubMed, Semantic Scholar, SciHub |
| [Web Search](#web-search) | Serper, SerpAPI, Tavily, + more |
| [AI-Powered Search](#ai-powered-search) | OpenAI, Anthropic, Gemini, Perplexity |
| [Productivity](#productivity) | Slack, GitHub, Atlassian |
| [Google Workspace](#google-workspace) | Gmail, Calendar, Drive, Contacts |
| [Microsoft 365](#microsoft-365) | Outlook, Teams, OneDrive |
| [Web Scraping](#web-scraping) | Generic web |
| [Reference](#reference) | Wikipedia |

---

## Media & Social

### YouTube
> Video details, transcripts, chapters, and search

| Tool | Description |
|------|-------------|
| `get_video_details` | Fetch video metadata, full transcript organized by chapters |
| `search_videos` | Search videos, playlists, or channels with filters |

**Task → Tool (MCP name):**
| Task | Tool |
|------|------|
| Video details + transcript | `youtube/get_video_details` |
| Search videos/playlists/channels | `youtube/search_videos` |

**Features:**
- Automatic transcript extraction with chapter grouping
- Search filters: upload date, sort order, content type
- No authentication required

**Example:**
```bash
arivu get youtube "dQw4w9WgXcQ"
arivu search youtube "rust programming" --limit 10
```

---

### Reddit
> Posts, comments, subreddits, and user profiles

| Tool | Description |
|------|-------------|
| `get_subreddit_top_posts` | Get top posts from any subreddit |
| `get_subreddit_hot_posts` | Get trending/hot posts |
| `get_subreddit_new_posts` | Get newest posts |
| `get_post_details` | Get post with full comment tree |
| `search_reddit` | Advanced search with filters |
| `get_user_info` | Get user profile and stats |
| `get_subreddit_info` | Get subreddit metadata |

**Task → Tool (MCP name):**
| Task | Tool |
|------|------|
| Top posts in a subreddit | `reddit/get_subreddit_top_posts` |
| Trending/hot posts | `reddit/get_subreddit_hot_posts` |
| Newest posts | `reddit/get_subreddit_new_posts` |
| Keyword search | `reddit/search_reddit` |
| Post + comments | `reddit/get_post_details` |

**Features:**
- Works anonymously or with authentication
- Comment threading with configurable depth
- Search by author, subreddit, flair, domain

**Authentication:** Optional (Client ID + Secret for higher rate limits)

---

### X (Twitter)
> Tweets, profiles, timelines, and direct messages

| Tool | Description |
|------|-------------|
| `get_profile` | Get user profile information |
| `search_tweets` | Search tweets by keyword |
| `get_tweet` | Get specific tweet with engagement data |
| `get_home_timeline` | Get authenticated user's feed |
| `fetch_tweets_and_replies` | Get all tweets from a user |
| `search_profiles` | Search for user profiles |
| `get_followers` | Get user's followers list |
| `get_direct_message_conversations` | Access DM threads |
| `send_direct_message` | Send DMs (authenticated) |

**Task → Tool (MCP name):**
| Task | Tool |
|------|------|
| User profile | `x/get_profile` |
| Keyword search | `x/search_tweets` |
| Tweet details | `x/get_tweet` |
| Home timeline | `x/get_home_timeline` |
| User tweets + replies | `x/fetch_tweets_and_replies` |
| Search profiles | `x/search_profiles` |
| Followers | `x/get_followers` |
| DM conversations | `x/get_direct_message_conversations` |
| Send DM | `x/send_direct_message` |

**Authentication:** Required (browser cookies or credentials)

```bash
arivu setup x                    # Configure via browser cookies
arivu config set x --browser chrome
```

---

### Hacker News
> Tech news, discussions, and job postings

| Tool | Description |
|------|-------------|
| `search_stories` | Keyword search via Algolia |
| `search_by_date` | Recent search via Algolia |
| `get_stories` | Stories by type (top/new/best/ask/show/job) |
| `get_post` | Story or comment with comments |

**Features:**
- Powered by Algolia search API
- Flattened or nested comment trees
- No authentication required

**Task → Tool (MCP name):**
| Task | Tool |
|------|------|
| Top/new/best/ask/show/job stories | `hackernews/get_stories` |
| Keyword search | `hackernews/search_stories` |
| Recent chronological search | `hackernews/search_by_date` |
| Story with comments | `hackernews/get_post` |

---

## Academic & Research

### arXiv
> Preprints in physics, mathematics, computer science, and more

| Tool | Description |
|------|-------------|
| `search_papers` | Search arXiv by query |
| `get_paper_details` | Paper metadata by arXiv ID |
| `get_paper_pdf` | Paper PDF (base64) |

**Features:**
- Field-specific search: `ti:` (title), `au:` (author), `abs:` (abstract)
- Sort by relevance, submission date, or update date
- No authentication required

**Task → Tool (MCP name):**
| Task | Tool |
|------|------|
| Search papers | `arxiv/search_papers` |
| Paper details | `arxiv/get_paper_details` |
| Download PDF | `arxiv/get_paper_pdf` |

**Example:**
```bash
arivu search arxiv "au:hinton AND ti:neural"
```

---

### PubMed
> Biomedical and life sciences literature

| Tool | Description |
|------|-------------|
| `search` | Search PubMed |
| `get_abstract` | Abstract + metadata by PMID |

**Features:**
- 35+ million citations from MEDLINE and life science journals
- MeSH term support
- No authentication required

**Task → Tool (MCP name):**
| Task | Tool |
|------|------|
| Search articles | `pubmed/search` |
| Get abstract | `pubmed/get_abstract` |

---

### bioRxiv / medRxiv (`biorxiv`)
> Preprints via the official bioRxiv API

| Tool | Description |
|------|-------------|
| `get_recent_preprints` | Recent preprints |
| `get_preprints_by_date` | Preprints by date range |
| `get_preprint_by_doi` | Preprint by DOI |

**Task → Tool (MCP name):**
| Task | Tool |
|------|------|
| Recent preprints | `biorxiv/get_recent_preprints` |
| Date range | `biorxiv/get_preprints_by_date` |
| DOI lookup | `biorxiv/get_preprint_by_doi` |

---

### Google Scholar (`google_scholar`)
> Scholar search via scraping (unofficial)

| Tool | Description |
|------|-------------|
| `search_papers` | Search papers |

**Task → Tool (MCP name):**
| Task | Tool |
|------|------|
| Search papers | `google_scholar/search_papers` |

---

### Semantic Scholar
> Academic papers with citation graphs

| Tool | Description |
|------|-------------|
| `search_papers` | Search papers |
| `get_paper_details` | Paper details by paper_id |
| `get_related_papers` | Related papers by paper_id |

**Features:**
- Citation and reference graphs
- Influence and citation velocity metrics
- Free API (no auth required)

**Task → Tool (MCP name):**
| Task | Tool |
|------|------|
| Search papers | `semantic_scholar/search_papers` |
| Paper details | `semantic_scholar/get_paper_details` |
| Related papers | `semantic_scholar/get_related_papers` |

---

### SciHub
> Research paper access

| Tool | Description |
|------|-------------|
| `get_paper` | Retrieve paper by DOI |

**Features:**
- Access papers by DOI
- No authentication required

**Task → Tool (MCP name):**
| Task | Tool |
|------|------|
| Fetch paper by DOI | `scihub/get_paper` |

---

## Web Search

### Search APIs

| Connector | Description | Auth Required |
|-----------|-------------|---------------|
| `serper-search` | Google Search via Serper | API Key |
| `serpapi-search` | Multi-engine via SerpAPI | API Key |
| `tavily-search` | AI-optimized search | API Key |
| `exa-search` | Neural search | API Key |
| `firecrawl-search` | Web crawling & search | API Key |

**Common Features:**
- Structured search results with snippets
- Pagination support
- Domain filtering

**Task → Tool (MCP name):**
| Task | Tool |
|------|------|
| Web search via Serper | `serper-search/search` |
| Web search via SerpAPI | `serpapi-search/search` |
| Web/news search via Tavily | `tavily-search/search` |
| Semantic search via Exa | `exa-search/search` |
| Search + scrape via Firecrawl | `firecrawl-search/search` |
| Exa extra tools | `exa-search/get_contents`, `exa-search/find_similar`, `exa-search/answer`, `exa-search/research` |

---

### Parallel Search (`parallel_search`)
> Parallel multi-query search and scheduled monitoring

| Tool | Description |
|------|-------------|
| `search` | Parallel web search |
| `create_monitor` | Create a monitor |
| `list_monitors` | List monitors |
| `get_monitor_events` | Monitor events |
| `cancel_monitor` | Cancel monitor |

**Task → Tool (MCP name):**
| Task | Tool |
|------|------|
| Parallel search | `parallel_search/search` |
| Create monitor | `parallel_search/create_monitor` |
| List monitors | `parallel_search/list_monitors` |
| Monitor events | `parallel_search/get_monitor_events` |
| Cancel monitor | `parallel_search/cancel_monitor` |

---

## AI-Powered Search

These connectors use LLM providers' native web search capabilities:

### OpenAI Search (`openai-search`)
> Web search via OpenAI Responses API

| Tool | Description |
|------|-------------|
| `search` | Grounded web search with AI synthesis |

**Task → Tool (MCP name):**
| Task | Tool |
|------|------|
| Grounded web search | `openai-search/search` |

**Auth:** `OPENAI_API_KEY`

---

### Claude Web Search (`anthropic-search`)
> Web search via Anthropic's Claude

| Tool | Description |
|------|-------------|
| `search` | Grounded search with citations |

**Parameters:**
- `query` - Search query
- `max_results` - Result limit
- `allowed_domains` / `blocked_domains` - Domain filtering
- `date_range` - Time filtering

**Auth:** `ANTHROPIC_API_KEY`

**Task → Tool (MCP name):**
| Task | Tool |
|------|------|
| Grounded web search | `anthropic-search/search` |

---

### Gemini Search (`gemini-search`)
> Google Search grounding via Gemini

| Tool | Description |
|------|-------------|
| `search` | Search with Google's latest index |

**Task → Tool (MCP name):**
| Task | Tool |
|------|------|
| Grounded web search | `gemini-search/search` |

**Auth:** `GOOGLE_API_KEY`

---

### Perplexity Search (`perplexity-search`)
> Real-time web search with AI synthesis

| Tool | Description |
|------|-------------|
| `search` | Search with real-time results |

**Auth:** `PERPLEXITY_API_KEY`

**Task → Tool (MCP name):**
| Task | Tool |
|------|------|
| Grounded web search | `perplexity-search/search` |

---

### Additional AI Search Providers

| Connector | Description | Auth |
|-----------|-------------|------|
| `tavily-search` | Fast search with summaries | `TAVILY_API_KEY` |
| `exa-search` | Semantic/neural search | `EXA_API_KEY` |
| `firecrawl-search` | Web scraping + search | `FIRECRAWL_API_KEY` |
| `xai-search` | X.AI Grok web search | `XAI_API_KEY` |

**Task → Tool (MCP name):**
| Task | Tool |
|------|------|
| Live search via xAI | `xai-search/search` |

---

## Productivity

### Slack
> Workspace messages, channels, and files

| Tool | Description |
|------|-------------|
| `test_auth` | Verify Slack connection |
| `list_channels` | List all workspace channels |
| `list_messages` | Get messages from a channel |
| `get_thread` | Get thread replies |
| `search_messages` | Search across workspace |
| `list_files` | List files in a channel |
| `get_thread_by_permalink` | Get thread by Slack URL |

**Auth:** Bot Token (`xoxb-...`)

```bash
arivu setup slack
arivu config set slack --value "xoxb-your-token"
```

**Required Scopes:** `channels:read`, `channels:history`, `users:read`, `files:read`, `search:read`

**Task → Tool (MCP name):**
| Task | Tool |
|------|------|
| List channels | `slack/list_channels` |
| Recent channel messages | `slack/list_messages` |
| Thread replies | `slack/get_thread` |
| Search messages | `slack/search_messages` |
| List files | `slack/list_files` |
| Thread from permalink | `slack/get_thread_by_permalink` |

---

### Discord (`discord`)
> Servers, channels, and messages (bot token)

| Tool | Description |
|------|-------------|
| `list_servers` | List servers |
| `get_server_info` | Server details |
| `list_channels` | List channels |
| `read_messages` | Read channel messages |
| `search_messages` | Search channel messages |
| `send_message` | Send message |

**Task → Tool (MCP name):**
| Task | Tool |
|------|------|
| List servers | `discord/list_servers` |
| Server info | `discord/get_server_info` |
| List channels | `discord/list_channels` |
| Read messages | `discord/read_messages` |
| Search messages | `discord/search_messages` |
| Send message | `discord/send_message` |

---

### GitHub
> Repositories, issues, PRs, and code search

| Tool | Description |
|------|-------------|
| `list_issues` | List issues with filters |
| `get_issue` | Get issue details |
| `list_pull_requests` | List pull requests |
| `get_pull_request` | Get PR details |
| `get_pull_diff` | Get PR diff (size-capped) |
| `code_search` | Search code across GitHub |
| `get_file` | Get file contents |

**Auth:** Personal Access Token

```bash
arivu setup github
arivu config set github --value "ghp_your_token"
```

**Required Scopes:** `repo` (read), `read:org`

**Task → Tool (MCP name):**
| Task | Tool |
|------|------|
| List issues | `github/list_issues` |
| Issue details | `github/get_issue` |
| List PRs | `github/list_pull_requests` |
| PR details | `github/get_pull_request` |
| PR diff | `github/get_pull_diff` |
| Code search | `github/code_search` |
| File contents | `github/get_file` |

---

### Atlassian
> Jira issues and Confluence pages

| Tool | Description |
|------|-------------|
| `test_auth` | Validate Jira/Confluence auth |
| `jira_search_issues` | Search Jira issues (JQL) |
| `jira_get_issue` | Get Jira issue details |
| `conf_search_pages` | Search Confluence pages (CQL) |
| `conf_get_page` | Get Confluence page |

**Auth:** API Token + Email

**Task → Tool (MCP name):**
| Task | Tool |
|------|------|
| Jira search (JQL) | `atlassian/jira_search_issues` |
| Jira issue details | `atlassian/jira_get_issue` |
| Confluence search | `atlassian/conf_search_pages` |
| Confluence page | `atlassian/conf_get_page` |

---

## Google Workspace

### Gmail (`google-gmail`)
| Tool | Description |
|------|-------------|
| `list_messages` | List messages (q filter) |
| `get_message` | Get message by id |
| `get_thread` | Get thread by id |
| `decode_message_raw` | Decode raw message |

**Task → Tool (MCP name):**
| Task | Tool |
|------|------|
| List/search messages | `google-gmail/list_messages` |
| Message details | `google-gmail/get_message` |
| Thread details | `google-gmail/get_thread` |
| Decode raw message | `google-gmail/decode_message_raw` |

**Notes:** Requires explicit user permission.

### Calendar (`google-calendar`)
| Tool | Description |
|------|-------------|
| `list_events` | List events |
| `create_event` | Create event |
| `update_event` | Update event |
| `delete_event` | Delete event |
| `sync_events` | Incremental sync |
| `watch_events` | Start webhook (if enabled) |
| `stop_channel` | Stop webhook |

**Task → Tool (MCP name):**
| Task | Tool |
|------|------|
| List events | `google-calendar/list_events` |
| Create event | `google-calendar/create_event` |
| Update event | `google-calendar/update_event` |
| Delete event | `google-calendar/delete_event` |
| Incremental sync | `google-calendar/sync_events` |

**Notes:** Requires explicit user permission.

### Drive (`google-drive`)
| Tool | Description |
|------|-------------|
| `list_files` | List files and folders |
| `get_file` | Get file metadata |
| `download_file` | Download file (base64) |
| `export_file` | Export Docs/Sheets/Slides |
| `upload_file` | Upload file (base64) |
| `upload_file_resumable` | Resumable upload |
| `find_and_export` | Find and export Doc/Sheet/Slide |

**Task → Tool (MCP name):**
| Task | Tool |
|------|------|
| List/search files | `google-drive/list_files` |
| File metadata | `google-drive/get_file` |
| Download content | `google-drive/download_file` |
| Export Doc/Sheet/Slide | `google-drive/export_file` |
| Upload file | `google-drive/upload_file` |
| Resumable upload | `google-drive/upload_file_resumable` |
| Find and export | `google-drive/find_and_export` |

**Notes:** Requires explicit user permission.

### Contacts (`google-people`)
| Tool | Description |
|------|-------------|
| `list_connections` | List contacts |
| `get_person` | Get contact by resourceName |

**Task → Tool (MCP name):**
| Task | Tool |
|------|------|
| List contacts | `google-people/list_connections` |
| Contact details | `google-people/get_person` |

**Notes:** Requires explicit user permission.

**Auth:** OAuth 2.0 or Service Account

---

## Microsoft 365

### Microsoft Graph (`microsoft`)
> Unified API for Microsoft 365 services

| Tool | Description |
|------|-------------|
| `list_messages` | List Outlook messages |
| `get_message` | Get message by ID |
| `list_events` | List calendar events |
| `send_mail` | Send email |
| `create_draft` | Create draft email |
| `upload_attachment_large` | Upload attachment (base64) |
| `upload_attachment_large_from_path` | Upload attachment from file |
| `send_draft` | Send draft |
| `auth_start` | Start device auth |
| `auth_poll` | Poll device auth |

**Auth:** Azure AD OAuth

**Task → Tool (MCP name):**
| Task | Tool |
|------|------|
| List messages | `microsoft/list_messages` |
| Message details | `microsoft/get_message` |
| List events | `microsoft/list_events` |
| Send mail | `microsoft/send_mail` |
| Draft + attachment | `microsoft/create_draft`, `microsoft/upload_attachment_large` |
| Send draft | `microsoft/send_draft` |

**Notes:** Requires explicit user permission.

---

## Feeds

### RSS (`rss`)
> RSS/Atom/JSON feeds

| Tool | Description |
|------|-------------|
| `get_feed` | Fetch feed + entries |
| `list_entries` | List entries |
| `search_feed` | Search entries |
| `discover_feeds` | Discover feeds on a webpage |

**Task → Tool (MCP name):**
| Task | Tool |
|------|------|
| Fetch feed | `rss/get_feed` |
| List entries | `rss/list_entries` |
| Search entries | `rss/search_feed` |
| Discover feeds | `rss/discover_feeds` |

---

## Local System

### Local Files (`localfs`)
> Local filesystem indexing and extraction

| Tool | Description |
|------|-------------|
| `list_files` | List files |
| `get_file_info` | File metadata |
| `extract_text` | Extract file text |
| `get_structure` | Document structure |
| `get_section` | Get section |
| `search_content` | Search within file |

**Task → Tool (MCP name):**
| Task | Tool |
|------|------|
| List files | `localfs/list_files` |
| File metadata | `localfs/get_file_info` |
| Extract text | `localfs/extract_text` |
| Document structure | `localfs/get_structure` |
| Get section | `localfs/get_section` |
| Search content | `localfs/search_content` |

---

### Spotlight (`spotlight`)
> macOS Spotlight index search

| Tool | Description |
|------|-------------|
| `search_content` | Full-text search |
| `search_by_name` | Search by name |
| `search_by_kind` | Search by kind |
| `search_recent` | Recently modified |
| `get_metadata` | File metadata |
| `raw_query` | Raw mdfind query |

**Task → Tool (MCP name):**
| Task | Tool |
|------|------|
| Full-text search | `spotlight/search_content` |
| Search by name | `spotlight/search_by_name` |
| Search by kind | `spotlight/search_by_kind` |
| Recent files | `spotlight/search_recent` |
| File metadata | `spotlight/get_metadata` |
| Raw query | `spotlight/raw_query` |

---

## Web Scraping

### Web (`web`)
> Generic web content extraction

| Tool | Description |
|------|-------------|
| `scrape_url` | Extract text content from URL |
| `scrape_with_config` | Advanced scraping with selectors |

**Features:**
- Clean text extraction
- Custom CSS selectors
- No authentication required

**Task → Tool (MCP name):**
| Task | Tool |
|------|------|
| Extract text from URL | `web/scrape_url` |
| Custom selectors | `web/scrape_with_config` |

---

## Reference

### Wikipedia
> Encyclopedia articles in multiple languages

| Tool | Description |
|------|-------------|
| `search` | Search Wikipedia |
| `get_article` | Get article content |
| `geosearch` | Find articles by location |

**Features:**
- Multi-language support
- Geographic search by coordinates
- No authentication required

**Task → Tool (MCP name):**
| Task | Tool |
|------|------|
| Keyword search | `wikipedia/search` |
| Article content | `wikipedia/get_article` |
| Geo search | `wikipedia/geosearch` |

---

## Authentication Quick Reference

### No Authentication Required
```
arxiv, hackernews, pubmed, scihub, semantic_scholar, web, wikipedia, youtube*
```
*YouTube works without auth but may have rate limits

### Environment Variables
```bash
# AI Search
export OPENAI_API_KEY="..."
export ANTHROPIC_API_KEY="..."
export PERPLEXITY_API_KEY="..."
export TAVILY_API_KEY="..."

# Productivity
export SLACK_TOKEN="xoxb-..."
export GITHUB_TOKEN="ghp_..."

# Social
export REDDIT_CLIENT_ID="..."
export REDDIT_CLIENT_SECRET="..."
```

### CLI Configuration
```bash
arivu setup                      # Interactive wizard
arivu setup <connector>          # Configure specific connector
arivu config set <connector> --value "token"
arivu config test <connector>    # Verify authentication
```

### Config File Location
- **macOS/Linux:** `~/.config/arivu/auth.json`
- **Windows:** `%APPDATA%\arivu\auth.json`

---

## Feature Flags

Build with specific connectors to minimize binary size:

```bash
# Minimal (no connectors)
cargo build --release -p arivu_cli --no-default-features

# Specific connectors
cargo build --release -p arivu_cli --features "youtube,hackernews,arxiv"

# All connectors
cargo build --release -p arivu_cli --features full

# AI search providers
cargo build --release -p arivu_cli --features "openai-search,anthropic-search"
```

---

## Need Help?

```bash
arivu --help                     # General help
arivu tools <connector>          # Show connector tools
arivu connectors                 # List all connectors
```

[GitHub Issues](https://github.com/srv1n/arivu/issues) | [Installation Guide](../INSTALLATION.md)
