use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(name = "arivu")]
#[command(about = "Arivu - Unified data access CLI for 30+ sources")]
#[command(version)]
#[command(after_help = "\x1b[1;36mQuick Start:\x1b[0m
  arivu list                              List all available connectors
  arivu tools                             Show all tools with auth requirements
  arivu tools youtube                     Show tools for a specific connector
  arivu search youtube \"rust tutorial\"    Search YouTube videos
  arivu hackernews search_stories \"rust\"  Search Hacker News directly

\x1b[1;36mAuthentication:\x1b[0m
  arivu setup                             Interactive setup wizard
  arivu setup slack                       Configure a specific connector
  arivu config show                       View current auth configuration
  arivu config test github                Test authentication

\x1b[1;36mMore Info:\x1b[0m
  arivu <command> --help                  Get help for any command
  https://github.com/srv1n/arivu          Full documentation")]
#[command(long_about = "
\x1b[1mArivu\x1b[0m - Unified Data Access CLI

Access 30+ data sources through a single interface:
  • Social: YouTube, Reddit, Hacker News, X/Twitter
  • Academic: arXiv, PubMed, Semantic Scholar
  • Productivity: Slack, GitHub, Atlassian, Microsoft 365, Google Workspace
  • Search: OpenAI, Anthropic, Perplexity, Exa, Tavily, Serper, and more

All connectors expose their capabilities as \x1b[1mtools\x1b[0m. Use `arivu tools` to see
what's available and their authentication requirements.
")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Launch interactive TUI mode
    #[arg(long, global = true)]
    pub tui: bool,

    /// Output format
    #[arg(long, global = true, value_enum, default_value_t = OutputFormat::Pretty)]
    pub output: OutputFormat,

    /// Disable colored output
    #[arg(long, global = true)]
    pub no_color: bool,

    /// Verbose output
    #[arg(short, long, global = true, action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// Copy output to clipboard
    #[arg(short, long, global = true)]
    pub copy: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    /// List all available connectors (data sources)
    ///
    /// Shows a table of all connectors with their descriptions and auth status.
    #[command(alias = "ls")]
    #[command(after_help = "\x1b[1;33mExamples:\x1b[0m
  arivu list                    Show all connectors
  arivu list --output json      Output as JSON")]
    List,

    /// Interactive setup wizard for configuring authentication
    ///
    /// Run without arguments for guided setup, or specify a connector name
    /// to configure it directly.
    #[command(alias = "init")]
    #[command(after_help = "\x1b[1;33mExamples:\x1b[0m
  arivu setup                   Start interactive wizard
  arivu setup slack             Configure Slack directly
  arivu setup github            Configure GitHub token")]
    Setup {
        /// Connector name to configure (omit for interactive wizard)
        connector: Option<String>,
    },

    /// Search for content across connectors
    ///
    /// Search a single connector or multiple connectors simultaneously using profiles.
    #[command(after_help = "\x1b[1;33mSingle Connector:\x1b[0m
  arivu search youtube \"rust programming\"
  arivu search hackernews \"async rust\" --limit 5
  arivu search arxiv \"machine learning\"

\x1b[1;33mFederated Search (Multiple Connectors):\x1b[0m
  arivu search \"CRISPR gene therapy\" --profile research
  arivu search \"release notes\" -s slack,confluence,google-drive
  arivu search \"attention mechanisms\" -p research --merge interleaved

\x1b[1;33mBuilt-in Profiles:\x1b[0m
  research    - pubmed, arxiv, semantic-scholar, google-scholar
  enterprise  - slack, atlassian, github
  social      - reddit, hackernews
  code        - github
  web         - perplexity, exa, tavily
  media       - youtube, wikipedia")]
    Search {
        /// The connector to use (e.g., youtube, reddit) OR the search query when using --profile/-s
        connector_or_query: String,
        /// The search query (optional when using --profile or -s, as first arg becomes the query)
        query: Option<String>,
        /// Maximum number of results per source
        #[arg(short, long, default_value_t = 10)]
        limit: u32,
        /// Search profile for federated search (research, enterprise, social, code, web)
        #[arg(short, long)]
        profile: Option<String>,
        /// Comma-separated list of connectors for ad-hoc federated search
        #[arg(short = 's', long = "sources")]
        connectors: Option<String>,
        /// Merge mode for federated results: grouped (default) or interleaved
        #[arg(short, long, default_value = "grouped")]
        merge: String,
        /// Add connectors to profile (use with --profile)
        #[arg(long)]
        add: Option<String>,
        /// Exclude connectors from profile (use with --profile)
        #[arg(long)]
        exclude: Option<String>,
    },

    /// Get specific content by ID
    ///
    /// Fetches detailed information for a specific resource.
    #[command(after_help = "\x1b[1;33mExamples:\x1b[0m
  arivu get youtube dQw4w9WgXcQ         Get video details + transcript
  arivu get hackernews 12345            Get HN story with comments
  arivu get arxiv 2301.07041            Get paper details")]
    Get {
        /// The connector to use
        connector: String,
        /// The resource ID or URL
        id: String,
    },

    /// Fetch content by automatically detecting the URL or ID type
    ///
    /// Paste any supported URL or ID and Arivu will route it to the right connector.
    #[command(alias = "f")]
    #[command(after_help = "\x1b[1;33mSupported Inputs:\x1b[0m
  YouTube:       https://youtube.com/watch?v=xxx, youtu.be/xxx, video ID
  Hacker News:   https://news.ycombinator.com/item?id=xxx, hn:12345678
  ArXiv:         https://arxiv.org/abs/xxx, arXiv:2301.07041
  PubMed:        https://pubmed.ncbi.nlm.nih.gov/xxx, PMID:12345678
  GitHub:        https://github.com/owner/repo, owner/repo
  Reddit:        https://reddit.com/r/xxx, r/rust
  X/Twitter:     https://x.com/user/status/xxx, @username
  Wikipedia:     https://en.wikipedia.org/wiki/xxx
  DOI:           https://doi.org/10.xxx, 10.1234/example
  Any URL:       Falls back to web scraper

\x1b[1;33mExamples:\x1b[0m
  arivu fetch https://www.youtube.com/watch?v=dQw4w9WgXcQ
  arivu fetch arXiv:2301.07041
  arivu fetch PMID:12345678
  arivu fetch rust-lang/rust
  arivu fetch r/rust")]
    Fetch {
        /// URL or ID to fetch (auto-detected)
        input: String,
    },

    /// Show all supported URL/ID patterns for auto-detection
    #[command(alias = "patterns")]
    Formats,

    /// Manage configuration and authentication
    ///
    /// Set, view, test, or remove authentication credentials for connectors.
    #[command(after_help = "\x1b[1;33mExamples:\x1b[0m
  arivu config show                     Show all saved credentials
  arivu config set slack --value xoxb-xxx
  arivu config set github --value ghp_xxx
  arivu config test slack               Test Slack authentication
  arivu config remove reddit            Remove Reddit credentials")]
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },

    /// Show detailed information about connectors
    ///
    /// Lists connectors with their tools, auth requirements, and examples.
    #[command(after_help = "\x1b[1;33mExamples:\x1b[0m
  arivu connectors              Show all connector details")]
    Connectors,

    /// List available tools with auth requirements
    ///
    /// Shows all tools across connectors, or tools for a specific connector.
    /// Each tool shows its parameters, whether auth is required, and examples.
    #[command(after_help = "\x1b[1;33mExamples:\x1b[0m
  arivu tools                   List ALL tools from all connectors
  arivu tools youtube           Show YouTube-specific tools
  arivu tools slack             Show Slack tools (requires auth)
  arivu tools --output json     Output as JSON for scripting")]
    Tools {
        /// Connector name to filter tools (omit to show all)
        connector: Option<String>,
    },

    /// Show pricing info for tools (if available)
    #[command(after_help = "\x1b[1;33mExamples:\x1b[0m
  arivu pricing                 List all pricing entries
  arivu pricing exa             Pricing for all Exa tools
  arivu pricing exa search      Pricing for Exa search tool
  arivu pricing openai-search search --model o4-mini")]
    Pricing {
        /// Connector name to filter (optional)
        connector: Option<String>,
        /// Tool name to filter (optional)
        tool: Option<String>,
        /// Filter by model (optional)
        #[arg(long)]
        model: Option<String>,
    },

    /// Show usage totals (overall or filtered)
    #[command(after_help = "\x1b[1;33mExamples:\x1b[0m
  arivu usage                   Show overall usage totals
  arivu usage --last            Show usage for the most recent run
  arivu usage --run run-123     Show usage for a specific run
  arivu usage exa search        Show usage for Exa search tool")]
    Usage {
        /// Connector name to filter (optional)
        connector: Option<String>,
        /// Tool name to filter (optional)
        tool: Option<String>,
        /// Filter by run id
        #[arg(long)]
        run: Option<String>,
        /// Show only the most recent run
        #[arg(long, conflicts_with = "run")]
        last: bool,
    },

    /// Call a tool directly from a connector
    ///
    /// You can use the simplified syntax: `arivu <connector> <tool> [args...]`
    /// The CLI automatically maps positional arguments to the tool's parameters.
    ///
    /// Advanced tool execution with JSON args. Prefer connector subcommands instead.
    #[command(after_help = "\x1b[1;33mExamples:\x1b[0m
  # Prefer connector subcommands (with proper flags):
  arivu youtube search --query \"rust\" --limit 10
  arivu hackernews top --limit 5
  arivu github search-repos --query \"rust cli\"

  # JSON args (for advanced/scripting use):
  arivu call youtube search_videos --args '{\"query\": \"rust\", \"limit\": 10}'
  arivu call github list_issues --args '{\"owner\": \"rust-lang\", \"repo\": \"rust\"}'

\x1b[1;36mTip:\x1b[0m Use `arivu <connector> --help` to see available subcommands.")]
    Call {
        /// Connector name (e.g., slack, github, youtube)
        connector: String,
        /// Tool name (e.g., list_channels, search_code)
        tool: String,
        /// JSON arguments (e.g., '{"query": "rust"}')
        #[arg(long, conflicts_with = "params")]
        args: Option<String>,
        /// Positional arguments for the tool (simplified syntax)
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        params: Vec<String>,
    },

    // ========================================================================
    // Connector-specific subcommands with proper CLI flags
    // ========================================================================
    /// OpenAI web search
    #[command(name = "openai-search")]
    #[command(after_help = "\x1b[1;33mExamples:\x1b[0m
  arivu openai-search search --query \"rust async programming\"
  arivu openai-search search --query \"AI news\" --max-results 10")]
    OpenaiSearch {
        #[command(subcommand)]
        tool: OpenaiSearchTools,
    },

    /// Anthropic web search
    #[command(name = "anthropic-search")]
    #[command(after_help = "\x1b[1;33mExamples:\x1b[0m
  arivu anthropic-search search --query \"rust async programming\"
  arivu anthropic-search search --query \"AI news\" --max-results 10")]
    AnthropicSearch {
        #[command(subcommand)]
        tool: AnthropicSearchTools,
    },

    /// Gemini web search
    #[command(name = "gemini-search")]
    #[command(after_help = "\x1b[1;33mExamples:\x1b[0m
  arivu gemini-search search --query \"rust async programming\"
  arivu gemini-search search --query \"AI news\" --max-results 10")]
    GeminiSearch {
        #[command(subcommand)]
        tool: GeminiSearchTools,
    },

    /// Perplexity web search
    #[command(name = "perplexity-search")]
    #[command(after_help = "\x1b[1;33mExamples:\x1b[0m
  arivu perplexity-search search --query \"rust async programming\"
  arivu perplexity-search search --query \"AI news\" --max-results 10")]
    PerplexitySearch {
        #[command(subcommand)]
        tool: PerplexitySearchTools,
    },

    /// xAI web search
    #[command(name = "xai-search")]
    #[command(after_help = "\x1b[1;33mExamples:\x1b[0m
  arivu xai-search search --query \"rust async programming\"
  arivu xai-search search --query \"AI news\" --max-results 10")]
    XaiSearch {
        #[command(subcommand)]
        tool: XaiSearchTools,
    },

    /// Exa neural search
    #[command(name = "exa")]
    #[command(after_help = "\x1b[1;33mExamples:\x1b[0m
  arivu exa search --query \"rust async programming\" --num-results 10
  arivu exa find-similar --url https://example.com
  arivu exa answer --query \"What is Rust?\"
  arivu exa get-contents --ids url1,url2")]
    Exa {
        #[command(subcommand)]
        tool: ExaTools,
    },

    /// Tavily web search
    #[command(name = "tavily-search")]
    #[command(after_help = "\x1b[1;33mExamples:\x1b[0m
  arivu tavily-search search --query \"rust async programming\"
  arivu tavily-search search --query \"AI news\" --max-results 10 --depth advanced")]
    TavilySearch {
        #[command(subcommand)]
        tool: TavilySearchTools,
    },

    /// Serper web search
    #[command(name = "serper-search")]
    #[command(after_help = "\x1b[1;33mExamples:\x1b[0m
  arivu serper-search search --query \"rust async programming\"
  arivu serper-search search --query \"AI news\" --max-results 10")]
    SerperSearch {
        #[command(subcommand)]
        tool: SerperSearchTools,
    },

    /// SerpAPI web search
    #[command(name = "serpapi-search")]
    #[command(after_help = "\x1b[1;33mExamples:\x1b[0m
  arivu serpapi-search search --query \"rust async programming\"
  arivu serpapi-search search --query \"AI news\" --max-results 10 --engine google")]
    SerpapiSearch {
        #[command(subcommand)]
        tool: SerpapiSearchTools,
    },

    /// Firecrawl search and scraping
    #[command(name = "firecrawl-search")]
    #[command(after_help = "\x1b[1;33mExamples:\x1b[0m
  arivu firecrawl-search search --query \"rust async programming\"
  arivu firecrawl-search search --query \"AI news\" --scrape false")]
    FirecrawlSearch {
        #[command(subcommand)]
        tool: FirecrawlSearchTools,
    },

    /// Parallel AI web search
    #[command(name = "parallel-search")]
    #[command(after_help = "\x1b[1;33mExamples:\x1b[0m
  arivu parallel-search search --query \"rust async programming\"
  arivu parallel-search search --query \"AI news\" --max-results 10")]
    ParallelSearch {
        #[command(subcommand)]
        tool: ParallelSearchTools,
    },

    /// Google Calendar events and management
    #[command(name = "google-calendar", alias = "gcal")]
    #[command(after_help = "\x1b[1;33mExamples:\x1b[0m
  arivu google-calendar list-events
  arivu google-calendar create-event --summary \"Meeting\" --start \"2025-01-01T10:00:00Z\" --end \"2025-01-01T11:00:00Z\"
  arivu google-calendar update-event --event-id abc123 --summary \"Updated Meeting\"
  arivu google-calendar delete-event --event-id abc123")]
    GoogleCalendar {
        #[command(subcommand)]
        tool: GoogleCalendarTools,
    },

    /// Google Drive file management
    #[command(name = "google-drive", alias = "gdrive")]
    #[command(after_help = "\x1b[1;33mExamples:\x1b[0m
  arivu google-drive list-files
  arivu google-drive get-file --file-id abc123
  arivu google-drive download-file --file-id abc123
  arivu google-drive export-file --file-id abc123 --mime-type application/pdf")]
    GoogleDrive {
        #[command(subcommand)]
        tool: GoogleDriveTools,
    },

    /// Google Gmail messages and threads
    #[command(name = "google-gmail", alias = "gmail")]
    #[command(after_help = "\x1b[1;33mExamples:\x1b[0m
  arivu google-gmail list-messages
  arivu google-gmail list-messages --q \"from:example@gmail.com\"
  arivu google-gmail get-message --id abc123
  arivu google-gmail get-thread --id abc123")]
    GoogleGmail {
        #[command(subcommand)]
        tool: GoogleGmailTools,
    },

    /// Google People contacts
    #[command(name = "google-people", alias = "gpeople")]
    #[command(after_help = "\x1b[1;33mExamples:\x1b[0m
  arivu google-people list-connections
  arivu google-people get-person --resource-name people/c123")]
    GooglePeople {
        #[command(subcommand)]
        tool: GooglePeopleTools,
    },

    /// Google Scholar paper search
    #[command(name = "google-scholar", alias = "gscholar")]
    #[command(after_help = "\x1b[1;33mExamples:\x1b[0m
  arivu google-scholar search-papers --query \"CRISPR gene therapy\"
  arivu google-scholar search-papers --query \"machine learning\" --limit 20")]
    GoogleScholar {
        #[command(subcommand)]
        tool: GoogleScholarTools,
    },

    /// Atlassian (Jira + Confluence)
    #[command(name = "atlassian", alias = "jira")]
    #[command(after_help = "\x1b[1;33mExamples:\x1b[0m
  arivu atlassian test-auth
  arivu atlassian jira-search --jql \"project = DEMO AND status = Open\"
  arivu atlassian jira-get --key DEMO-123
  arivu atlassian conf-search --cql \"type = page AND space = DEMO\"
  arivu atlassian conf-get --id 123456")]
    Atlassian {
        #[command(subcommand)]
        tool: AtlassianTools,
    },

    /// Microsoft Graph (Microsoft 365)
    #[command(name = "microsoft-graph", alias = "msgraph")]
    #[command(after_help = "\x1b[1;33mExamples:\x1b[0m
  arivu microsoft-graph list-messages --top 20
  arivu microsoft-graph list-events --days-ahead 7
  arivu microsoft-graph get-message --message-id ABC123
  arivu microsoft-graph send-mail --to user@example.com --subject \"Hello\" --body \"Test\"
  arivu microsoft-graph create-draft --to user@example.com --subject \"Draft\" --body \"Draft message\"")]
    MicrosoftGraph {
        #[command(subcommand)]
        tool: MicrosoftGraphTools,
    },

    /// IMAP email
    #[command(name = "imap", alias = "email")]
    #[command(after_help = "\x1b[1;33mExamples:\x1b[0m
  arivu imap list-mailboxes
  arivu imap fetch-messages --limit 20
  arivu imap get-message --uid 12345
  arivu imap search --query \"UNSEEN\"
  arivu imap search --query \"FROM alice SINCE 1-Jan-2024\"")]
    Imap {
        #[command(subcommand)]
        tool: ImapTools,
    },

    /// Local filesystem text extraction (PDF, EPUB, DOCX, HTML, Markdown, code)
    #[command(name = "localfs", alias = "fs", alias = "file")]
    #[command(after_help = "\x1b[1;33mExamples:\x1b[0m
  arivu localfs list-files --path ~/Documents --recursive --extensions pdf,md
  arivu localfs extract-text --path ~/paper.pdf
  arivu localfs structure --path ~/book.epub")]
    Localfs {
        #[command(subcommand)]
        tool: LocalfsTools,
    },

    /// YouTube video details, transcripts, and search
    #[command(name = "youtube", alias = "yt")]
    #[command(after_help = "\x1b[1;33mExamples:\x1b[0m
  arivu youtube search --query \"rust programming\" --limit 10
  arivu youtube video --id dQw4w9WgXcQ")]
    Youtube {
        #[command(subcommand)]
        tool: YoutubeTools,
    },

    /// Hacker News stories, comments, and search
    #[command(name = "hackernews", alias = "hn")]
    #[command(after_help = "\x1b[1;33mExamples:\x1b[0m
  arivu hackernews search --query \"rust\" --limit 10
  arivu hackernews story --id 12345678")]
    Hackernews {
        #[command(subcommand)]
        tool: HackernewsTools,
    },

    /// arXiv paper search and retrieval
    #[command(name = "arxiv")]
    #[command(after_help = "\x1b[1;33mExamples:\x1b[0m
  arivu arxiv search --query \"transformer architecture\" --limit 10
  arivu arxiv paper --id 2301.07041")]
    Arxiv {
        #[command(subcommand)]
        tool: ArxivTools,
    },

    /// GitHub repositories, issues, PRs, and code search
    #[command(name = "github", alias = "gh")]
    #[command(after_help = "\x1b[1;33mExamples:\x1b[0m
  arivu github search-repos --query \"rust cli\"
  arivu github search-code --query \"async fn\" --repo tokio-rs/tokio")]
    Github {
        #[command(subcommand)]
        tool: GithubTools,
    },

    /// Reddit posts, comments, and subreddit search
    #[command(name = "reddit")]
    #[command(after_help = "\x1b[1;33mExamples:\x1b[0m
  arivu reddit search --query \"rust\" --subreddit programming
  arivu reddit hot --subreddit rust --limit 20")]
    Reddit {
        #[command(subcommand)]
        tool: RedditTools,
    },

    /// Web page scraping and content extraction
    #[command(name = "web")]
    #[command(after_help = "\x1b[1;33mExamples:\x1b[0m
  arivu web scrape --url https://example.com")]
    Web {
        #[command(subcommand)]
        tool: WebTools,
    },

    /// Wikipedia article search and retrieval
    #[command(name = "wikipedia", alias = "wiki")]
    #[command(after_help = "\x1b[1;33mExamples:\x1b[0m
  arivu wikipedia search --query \"Rust programming\"
  arivu wikipedia article --title \"Rust (programming language)\"")]
    Wikipedia {
        #[command(subcommand)]
        tool: WikipediaTools,
    },

    /// PubMed medical literature search
    #[command(name = "pubmed")]
    #[command(after_help = "\x1b[1;33mExamples:\x1b[0m
  arivu pubmed search --query \"CRISPR gene therapy\" --limit 10
  arivu pubmed article --pmid 12345678")]
    Pubmed {
        #[command(subcommand)]
        tool: PubmedTools,
    },

    /// Semantic Scholar academic paper search
    #[command(name = "semantic-scholar", alias = "scholar")]
    #[command(after_help = "\x1b[1;33mExamples:\x1b[0m
  arivu semantic-scholar search --query \"attention mechanism\" --limit 10
  arivu semantic-scholar paper --id abc123")]
    SemanticScholar {
        #[command(subcommand)]
        tool: SemanticScholarTools,
    },

    /// Slack channels, messages, and search
    #[command(name = "slack")]
    #[command(after_help = "\x1b[1;33mExamples:\x1b[0m
  arivu slack channels
  arivu slack messages --channel general --limit 50")]
    Slack {
        #[command(subcommand)]
        tool: SlackTools,
    },

    /// X (Twitter) profiles, tweets, and search
    #[command(name = "x", alias = "twitter")]
    #[command(after_help = "\x1b[1;33mExamples:\x1b[0m
  arivu x profile --username elonmusk
  arivu x search --query \"rust lang\" --limit 20")]
    X {
        #[command(subcommand)]
        tool: XTools,
    },

    /// Discord servers, channels, and messages
    #[command(name = "discord")]
    #[command(after_help = "\x1b[1;33mExamples:\x1b[0m
  arivu discord servers
  arivu discord channels --guild-id 123456789")]
    Discord {
        #[command(subcommand)]
        tool: DiscordTools,
    },

    /// RSS feed reader
    #[command(name = "rss")]
    #[command(after_help = "\x1b[1;33mExamples:\x1b[0m
  arivu rss feed --url https://example.com/feed.xml
  arivu rss entries --url https://example.com/feed.xml --limit 20")]
    Rss {
        #[command(subcommand)]
        tool: RssTools,
    },

    /// bioRxiv and medRxiv preprint search
    #[command(name = "biorxiv")]
    #[command(after_help = "\x1b[1;33mExamples:\x1b[0m
  arivu biorxiv recent --server biorxiv --count 20
  arivu biorxiv paper --server biorxiv --doi 10.1101/2024.01.01.123456")]
    Biorxiv {
        #[command(subcommand)]
        tool: BiorxivTools,
    },

    /// Sci-Hub paper access
    #[command(name = "scihub")]
    #[command(after_help = "\x1b[1;33mExamples:\x1b[0m
  arivu scihub paper --doi 10.1038/nature12373")]
    Scihub {
        #[command(subcommand)]
        tool: ScihubTools,
    },

    /// macOS automation and scripting
    #[command(name = "macos")]
    #[command(after_help = "\x1b[1;33mExamples:\x1b[0m
  arivu macos script --script \"display dialog \\\"Hello\\\"\"
  arivu macos notify --message \"Task complete\"")]
    Macos {
        #[command(subcommand)]
        tool: MacosTools,
    },

    /// Spotlight file search
    #[command(name = "spotlight")]
    #[command(after_help = "\x1b[1;33mExamples:\x1b[0m
  arivu spotlight search --query \"rust async\"
  arivu spotlight name --name \"cargo.toml\"")]
    Spotlight {
        #[command(subcommand)]
        tool: SpotlightTools,
    },
}

#[derive(Subcommand, Clone)]
pub enum ConfigAction {
    /// Show current configuration
    Show,
    /// Set authentication for a connector
    Set {
        /// Connector name
        connector: String,
        /// Authentication method (api-key, browser, oauth)
        #[arg(long)]
        auth_type: Option<String>,
        /// API key or credential value
        #[arg(long)]
        value: Option<String>,
        /// Browser to extract cookies from (chrome, firefox, safari, brave)
        #[arg(long)]
        browser: Option<String>,
    },
    /// Remove authentication for a connector
    Remove {
        /// Connector name
        connector: String,
    },
    /// Test authentication for a connector
    Test {
        /// Connector name
        connector: String,
    },
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub enum OutputFormat {
    /// Human-readable formatted output
    Pretty,
    /// JSON output
    Json,
    /// YAML output
    Yaml,
    /// Plain text output
    Text,
    /// Markdown output
    Markdown,
}

// ============================================================================
// Connector-specific tool enums with proper CLI flags
// ============================================================================

/// OpenAI Search tools
#[derive(Subcommand, Clone)]
pub enum OpenaiSearchTools {
    /// Search the web using OpenAI with grounding
    #[command(name = "search")]
    Search {
        /// Search query
        #[arg(long, short)]
        query: String,
        /// Maximum number of sources to cite
        #[arg(long, default_value_t = 5)]
        max_results: u32,
        /// Model name (e.g., o4-mini, gpt-4.1)
        #[arg(long)]
        model: Option<String>,
        /// Response format: concise or detailed
        #[arg(long, default_value = "concise")]
        response_format: String,
    },
}

/// Anthropic Search tools
#[derive(Subcommand, Clone)]
pub enum AnthropicSearchTools {
    /// Search the web using Claude with grounding
    #[command(name = "search")]
    Search {
        /// Search query
        #[arg(long, short)]
        query: String,
        /// Maximum number of sources to cite
        #[arg(long, default_value_t = 5)]
        max_results: u32,
        /// Model name (e.g., claude-3-7-sonnet-latest)
        #[arg(long)]
        model: Option<String>,
        /// Response format: concise or detailed
        #[arg(long, default_value = "concise")]
        response_format: String,
    },
}

/// Gemini Search tools
#[derive(Subcommand, Clone)]
pub enum GeminiSearchTools {
    /// Search the web using Gemini with grounding
    #[command(name = "search")]
    Search {
        /// Search query
        #[arg(long, short)]
        query: String,
        /// Maximum number of sources to cite
        #[arg(long, default_value_t = 5)]
        max_results: u32,
        /// Model name (e.g., gemini-1.5-pro-latest)
        #[arg(long)]
        model: Option<String>,
        /// Response format: concise or detailed
        #[arg(long, default_value = "concise")]
        response_format: String,
    },
}

/// Perplexity Search tools
#[derive(Subcommand, Clone)]
pub enum PerplexitySearchTools {
    /// Search the web using Perplexity with grounding
    #[command(name = "search")]
    Search {
        /// Search query
        #[arg(long, short)]
        query: String,
        /// Maximum number of sources to cite
        #[arg(long, default_value_t = 5)]
        max_results: u32,
        /// Model name (e.g., sonar-pro)
        #[arg(long)]
        model: Option<String>,
        /// Response format: concise or detailed
        #[arg(long, default_value = "concise")]
        response_format: String,
    },
}

/// xAI Search tools
#[derive(Subcommand, Clone)]
pub enum XaiSearchTools {
    /// Search the web and X using xAI with grounding
    #[command(name = "search")]
    Search {
        /// Search query
        #[arg(long, short)]
        query: String,
        /// Maximum number of sources to cite
        #[arg(long, default_value_t = 5)]
        max_results: u32,
        /// Model name (e.g., grok-4-fast)
        #[arg(long)]
        model: Option<String>,
        /// Response format: concise or detailed
        #[arg(long, default_value = "concise")]
        response_format: String,
    },
}

/// Exa tools
#[derive(Subcommand, Clone)]
pub enum ExaTools {
    /// Neural search using embeddings
    #[command(name = "search")]
    Search {
        /// Search query
        #[arg(long, short)]
        query: String,
        /// Number of results
        #[arg(long, default_value_t = 10)]
        num_results: u32,
        /// Search type: auto, fast, or deep
        #[arg(long, default_value = "auto")]
        type_: String,
        /// Response format: concise or detailed
        #[arg(long, default_value = "concise")]
        response_format: String,
    },

    /// Get clean parsed content from URLs
    #[command(name = "get-contents")]
    GetContents {
        /// Comma-separated list of URLs or Exa result IDs
        #[arg(long, short)]
        ids: String,
    },

    /// Find similar pages to a URL
    #[command(name = "find-similar")]
    FindSimilar {
        /// URL to find similar pages for
        #[arg(long, short)]
        url: String,
        /// Number of results
        #[arg(long, default_value_t = 10)]
        num_results: u32,
    },

    /// Get LLM-generated answer with citations
    #[command(name = "answer")]
    Answer {
        /// Question to answer
        #[arg(long, short)]
        query: String,
        /// Answer mode: precise or detailed
        #[arg(long)]
        mode: Option<String>,
    },
}

/// Tavily Search tools
#[derive(Subcommand, Clone)]
pub enum TavilySearchTools {
    /// Search the web using Tavily
    #[command(name = "search")]
    Search {
        /// Search query
        #[arg(long, short)]
        query: String,
        /// Maximum number of results
        #[arg(long, default_value_t = 10)]
        max_results: u32,
        /// Search depth: basic or advanced
        #[arg(long, default_value = "basic")]
        depth: String,
        /// Response format: concise or detailed
        #[arg(long, default_value = "concise")]
        response_format: String,
    },
}

/// Serper Search tools
#[derive(Subcommand, Clone)]
pub enum SerperSearchTools {
    /// Search Google via Serper.dev
    #[command(name = "search")]
    Search {
        /// Search query
        #[arg(long, short)]
        query: String,
        /// Maximum number of results
        #[arg(long, default_value_t = 10)]
        max_results: u32,
        /// Response format: concise or detailed
        #[arg(long, default_value = "concise")]
        response_format: String,
    },
}

/// SerpAPI Search tools
#[derive(Subcommand, Clone)]
pub enum SerpapiSearchTools {
    /// Search Google via SerpAPI
    #[command(name = "search")]
    Search {
        /// Search query
        #[arg(long, short)]
        query: String,
        /// Maximum number of results
        #[arg(long, default_value_t = 10)]
        max_results: u32,
        /// Search engine: google, bing, etc.
        #[arg(long, default_value = "google")]
        engine: String,
        /// Response format: concise or detailed
        #[arg(long, default_value = "concise")]
        response_format: String,
    },
}

/// Firecrawl Search tools
#[derive(Subcommand, Clone)]
pub enum FirecrawlSearchTools {
    /// Search and scrape the web using Firecrawl
    #[command(name = "search")]
    Search {
        /// Search query
        #[arg(long, short)]
        query: String,
        /// Maximum number of results
        #[arg(long, default_value_t = 10)]
        max_results: u32,
        /// Whether to scrape and parse content
        #[arg(long, default_value_t = true)]
        scrape: bool,
        /// Response format: concise or detailed
        #[arg(long, default_value = "concise")]
        response_format: String,
    },
}

/// Parallel Search tools
#[derive(Subcommand, Clone)]
pub enum ParallelSearchTools {
    /// Search the web using Parallel AI
    #[command(name = "search")]
    Search {
        /// Search query or objective
        #[arg(long, short)]
        query: String,
        /// Maximum number of results
        #[arg(long, default_value_t = 10)]
        max_results: u32,
    },
}

// ============================================================================
// Google Connector tools with proper CLI flags
// ============================================================================

/// Google Calendar tools
#[derive(Subcommand, Clone)]
pub enum GoogleCalendarTools {
    /// List upcoming events from primary calendar
    #[command(name = "list-events", alias = "events")]
    ListEvents {
        /// Maximum number of results
        #[arg(long, short, default_value_t = 10)]
        max_results: u32,
        /// Minimum time (RFC3339 format)
        #[arg(long)]
        time_min: Option<String>,
        /// Response format (concise or full)
        #[arg(long, default_value = "concise")]
        response_format: String,
    },

    /// Create an event in primary calendar
    #[command(name = "create-event", alias = "create")]
    CreateEvent {
        /// Event title/summary
        #[arg(long, short)]
        summary: String,
        /// Start time (RFC3339 format)
        #[arg(long)]
        start: String,
        /// End time (RFC3339 format)
        #[arg(long)]
        end: String,
    },

    /// Incremental sync using syncToken
    #[command(name = "sync-events", alias = "sync")]
    SyncEvents {
        /// Sync token from previous sync
        #[arg(long, short)]
        sync_token: String,
        /// Maximum number of results
        #[arg(long, short, default_value_t = 10)]
        max_results: u32,
    },

    /// Update an event in primary calendar
    #[command(name = "update-event", alias = "update")]
    UpdateEvent {
        /// Event ID
        #[arg(long, short)]
        event_id: String,
        /// New event title/summary
        #[arg(long)]
        summary: Option<String>,
        /// New start time (RFC3339 format)
        #[arg(long)]
        start: Option<String>,
        /// New end time (RFC3339 format)
        #[arg(long)]
        end: Option<String>,
    },

    /// Delete an event in primary calendar
    #[command(name = "delete-event", alias = "delete")]
    DeleteEvent {
        /// Event ID
        #[arg(long, short)]
        event_id: Option<String>,
    },
}

/// Google Drive tools
#[derive(Subcommand, Clone)]
pub enum GoogleDriveTools {
    /// List files in Drive
    #[command(name = "list-files", alias = "list")]
    ListFiles {
        /// Drive query string
        #[arg(long, short)]
        q: Option<String>,
        /// Maximum number of results
        #[arg(long, short, default_value_t = 10)]
        page_size: u32,
        /// Response format (concise or full)
        #[arg(long, default_value = "concise")]
        response_format: String,
    },

    /// Get file metadata by ID
    #[command(name = "get-file", alias = "get")]
    GetFile {
        /// File ID
        #[arg(long, short)]
        file_id: String,
        /// Response format (concise or full)
        #[arg(long, default_value = "concise")]
        response_format: String,
    },

    /// Download file content by ID
    #[command(name = "download-file", alias = "download")]
    DownloadFile {
        /// File ID
        #[arg(long, short)]
        file_id: String,
        /// Maximum bytes to download
        #[arg(long)]
        max_bytes: Option<u64>,
    },

    /// Export Google Docs/Sheets/Slides to target MIME type
    #[command(name = "export-file", alias = "export")]
    ExportFile {
        /// File ID
        #[arg(long, short)]
        file_id: String,
        /// Target MIME type (e.g., application/pdf, text/csv)
        #[arg(long, short)]
        mime_type: String,
    },

    /// Upload a small file via base64
    #[command(name = "upload-file", alias = "upload")]
    UploadFile {
        /// File name
        #[arg(long, short)]
        name: String,
        /// MIME type
        #[arg(long, short)]
        mime_type: String,
        /// Base64 encoded data
        #[arg(long, short)]
        data_base64: String,
        /// Parent folder IDs (comma-separated)
        #[arg(long)]
        parents: Option<String>,
    },

    /// Resumable upload via temp file
    #[command(name = "upload-file-resumable", alias = "upload-resumable")]
    UploadFileResumable {
        /// File name
        #[arg(long, short)]
        name: String,
        /// MIME type
        #[arg(long, short)]
        mime_type: String,
        /// Base64 encoded data
        #[arg(long, short)]
        data_base64: String,
        /// Parent folder IDs (comma-separated)
        #[arg(long)]
        parents: Option<String>,
    },
}

/// Google Gmail tools
#[derive(Subcommand, Clone)]
pub enum GoogleGmailTools {
    /// List messages in mailbox
    #[command(name = "list-messages", alias = "list")]
    ListMessages {
        /// Gmail query string
        #[arg(long, short)]
        q: Option<String>,
        /// Maximum number of results
        #[arg(long, short, default_value_t = 10)]
        max_results: u32,
        /// Response format (concise or full)
        #[arg(long, default_value = "concise")]
        response_format: String,
    },

    /// Decode a Gmail raw message
    #[command(name = "decode-message-raw", alias = "decode")]
    DecodeMessageRaw {
        /// Base64url encoded raw message
        #[arg(long, short)]
        raw_base64url: String,
    },

    /// Get a message by ID
    #[command(name = "get-message", alias = "get")]
    GetMessage {
        /// Message ID
        #[arg(long, short)]
        id: String,
        /// Format (raw, full, metadata)
        #[arg(long, short, default_value = "full")]
        format: String,
        /// Response format (concise or full)
        #[arg(long, default_value = "concise")]
        response_format: String,
    },

    /// Get a thread by ID
    #[command(name = "get-thread", alias = "thread")]
    GetThread {
        /// Thread ID
        #[arg(long, short)]
        id: String,
    },
}

/// Google People tools
#[derive(Subcommand, Clone)]
pub enum GooglePeopleTools {
    /// List contacts
    #[command(name = "list-connections", alias = "list")]
    ListConnections {
        /// Maximum number of results
        #[arg(long, short, default_value_t = 10)]
        page_size: u32,
        /// Response format (concise or full)
        #[arg(long, default_value = "concise")]
        response_format: String,
    },

    /// Get a person by resourceName
    #[command(name = "get-person", alias = "get")]
    GetPerson {
        /// Resource name (e.g., people/c123)
        #[arg(long, short)]
        resource_name: String,
        /// Comma-separated person fields (e.g., names,emailAddresses,phoneNumbers)
        #[arg(long, short)]
        person_fields: Option<String>,
        /// Response format (concise or full)
        #[arg(long, default_value = "concise")]
        response_format: String,
    },
}

/// Google Scholar tools
#[derive(Subcommand, Clone)]
pub enum GoogleScholarTools {
    /// Search for papers on Google Scholar
    #[command(name = "search-papers", alias = "search")]
    SearchPapers {
        /// Search query
        #[arg(long, short)]
        query: String,
        /// Maximum number of results
        #[arg(long, short, default_value_t = 10)]
        limit: u32,
    },
}

// ============================================================================
// Productivity connector tools
// ============================================================================

/// Atlassian tools (Jira + Confluence)
#[derive(Subcommand, Clone)]
pub enum AtlassianTools {
    /// Test authentication
    #[command(name = "test-auth")]
    TestAuth,

    /// Search Jira issues with JQL
    #[command(name = "jira-search", alias = "jira")]
    JiraSearch {
        /// JQL query
        #[arg(long, short)]
        jql: String,
        /// Starting index
        #[arg(long, default_value_t = 0)]
        start_at: u32,
        /// Maximum results
        #[arg(long, short, default_value_t = 50)]
        max_results: u32,
        /// Comma-separated list of fields to return
        #[arg(long, short)]
        fields: Option<String>,
    },

    /// Get a Jira issue by key
    #[command(name = "jira-get", alias = "issue")]
    JiraGet {
        /// Issue key (e.g., PROJ-123)
        #[arg(long, short)]
        key: String,
        /// Expand options (comma-separated)
        #[arg(long, short)]
        expand: Option<String>,
    },

    /// Search Confluence pages with CQL
    #[command(name = "conf-search", alias = "confluence")]
    ConfSearch {
        /// CQL query
        #[arg(long, short)]
        cql: String,
        /// Starting index
        #[arg(long, default_value_t = 0)]
        start: u32,
        /// Maximum results
        #[arg(long, short, default_value_t = 25)]
        limit: u32,
    },

    /// Get a Confluence page by ID
    #[command(name = "conf-get", alias = "page")]
    ConfGet {
        /// Page ID
        #[arg(long, short)]
        id: String,
        /// Expand options (comma-separated)
        #[arg(long, short)]
        expand: Option<String>,
    },
}

/// Microsoft Graph tools (Microsoft 365)
#[derive(Subcommand, Clone)]
pub enum MicrosoftGraphTools {
    /// List recent Outlook messages
    #[command(name = "list-messages", alias = "messages")]
    ListMessages {
        /// Maximum messages (1-50)
        #[arg(long, short, default_value_t = 20)]
        top: u32,
        /// Response format: concise or detailed
        #[arg(long, default_value = "concise")]
        response_format: String,
    },

    /// List upcoming calendar events
    #[command(name = "list-events", alias = "events")]
    ListEvents {
        /// Window in days
        #[arg(long, short, default_value_t = 7)]
        days_ahead: u32,
        /// Response format: concise or detailed
        #[arg(long, default_value = "concise")]
        response_format: String,
    },

    /// Get a message by ID
    #[command(name = "get-message", alias = "message")]
    GetMessage {
        /// Message ID
        #[arg(long, short)]
        message_id: String,
    },

    /// Send a simple email
    #[command(name = "send-mail", alias = "send")]
    SendMail {
        /// Recipient email addresses (comma-separated)
        #[arg(long, short)]
        to: String,
        /// Email subject
        #[arg(long, short)]
        subject: String,
        /// Email body text
        #[arg(long, short)]
        body: String,
    },

    /// Create a draft message
    #[command(name = "create-draft", alias = "draft")]
    CreateDraft {
        /// Recipient email addresses (comma-separated)
        #[arg(long, short)]
        to: String,
        /// Email subject
        #[arg(long, short)]
        subject: String,
        /// Email body text
        #[arg(long, short)]
        body: String,
    },

    /// Upload a large attachment to a draft
    #[command(name = "upload-attachment")]
    UploadAttachment {
        /// Message ID
        #[arg(long, short)]
        message_id: String,
        /// Filename
        #[arg(long, short)]
        filename: String,
        /// MIME type
        #[arg(long, short)]
        mime_type: String,
        /// Base64-encoded data
        #[arg(long, short)]
        data_base64: String,
    },

    /// Send a draft message
    #[command(name = "send-draft")]
    SendDraft {
        /// Message ID
        #[arg(long, short)]
        message_id: String,
    },

    /// Upload attachment from file path
    #[command(name = "upload-attachment-from-path", alias = "upload-file")]
    UploadAttachmentFromPath {
        /// Message ID
        #[arg(long, short)]
        message_id: String,
        /// File path
        #[arg(long, short)]
        file_path: String,
        /// Filename (optional, inferred from path if not provided)
        #[arg(long, short)]
        filename: Option<String>,
        /// MIME type (optional, inferred if not provided)
        #[arg(long, short)]
        mime_type: Option<String>,
    },

    /// Start device authorization flow
    #[command(name = "auth-start")]
    AuthStart {
        /// Tenant ID
        #[arg(long)]
        tenant_id: Option<String>,
        /// Client ID
        #[arg(long)]
        client_id: Option<String>,
        /// Scopes (space-separated)
        #[arg(long)]
        scopes: Option<String>,
    },

    /// Poll token endpoint for device flow
    #[command(name = "auth-poll")]
    AuthPoll {
        /// Tenant ID
        #[arg(long)]
        tenant_id: Option<String>,
        /// Client ID
        #[arg(long, short)]
        client_id: String,
        /// Device code
        #[arg(long, short)]
        device_code: String,
    },
}

/// IMAP email tools
#[derive(Subcommand, Clone)]
pub enum ImapTools {
    /// List mailboxes on the IMAP server
    #[command(name = "list-mailboxes", alias = "mailboxes")]
    ListMailboxes {
        /// IMAP reference name
        #[arg(long, short)]
        reference: Option<String>,
        /// Mailbox pattern (e.g., *)
        #[arg(long, short, default_value = "*")]
        pattern: String,
        /// Include subscription information
        #[arg(long)]
        include_subscribed: bool,
    },

    /// Fetch recent message summaries
    #[command(name = "fetch-messages", alias = "messages")]
    FetchMessages {
        /// Mailbox name
        #[arg(long, short)]
        mailbox: Option<String>,
        /// Maximum number of messages
        #[arg(long, short, default_value_t = 20)]
        limit: u32,
    },

    /// Get a full message by UID
    #[command(name = "get-message", alias = "message")]
    GetMessage {
        /// Mailbox name
        #[arg(long, short)]
        mailbox: Option<String>,
        /// Message UID
        #[arg(long, short)]
        uid: u32,
        /// Include base64 encoded raw message
        #[arg(long)]
        include_raw: bool,
    },

    /// Search messages in a mailbox
    #[command(name = "search")]
    Search {
        /// Mailbox to search
        #[arg(long, short)]
        mailbox: Option<String>,
        /// IMAP search query (e.g., 'UNSEEN', 'FROM "alice"')
        #[arg(long, short)]
        query: String,
        /// Maximum number of UIDs to return
        #[arg(long, short, default_value_t = 50)]
        limit: u32,
    },
}

/// Local filesystem tools for text extraction from documents
#[derive(Subcommand, Clone)]
pub enum LocalfsTools {
    /// List files in a directory
    #[command(name = "list-files", alias = "ls")]
    ListFiles {
        /// Directory path to list
        #[arg(long, short)]
        path: String,
        /// Recurse into subdirectories
        #[arg(long, short, default_value_t = false)]
        recursive: bool,
        /// Comma-separated list of extensions to filter (e.g., "pdf,md,txt")
        #[arg(long, short)]
        extensions: Option<String>,
        /// Maximum number of files to return
        #[arg(long, short, default_value_t = 100)]
        limit: u32,
    },

    /// Get metadata about a file
    #[command(name = "file-info", alias = "info")]
    FileInfo {
        /// File path
        #[arg(long, short)]
        path: String,
    },

    /// Extract all text from a file (PDF, EPUB, DOCX, HTML, Markdown, code, text)
    #[command(name = "extract-text", alias = "extract", alias = "read")]
    ExtractText {
        /// File path
        #[arg(long, short)]
        path: String,
        /// Output format: plain or markdown
        #[arg(long, short, default_value = "plain")]
        format: String,
    },

    /// Get document structure (table of contents, headings, chapters)
    #[command(name = "structure", alias = "toc")]
    Structure {
        /// File path
        #[arg(long, short)]
        path: String,
    },

    /// Get a specific section from a document
    #[command(name = "section", alias = "get-section")]
    Section {
        /// File path
        #[arg(long, short)]
        path: String,
        /// Section identifier (e.g., "page:5", "chapter:3", "heading:2", "lines:10-50")
        #[arg(long, short)]
        section: String,
    },

    /// Search within a file
    #[command(name = "search", alias = "grep")]
    Search {
        /// File path
        #[arg(long, short)]
        path: String,
        /// Search query
        #[arg(long, short)]
        query: String,
        /// Lines of context around matches
        #[arg(long, short, default_value_t = 2)]
        context: u32,
    },
}

/// YouTube tools
#[derive(Subcommand, Clone)]
pub enum YoutubeTools {
    /// Search for videos
    #[command(name = "search")]
    Search {
        /// Search query
        #[arg(long, short)]
        query: String,
        /// Maximum number of results
        #[arg(long, short, default_value_t = 10)]
        limit: u32,
    },

    /// Get video details
    #[command(name = "video", alias = "get")]
    Video {
        /// Video ID or URL
        #[arg(long, short)]
        id: String,
    },

    /// Get video transcript
    #[command(name = "transcript", alias = "captions")]
    Transcript {
        /// Video ID or URL
        #[arg(long, short)]
        id: String,
        /// Language code (e.g., "en", "es")
        #[arg(long, short)]
        lang: Option<String>,
    },

    /// Get video chapters
    #[command(name = "chapters")]
    Chapters {
        /// Video ID or URL
        #[arg(long, short)]
        id: String,
    },
}

/// Hacker News tools
#[derive(Subcommand, Clone)]
pub enum HackernewsTools {
    /// Search stories
    #[command(name = "search")]
    Search {
        /// Search query
        #[arg(long, short)]
        query: String,
        /// Maximum number of results
        #[arg(long, short, default_value_t = 10)]
        limit: u32,
    },

    /// Get a story by ID
    #[command(name = "story", alias = "get")]
    Story {
        /// Story ID
        #[arg(long, short)]
        id: u64,
    },

    /// Get top stories
    #[command(name = "top")]
    Top {
        /// Maximum number of results
        #[arg(long, short, default_value_t = 30)]
        limit: u32,
    },

    /// Get new stories
    #[command(name = "new", alias = "latest")]
    New {
        /// Maximum number of results
        #[arg(long, short, default_value_t = 30)]
        limit: u32,
    },

    /// Get best stories
    #[command(name = "best")]
    Best {
        /// Maximum number of results
        #[arg(long, short, default_value_t = 30)]
        limit: u32,
    },

    /// Get comments for a story
    #[command(name = "comments")]
    Comments {
        /// Story ID
        #[arg(long, short)]
        id: u64,
        /// Maximum number of comments
        #[arg(long, short, default_value_t = 50)]
        limit: u32,
    },
}

/// arXiv tools
#[derive(Subcommand, Clone)]
pub enum ArxivTools {
    /// Search papers
    #[command(name = "search")]
    Search {
        /// Search query
        #[arg(long, short)]
        query: String,
        /// Maximum number of results
        #[arg(long, short, default_value_t = 10)]
        limit: u32,
        /// Sort by: relevance, lastUpdatedDate, submittedDate
        #[arg(long, default_value = "relevance")]
        sort: String,
    },

    /// Get paper details
    #[command(name = "paper", alias = "get")]
    Paper {
        /// arXiv ID (e.g., 2301.07041)
        #[arg(long, short)]
        id: String,
    },

    /// Get paper PDF URL
    #[command(name = "pdf")]
    Pdf {
        /// arXiv ID
        #[arg(long, short)]
        id: String,
    },
}

/// GitHub tools
#[derive(Subcommand, Clone)]
pub enum GithubTools {
    /// Search repositories
    #[command(name = "search-repos", alias = "repos")]
    SearchRepos {
        /// Search query
        #[arg(long, short)]
        query: String,
        /// Maximum number of results
        #[arg(long, short, default_value_t = 10)]
        limit: u32,
    },

    /// Search code
    #[command(name = "search-code", alias = "code")]
    SearchCode {
        /// Search query
        #[arg(long, short)]
        query: String,
        /// Repository (owner/repo)
        #[arg(long, short)]
        repo: Option<String>,
        /// Maximum number of results
        #[arg(long, short, default_value_t = 10)]
        limit: u32,
    },

    /// List repository issues
    #[command(name = "issues")]
    Issues {
        /// Repository (owner/repo)
        #[arg(long, short)]
        repo: String,
        /// State: open, closed, all
        #[arg(long, default_value = "open")]
        state: String,
        /// Maximum number of results
        #[arg(long, short, default_value_t = 30)]
        limit: u32,
    },

    /// List repository pull requests
    #[command(name = "pulls", alias = "prs")]
    Pulls {
        /// Repository (owner/repo)
        #[arg(long, short)]
        repo: String,
        /// State: open, closed, all
        #[arg(long, default_value = "open")]
        state: String,
        /// Maximum number of results
        #[arg(long, short, default_value_t = 30)]
        limit: u32,
    },

    /// Get repository info
    #[command(name = "repo", alias = "get")]
    Repo {
        /// Repository (owner/repo)
        #[arg(long, short)]
        repo: String,
    },
}

/// Reddit tools
#[derive(Subcommand, Clone)]
pub enum RedditTools {
    /// Search posts
    #[command(name = "search")]
    Search {
        /// Search query
        #[arg(long, short)]
        query: String,
        /// Subreddit to search in
        #[arg(long, short)]
        subreddit: Option<String>,
        /// Maximum number of results
        #[arg(long, short, default_value_t = 25)]
        limit: u32,
    },

    /// Get hot posts
    #[command(name = "hot")]
    Hot {
        /// Subreddit
        #[arg(long, short)]
        subreddit: String,
        /// Maximum number of results
        #[arg(long, short, default_value_t = 25)]
        limit: u32,
    },

    /// Get new posts
    #[command(name = "new")]
    New {
        /// Subreddit
        #[arg(long, short)]
        subreddit: String,
        /// Maximum number of results
        #[arg(long, short, default_value_t = 25)]
        limit: u32,
    },

    /// Get top posts
    #[command(name = "top")]
    Top {
        /// Subreddit
        #[arg(long, short)]
        subreddit: String,
        /// Time filter: hour, day, week, month, year, all
        #[arg(long, short, default_value = "day")]
        time: String,
        /// Maximum number of results
        #[arg(long, short, default_value_t = 25)]
        limit: u32,
    },

    /// Get post details
    #[command(name = "post", alias = "get")]
    Post {
        /// Post ID or URL
        #[arg(long, short)]
        id: String,
    },
}

/// Web scraping tools
#[derive(Subcommand, Clone)]
pub enum WebTools {
    /// Scrape a web page
    #[command(name = "scrape", alias = "get")]
    Scrape {
        /// URL to scrape
        #[arg(long, short)]
        url: String,
        /// Output format: text, markdown, html
        #[arg(long, short, default_value = "markdown")]
        format: String,
    },

    /// Extract main content from a page
    #[command(name = "extract")]
    Extract {
        /// URL to extract from
        #[arg(long, short)]
        url: String,
        /// Extract images
        #[arg(long)]
        images: bool,
        /// Extract links
        #[arg(long)]
        links: bool,
    },

    /// Get page metadata
    #[command(name = "metadata", alias = "meta")]
    Metadata {
        /// URL
        #[arg(long, short)]
        url: String,
    },
}

/// Wikipedia tools
#[derive(Subcommand, Clone)]
pub enum WikipediaTools {
    /// Search articles
    #[command(name = "search")]
    Search {
        /// Search query
        #[arg(long, short)]
        query: String,
        /// Maximum number of results
        #[arg(long, short, default_value_t = 10)]
        limit: u32,
    },

    /// Get article content
    #[command(name = "article", alias = "get")]
    Article {
        /// Article title
        #[arg(long, short)]
        title: String,
    },

    /// Get article summary
    #[command(name = "summary")]
    Summary {
        /// Article title
        #[arg(long, short)]
        title: String,
    },
}

/// PubMed tools
#[derive(Subcommand, Clone)]
pub enum PubmedTools {
    /// Search articles
    #[command(name = "search")]
    Search {
        /// Search query
        #[arg(long, short)]
        query: String,
        /// Maximum number of results
        #[arg(long, short, default_value_t = 10)]
        limit: u32,
    },

    /// Get article by PMID
    #[command(name = "article", alias = "get")]
    Article {
        /// PubMed ID
        #[arg(long, short)]
        pmid: String,
    },
}

/// Semantic Scholar tools
#[derive(Subcommand, Clone)]
pub enum SemanticScholarTools {
    /// Search papers
    #[command(name = "search")]
    Search {
        /// Search query
        #[arg(long, short)]
        query: String,
        /// Maximum number of results
        #[arg(long, short, default_value_t = 10)]
        limit: u32,
    },

    /// Get paper details
    #[command(name = "paper", alias = "get")]
    Paper {
        /// Paper ID
        #[arg(long, short)]
        id: String,
    },

    /// Get paper citations
    #[command(name = "citations")]
    Citations {
        /// Paper ID
        #[arg(long, short)]
        id: String,
        /// Maximum number of results
        #[arg(long, short, default_value_t = 50)]
        limit: u32,
    },

    /// Get paper references
    #[command(name = "references")]
    References {
        /// Paper ID
        #[arg(long, short)]
        id: String,
        /// Maximum number of results
        #[arg(long, short, default_value_t = 50)]
        limit: u32,
    },
}

/// Slack tools
#[derive(Subcommand, Clone)]
pub enum SlackTools {
    /// List channels
    #[command(name = "channels", alias = "list-channels")]
    Channels {
        /// Maximum number of results
        #[arg(long, short, default_value_t = 100)]
        limit: u32,
    },

    /// Get channel messages
    #[command(name = "messages", alias = "history")]
    Messages {
        /// Channel name or ID
        #[arg(long, short)]
        channel: String,
        /// Maximum number of messages
        #[arg(long, short, default_value_t = 50)]
        limit: u32,
    },

    /// Search messages
    #[command(name = "search")]
    Search {
        /// Search query
        #[arg(long, short)]
        query: String,
        /// Maximum number of results
        #[arg(long, short, default_value_t = 50)]
        limit: u32,
    },

    /// List users
    #[command(name = "users")]
    Users {
        /// Maximum number of results
        #[arg(long, short, default_value_t = 100)]
        limit: u32,
    },
}

/// X (Twitter) tools
#[derive(Subcommand, Clone)]
pub enum XTools {
    /// Get user profile
    #[command(name = "profile", alias = "get-profile")]
    Profile {
        /// X username
        #[arg(long, short)]
        username: String,
    },

    /// Search tweets
    #[command(name = "search", alias = "search-tweets")]
    SearchTweets {
        /// Search query
        #[arg(long, short)]
        query: String,
        /// Maximum number of tweets
        #[arg(long, short)]
        limit: Option<u32>,
    },

    /// Get user followers
    #[command(name = "followers", alias = "get-followers")]
    Followers {
        /// Username
        #[arg(long, short)]
        username: String,
        /// Maximum number of followers
        #[arg(long, short)]
        limit: u32,
        /// Pagination cursor
        #[arg(long)]
        cursor: Option<String>,
    },

    /// Get tweet details
    #[command(name = "tweet", alias = "get-tweet")]
    Tweet {
        /// Tweet ID
        #[arg(long, short)]
        tweet_id: String,
    },

    /// Get home timeline
    #[command(name = "timeline", alias = "home")]
    Timeline {
        /// Number of tweets
        #[arg(long, short)]
        count: u32,
        /// Exclude replies
        #[arg(long)]
        exclude_replies: Option<bool>,
    },

    /// Fetch tweets and replies
    #[command(name = "tweets-and-replies")]
    TweetsAndReplies {
        /// Username
        #[arg(long, short)]
        username: String,
        /// Maximum number of tweets
        #[arg(long, short)]
        limit: u32,
        /// Pagination cursor
        #[arg(long)]
        cursor: Option<String>,
    },

    /// Search profiles
    #[command(name = "search-profiles")]
    SearchProfiles {
        /// Search query
        #[arg(long, short)]
        query: String,
        /// Maximum number of profiles
        #[arg(long, short)]
        limit: u32,
        /// Pagination cursor
        #[arg(long)]
        cursor: Option<String>,
    },

    /// Get direct message conversations
    #[command(name = "dm-conversations", alias = "dms")]
    DmConversations {
        /// User ID
        #[arg(long, short)]
        user_id: String,
        /// Pagination cursor
        #[arg(long)]
        cursor: Option<String>,
    },

    /// Send direct message
    #[command(name = "send-dm")]
    SendDm {
        /// Conversation ID
        #[arg(long, short)]
        conversation_id: String,
        /// Message text
        #[arg(long, short)]
        text: String,
    },
}

/// Discord tools
#[derive(Subcommand, Clone)]
pub enum DiscordTools {
    /// List servers
    #[command(name = "servers", alias = "list-servers")]
    Servers,

    /// Get server info
    #[command(name = "server", alias = "server-info")]
    Server {
        /// Guild/server ID
        #[arg(long, short)]
        guild_id: u64,
    },

    /// List channels
    #[command(name = "channels", alias = "list-channels")]
    Channels {
        /// Guild/server ID
        #[arg(long, short)]
        guild_id: u64,
    },

    /// Read messages
    #[command(name = "messages", alias = "read-messages")]
    Messages {
        /// Channel ID
        #[arg(long, short)]
        channel_id: u64,
        /// Number of messages (max 100)
        #[arg(long, short)]
        limit: Option<u32>,
    },

    /// Send message
    #[command(name = "send", alias = "send-message")]
    Send {
        /// Channel ID
        #[arg(long, short)]
        channel_id: u64,
        /// Message content
        #[arg(long, short)]
        content: String,
    },

    /// Search messages
    #[command(name = "search")]
    Search {
        /// Channel ID
        #[arg(long, short)]
        channel_id: u64,
        /// Search query
        #[arg(long, short)]
        query: String,
        /// Number of matching messages (max 100)
        #[arg(long, short)]
        limit: Option<u32>,
    },
}

/// RSS tools
#[derive(Subcommand, Clone)]
pub enum RssTools {
    /// Get feed metadata and recent entries
    #[command(name = "feed", alias = "get-feed")]
    Feed {
        /// Feed URL
        #[arg(long, short)]
        url: String,
        /// Number of entries
        #[arg(long, short)]
        limit: Option<u32>,
    },

    /// List feed entries
    #[command(name = "entries", alias = "list-entries")]
    Entries {
        /// Feed URL
        #[arg(long, short)]
        url: String,
        /// Number of entries
        #[arg(long, short)]
        limit: Option<u32>,
    },

    /// Search feed entries
    #[command(name = "search")]
    Search {
        /// Feed URL
        #[arg(long, short)]
        url: String,
        /// Search query
        #[arg(long, short)]
        query: String,
        /// Number of entries
        #[arg(long, short)]
        limit: Option<u32>,
    },

    /// Discover feeds on a webpage
    #[command(name = "discover")]
    Discover {
        /// Webpage URL
        #[arg(long, short)]
        url: String,
    },
}

/// bioRxiv tools
#[derive(Subcommand, Clone)]
pub enum BiorxivTools {
    /// Get recent preprints
    #[command(name = "recent")]
    Recent {
        /// Server (biorxiv or medrxiv)
        #[arg(long, short)]
        server: String,
        /// Number of papers (max 100)
        #[arg(long, short)]
        count: Option<u32>,
    },

    /// Get preprints by date range
    #[command(name = "date-range")]
    DateRange {
        /// Server (biorxiv or medrxiv)
        #[arg(long, short)]
        server: String,
        /// Start date (YYYY-MM-DD)
        #[arg(long)]
        start_date: String,
        /// End date (YYYY-MM-DD)
        #[arg(long)]
        end_date: String,
    },

    /// Get preprint by DOI
    #[command(name = "paper", alias = "get-paper")]
    Paper {
        /// Server (biorxiv or medrxiv)
        #[arg(long, short)]
        server: String,
        /// DOI
        #[arg(long, short)]
        doi: String,
    },
}

/// Sci-Hub tools
#[derive(Subcommand, Clone)]
pub enum ScihubTools {
    /// Get paper by DOI
    #[command(name = "paper", alias = "get-paper")]
    Paper {
        /// DOI
        #[arg(long, short)]
        doi: String,
    },
}

/// macOS tools
#[derive(Subcommand, Clone)]
pub enum MacosTools {
    /// Run AppleScript or JXA
    #[command(name = "script", alias = "run-script")]
    Script {
        /// Script language (applescript, javascript, jxa)
        #[arg(long, short, default_value = "applescript")]
        language: String,
        /// Script source code
        #[arg(long, short)]
        script: String,
        /// Optional parameters (JSON)
        #[arg(long)]
        params: Option<String>,
        /// Max output characters
        #[arg(long)]
        max_output_chars: Option<u32>,
    },

    /// Show notification
    #[command(name = "notify", alias = "notification")]
    Notify {
        /// Title
        #[arg(long, short)]
        title: Option<String>,
        /// Message
        #[arg(long, short)]
        message: String,
        /// Subtitle
        #[arg(long)]
        subtitle: Option<String>,
    },

    /// Reveal file in Finder
    #[command(name = "reveal")]
    Reveal {
        /// File path
        #[arg(long, short)]
        path: String,
    },

    /// Get clipboard content
    #[command(name = "clipboard", alias = "get-clipboard")]
    GetClipboard,

    /// Set clipboard content
    #[command(name = "set-clipboard")]
    SetClipboard {
        /// Text to copy
        #[arg(long, short)]
        text: String,
    },

    /// Run Apple Shortcut
    #[command(name = "shortcut", alias = "run-shortcut")]
    Shortcut {
        /// Shortcut name
        #[arg(long, short)]
        name: String,
        /// Optional input (JSON)
        #[arg(long, short)]
        input: Option<String>,
    },
}

/// Spotlight tools
#[derive(Subcommand, Clone)]
pub enum SpotlightTools {
    /// Full-text content search
    #[command(name = "search", alias = "search-content")]
    SearchContent {
        /// Search query
        #[arg(long, short)]
        query: String,
        /// Directory to search in
        #[arg(long, short)]
        directory: Option<String>,
        /// File kind filter
        #[arg(long, short)]
        kind: Option<String>,
        /// Maximum results
        #[arg(long, short, default_value_t = 50)]
        limit: u32,
    },

    /// Search by file name
    #[command(name = "name", alias = "search-by-name")]
    SearchByName {
        /// File name
        #[arg(long, short)]
        name: String,
        /// Directory to search in
        #[arg(long, short)]
        directory: Option<String>,
        /// Maximum results
        #[arg(long, short, default_value_t = 50)]
        limit: u32,
    },

    /// Search by file kind
    #[command(name = "kind", alias = "search-by-kind")]
    SearchByKind {
        /// File kind (pdf, image, video, etc.)
        #[arg(long, short)]
        kind: String,
        /// Directory to search in
        #[arg(long, short)]
        directory: Option<String>,
        /// Maximum results
        #[arg(long, short, default_value_t = 50)]
        limit: u32,
    },

    /// Search recent files
    #[command(name = "recent", alias = "search-recent")]
    SearchRecent {
        /// Number of days
        #[arg(long, short, default_value_t = 7)]
        days: u32,
        /// File kind filter
        #[arg(long, short)]
        kind: Option<String>,
        /// Directory to search in
        #[arg(long)]
        directory: Option<String>,
        /// Maximum results
        #[arg(long, short, default_value_t = 50)]
        limit: u32,
    },

    /// Get file metadata
    #[command(name = "metadata", alias = "get-metadata")]
    Metadata {
        /// File path
        #[arg(long, short)]
        path: String,
    },

    /// Raw Spotlight query
    #[command(name = "raw", alias = "raw-query")]
    RawQuery {
        /// Raw mdfind query
        #[arg(long, short)]
        query: String,
        /// Directory to search in
        #[arg(long, short)]
        directory: Option<String>,
        /// Maximum results
        #[arg(long, short, default_value_t = 50)]
        limit: u32,
    },
}
