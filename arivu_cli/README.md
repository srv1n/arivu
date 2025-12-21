# Arivu CLI - User Guide

## Overview

Arivu CLI (`arivu`) is a unified command-line tool for accessing data from 20+ sources including YouTube, Reddit, academic papers, news, and more. Get structured data with beautiful terminal output and multiple export formats.

## Installation

### Prerequisites
- Rust 1.70+ and Cargo

### Quick Install
```bash
git clone https://github.com/srv1n/arivu.git
cd arivu
cargo build --release

# Add to PATH (optional)
sudo cp target/release/arivu /usr/local/bin/
```

### Verify Installation
```bash
arivu --version
arivu --help
```

## Quick Start

### 1. List Available Data Sources
```bash
arivu list
```
```
Available Data Sources

┌─────────────────────┬──────────────────────────────────────────────┐
│ Name                │ Description                                  │
├─────────────────────┼──────────────────────────────────────────────┤
│ youtube            │ A connector for interacting with YouTube.    │
│ hackernews         │ Hacker News via Firebase and Algolia API     │
│ wikipedia          │ Wikipedia article search and retrieval       │
│ arxiv              │ Academic papers from arXiv.org               │
│ pubmed             │ Medical literature database                  │
└─────────────────────┴──────────────────────────────────────────────┘
```

### 2. Search for Content
```bash
arivu search youtube "rust programming" --limit 3
```
```
Search Results: rust programming
Connector: youtube

1. Learn Rust Programming - Complete Course
   https://www.youtube.com/watch?v=BpPEoZW5IiY
   In this comprehensive Rust course for beginners...

2. Rust Programming Full Course | Learn in 2024
   https://www.youtube.com/watch?v=rQ_J9WH6CGk
   Duration: 3 hours and 5 minutes...
```

### 3. Get Detailed Content
```bash
arivu get youtube BpPEoZW5IiY
```
```
Resource: BpPEoZW5IiY (youtube)

Title: Learn Rust Programming - Complete Course

Description:
In this comprehensive Rust course for beginners, you will learn about
the core concepts of the language and underlying mechanisms in theory.

Chapters:
  * 0:00 - Introduction & Learning Resources
      Welcome to this rust programming course for beginners...
  * 6:19 - Variables
      Variables are assigned using the let keyword...
  * 27:07 - Numbers & Binary System
      Numbers in Rust come in different types...
```

## Core Commands

### `arivu list` - Show Available Connectors
Lists all available data source connectors with descriptions.

**Usage:**
```bash
arivu list [OPTIONS]
```

**Options:**
- `--output FORMAT` - Output format (pretty, json, yaml, text, markdown)

**Examples:**
```bash
arivu list                    # Pretty table view
arivu list --output json      # JSON format
arivu list --output markdown  # Markdown format
```

### `arivu search` - Search for Content
Search for content using a specific data source connector.

**Usage:**
```bash
arivu search <CONNECTOR> <QUERY> [OPTIONS]
```

**Arguments:**
- `CONNECTOR` - Name of the connector (youtube, reddit, hackernews, etc.)
- `QUERY` - Search query string

**Options:**
- `--limit NUMBER` - Maximum number of results (default: 10)
- `--output FORMAT` - Output format

**Examples:**
```bash
# Search YouTube videos
arivu search youtube "machine learning" --limit 5

# Search academic papers
arivu search arxiv "quantum computing" --limit 3

# Search Hacker News
arivu search hackernews "rust language" --limit 10

# Export results as JSON
arivu search reddit "programming" --output json --limit 5
```

### `arivu get` - Fetch Specific Content
Retrieve detailed information about a specific resource.

**Usage:**
```bash
arivu get <CONNECTOR> <ID> [OPTIONS]
```

**Arguments:**
- `CONNECTOR` - Name of the connector
- `ID` - Resource ID or URL

**Options:**
- `--output FORMAT` - Output format

**Examples:**
```bash
# Get YouTube video with transcript
arivu get youtube dQw4w9WgXcQ
arivu get youtube "https://www.youtube.com/watch?v=dQw4w9WgXcQ"

# Get Wikipedia article
arivu get wikipedia "Rust (programming language)"

# Get research paper
arivu get arxiv "2301.07041"

# Export as JSON
arivu get youtube BpPEoZW5IiY --output json
```

### `arivu connectors` - Detailed Connector Information
Show comprehensive information about all connectors including status and capabilities.

**Usage:**
```bash
arivu connectors [OPTIONS]
```

**Examples:**
```bash
arivu connectors                  # Show all connector details
arivu connectors --output yaml    # Export as YAML
```

### Connector Subcommands (Recommended)

Each connector has its own subcommand with proper CLI flags:

```bash
# YouTube
arivu youtube search --query "rust programming" --limit 10
arivu youtube video --id dQw4w9WgXcQ
arivu youtube transcript --id dQw4w9WgXcQ

# Hacker News
arivu hackernews top --limit 20
arivu hackernews search --query "rust" --limit 10
arivu hn story --id 38500000

# arXiv
arivu arxiv search --query "transformer architecture" --limit 10
arivu arxiv paper --id 2301.07041

# GitHub
arivu github search-repos --query "rust cli"
arivu gh issues --repo rust-lang/rust --state open

# Local filesystem
arivu localfs list-files --path ~/Documents --recursive
arivu localfs extract-text --path ~/paper.pdf

# Use --help on any subcommand
arivu hackernews --help
arivu youtube search --help
```

### Tool Discovery

Use `arivu tools <connector>` to see what each connector exposes and then use the connector's
subcommand wrappers (recommended):

```bash
arivu tools reddit
arivu reddit --help
```

### `arivu config` - Manage Authentication

Manage authentication credentials for connectors.

**Usage:**
```bash
arivu config <ACTION> [OPTIONS]
```

**Actions:**
- `show` - Display current configuration
- `set` - Configure authentication
- `test` - Test authentication
- `remove` - Remove authentication

**Examples:**
```bash
# Show current config
arivu config show

# Set API key authentication
arivu config set github --value "ghp_your_token"

# Set browser cookie authentication
arivu config set x --auth-type browser --browser chrome

# Test authentication
arivu config test github

# Remove authentication
arivu config remove reddit
```

## Available Data Sources

### Media & Entertainment

#### YouTube (`youtube`)
- **Search videos** by keywords
- **Get video details** with full transcripts organized by chapters
- **No authentication required**

```bash
arivu search youtube "rust tutorial"
arivu get youtube dQw4w9WgXcQ
```

#### Reddit (`reddit`) *[Requires Auth]*
- **Search posts** and comments
- **Get full thread hierarchies**
- **Authentication:** Client ID & Secret

```bash
# Set up authentication first
export REDDIT_CLIENT_ID="your_client_id"
export REDDIT_CLIENT_SECRET="your_client_secret"

arivu search reddit "programming tips"
arivu get reddit "post_id_here"
```

### Academic & Research

#### ArXiv (`arxiv`)
- **Search academic preprints**
- **Get full paper metadata** and abstracts
- **No authentication required**

```bash
arivu search arxiv "machine learning"
arivu get arxiv "2301.07041"
```

#### PubMed (`pubmed`)
- **Search medical literature**
- **Get article abstracts** with MeSH terms
- **No authentication required**

```bash
arivu search pubmed "covid vaccine efficacy"
arivu get pubmed "34762503"
```

#### Semantic Scholar (`semantic_scholar`)
- **Search academic papers** with citation data
- **Get citation graphs** and influence metrics
- **No authentication required**

```bash
arivu search semantic_scholar "neural networks"
arivu get semantic_scholar "paper_id"
```

### Web & Social

#### Hacker News (`hackernews`)
- **Search tech news** and discussions
- **Get comment threads** with user karma
- **No authentication required**

```bash
arivu search hackernews "artificial intelligence"
arivu get hackernews "story_id"
```

#### Wikipedia (`wikipedia`)
- **Search encyclopedia articles**
- **Get full article content** with references
- **No authentication required**

```bash
arivu search wikipedia "quantum computing"
arivu get wikipedia "Rust (programming language)"
```

#### X/Twitter (`x`) *[Requires Auth]*
- **Search tweets** and profiles
- **Get real-time data** and user analytics
- **Authentication:** Browser cookies or credentials

```bash
# Browser cookie authentication (recommended)
arivu config set x --auth-type browser --browser chrome

# Manual credentials
arivu config set x --auth-type credentials \
  --username "your_username" \
  --password "your_password"

arivu search x "machine learning"
```

### Web Scraping

#### Web (`web`)
- **General web scraping** with CSS selectors
- **Form handling** and custom requests
- **No authentication required**

```bash
arivu tools web  # See available scraping tools
```

## Output Formats

### Pretty (Default)
Human-readable output with colors, tables, and formatting.
```bash
arivu list  # Uses pretty format by default
```

### JSON
Machine-readable structured data.
```bash
arivu search youtube "rust" --output json
```
```json
{
  "type": "SearchResults",
  "data": {
    "connector": "youtube",
    "query": "rust",
    "results": {
      "videos": [...]
    }
  }
}
```

### YAML
YAML format for configuration files.
```bash
arivu connectors --output yaml
```

### Text
Plain text without formatting.
```bash
arivu list --output text
```

### Markdown
Markdown format for documentation.
```bash
arivu tools youtube --output markdown
```

## Global Options

All global options must be placed **before** the subcommand.

| Option | Short | Description |
|--------|-------|-------------|
| `--output <FORMAT>` | | Output format: `pretty`, `json`, `yaml`, `text`, `markdown` |
| `--copy` | `-c` | Copy output to system clipboard |
| `--no-color` | | Disable colored output |
| `--verbose` | `-v` | Verbose output (can repeat: `-vv`, `-vvv`) |
| `--tui` | | Launch interactive TUI mode *(Coming Soon)* |

### Examples
```bash
# Copy results to clipboard
arivu --copy fetch https://arxiv.org/abs/2301.07041
arivu -c search youtube "rust tutorial"

# Output as JSON and copy to clipboard
arivu --copy --output json hackernews search_stories "rust"

# Verbose mode for debugging
arivu -vv search youtube "test"
```

**Note:** Global flags must come before the subcommand:
```bash
arivu --copy fetch hn:12345678    # ✓ Correct
arivu fetch --copy hn:12345678    # ✗ Won't work
```

## Authentication Setup

### Environment Variables Method (Recommended)

#### Reddit
```bash
export REDDIT_CLIENT_ID="your_reddit_client_id"
export REDDIT_CLIENT_SECRET="your_reddit_client_secret"
```

### Browser Cookie Method

For services like X/Twitter, you can extract cookies from your browser:

```bash
arivu config set x --auth-type browser --browser chrome
```

Supported browsers: `chrome`, `firefox`, `safari`, `brave`

## Common Use Cases

### Content Research
```bash
# Find educational videos
arivu search youtube "rust programming tutorial" --limit 5

# Get full transcript for analysis
arivu get youtube BpPEoZW5IiY --output json > transcript.json

# Cross-reference with academic papers
arivu search arxiv "rust programming language"
```

### Market Research
```bash
# Track discussions about a topic
arivu search hackernews "artificial intelligence" --limit 20
arivu search reddit "AI tools" --limit 15

# Monitor news coverage
arivu search hackernews "AI startup funding"
```

### Academic Research
```bash
# Literature review
arivu search pubmed "cancer immunotherapy" --limit 50 --output json
arivu search arxiv "machine learning medicine" --limit 30
arivu search semantic_scholar "deep learning healthcare"

# Get specific papers
arivu get arxiv "2301.07041" --output markdown > paper_summary.md
```

### Data Collection Pipelines
```bash
# Collect and export data
arivu search youtube "data science" --output json > youtube_results.json
arivu search arxiv "data science" --output json > arxiv_results.json
arivu search hackernews "data science" --output json > hn_results.json

# Combine results with jq
jq -s '.' *.json > combined_results.json
```

## Tips & Best Practices

### Performance
- Use `--limit` to control result size
- Export large datasets as JSON for processing
- Run time-consuming operations in background

### Security
- Store API keys in environment variables
- Use browser cookie method for personal accounts
- Rotate credentials regularly

### Automation
- Use JSON output for scripting: `--output json`
- Combine with `jq` for data processing
- Set up aliases for common searches

### Documentation
- Use `arivu tools <connector>` to see available options
- Check connector status with `arivu connectors`
- Export schemas with `--output markdown`

## Troubleshooting

### Common Issues

#### "Connector not found"
```bash
arivu list  # Check available connectors
arivu connectors  # Check connector status
```

#### Authentication errors
```bash
arivu config show  # Check current config
arivu config test <connector>  # Test specific connector
```

#### Network timeouts
```bash
arivu -vv search youtube "test"  # Enable verbose logging
```

#### Missing results
```bash
arivu tools <connector>  # Check available tools
```

### Debug Mode
```bash
RUST_LOG=debug arivu search youtube "test"
```

### Report Issues
If you encounter bugs or have feature requests:
1. Check existing issues at GitHub repository
2. Include command that failed
3. Include error output with `-vv` flag
4. Include system information (OS, Rust version)

## Advanced Usage

### Shell Integration

#### Shell Aliases
```bash
alias yt="arivu search youtube"
alias arxiv="arivu search arxiv"
alias hn="arivu search hackernews"

# Usage
yt "rust tutorial"
arxiv "machine learning"
hn "programming news"
```

### Configuration File *(Coming Soon)*
```toml
# ~/.config/arivu/config.toml
[default]
output_format = "pretty"
verbosity = 1

[connectors.youtube]
default_limit = 10

[connectors.github]
token = "ghp_your_token"
```

### Cross-Platform Usage

#### Windows PowerShell
```powershell
$env:GITHUB_TOKEN="ghp_your_token"
arivu search hackernews "windows tutorial"
```

#### macOS/Linux
```bash
export GITHUB_TOKEN="ghp_your_token"
arivu search hackernews "macos tutorial"
```

This user guide provides comprehensive documentation for effectively using the Arivu CLI tool across various data sources and use cases.
