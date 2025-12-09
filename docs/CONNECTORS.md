# Arivu - Connector Reference

> A unified interface to 30+ data sources through the Model Context Protocol

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
| `search_stories` | Full-text search via Algolia |
| `search_by_date` | Search recent stories chronologically |
| `get_top_stories` | Get current top stories |
| `get_new_stories` | Get newest submissions |
| `get_best_stories` | Get highest-ranked stories |
| `get_ask_stories` | Get "Ask HN" posts |
| `get_show_stories` | Get "Show HN" posts |
| `get_job_stories` | Get job postings |
| `get_post` | Get post with nested comments |

**Features:**
- Powered by Algolia search API
- Flattened or nested comment trees
- No authentication required

---

## Academic & Research

### arXiv
> Preprints in physics, mathematics, computer science, and more

| Tool | Description |
|------|-------------|
| `search_papers` | Search papers with field-specific queries |
| `get_paper_details` | Get full metadata by arXiv ID |
| `get_paper_pdf` | Download PDF (base64 encoded) |

**Features:**
- Field-specific search: `ti:` (title), `au:` (author), `abs:` (abstract)
- Sort by relevance, submission date, or update date
- No authentication required

**Example:**
```bash
arivu search arxiv "au:hinton AND ti:neural"
```

---

### PubMed
> Biomedical and life sciences literature

| Tool | Description |
|------|-------------|
| `search` | Search medical literature |
| `get_article` | Get article abstract and metadata |

**Features:**
- 35+ million citations from MEDLINE and life science journals
- MeSH term support
- No authentication required

---

### Semantic Scholar
> Academic papers with citation graphs

| Tool | Description |
|------|-------------|
| `search_papers` | Search academic papers |
| `get_paper` | Get paper with citations and references |
| `get_author` | Get author profile and publications |

**Features:**
- Citation and reference graphs
- Influence and citation velocity metrics
- Free API (no auth required)

---

### SciHub
> Research paper access

| Tool | Description |
|------|-------------|
| `get_paper` | Retrieve paper by DOI |

**Features:**
- Access papers by DOI
- No authentication required

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

---

## AI-Powered Search

These connectors use LLM providers' native web search capabilities:

### OpenAI Search (`openai-search`)
> Web search via OpenAI Responses API

| Tool | Description |
|------|-------------|
| `search` | Grounded web search with AI synthesis |

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

---

### Gemini Search (`gemini-search`)
> Google Search grounding via Gemini

| Tool | Description |
|------|-------------|
| `search` | Search with Google's latest index |

**Auth:** `GOOGLE_API_KEY`

---

### Perplexity Search (`perplexity-search`)
> Real-time web search with AI synthesis

| Tool | Description |
|------|-------------|
| `search` | Search with real-time results |

**Auth:** `PERPLEXITY_API_KEY`

---

### Additional AI Search Providers

| Connector | Description | Auth |
|-----------|-------------|------|
| `tavily-search` | Fast search with summaries | `TAVILY_API_KEY` |
| `exa-search` | Semantic/neural search | `EXA_API_KEY` |
| `firecrawl-search` | Web scraping + search | `FIRECRAWL_API_KEY` |
| `xai-search` | X.AI Grok web search | `XAI_API_KEY` |

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

---

### GitHub
> Repositories, issues, PRs, and code search

| Tool | Description |
|------|-------------|
| `list_issues` | List issues with filters |
| `get_issue` | Get issue details |
| `list_pulls` | List pull requests |
| `get_pull` | Get PR details and diff |
| `code_search` | Search code across GitHub |
| `get_file` | Get file contents |

**Auth:** Personal Access Token

```bash
arivu setup github
arivu config set github --value "ghp_your_token"
```

**Required Scopes:** `repo` (read), `read:org`

---

### Atlassian
> Jira issues and Confluence pages

| Tool | Description |
|------|-------------|
| `search_issues` | Search Jira issues (JQL) |
| `get_issue` | Get issue details |
| `search_confluence` | Search Confluence pages |
| `get_page` | Get page content |

**Auth:** API Token + Email

---

## Google Workspace

### Gmail (`google-gmail`)
| Tool | Description |
|------|-------------|
| `list_messages` | List emails with filters |
| `get_message` | Get email content |
| `search` | Search emails |

### Calendar (`google-calendar`)
| Tool | Description |
|------|-------------|
| `list_events` | List calendar events |
| `get_event` | Get event details |
| `list_calendars` | List available calendars |

### Drive (`google-drive`)
| Tool | Description |
|------|-------------|
| `list_files` | List files and folders |
| `get_file` | Get file metadata |
| `search` | Search files |

### Contacts (`google-people`)
| Tool | Description |
|------|-------------|
| `list_contacts` | List contacts |
| `get_contact` | Get contact details |
| `search` | Search contacts |

**Auth:** OAuth 2.0 or Service Account

---

## Microsoft 365

### Microsoft Graph (`microsoft`)
> Unified API for Microsoft 365 services

| Tool | Description |
|------|-------------|
| `list_messages` | List Outlook emails |
| `get_message` | Get email content |
| `list_events` | List calendar events |
| `list_files` | List OneDrive files |
| `search` | Cross-service search |

**Auth:** Azure AD OAuth

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

---

## Reference

### Wikipedia
> Encyclopedia articles in multiple languages

| Tool | Description |
|------|-------------|
| `search_articles` | Search Wikipedia |
| `get_article` | Get full article content |
| `geo_search` | Find articles by location |

**Features:**
- Multi-language support
- Geographic search by coordinates
- No authentication required

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
