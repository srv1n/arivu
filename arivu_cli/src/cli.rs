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
    /// A convenience command that calls the search tool of a connector.
    #[command(after_help = "\x1b[1;33mExamples:\x1b[0m
  arivu search youtube \"rust programming\"
  arivu search hackernews \"async rust\" --limit 5
  arivu search arxiv \"machine learning\"
  arivu search reddit \"cli tools\" --limit 20")]
    Search {
        /// The connector to use (e.g., youtube, reddit, hackernews)
        connector: String,
        /// The search query
        query: String,
        /// Maximum number of results
        #[arg(short, long, default_value_t = 10)]
        limit: u32,
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

    /// Call a tool directly from a connector
    ///
    /// You can use the simplified syntax: `arivu <connector> <tool> [args...]`
    /// The CLI automatically maps positional arguments to the tool's parameters.
    ///
    /// Or use the advanced syntax with JSON for full control.
    #[command(after_help = "\x1b[1;33mExamples:\x1b[0m
  # Simplified Syntax (Recommended):
  arivu youtube search_videos huberman
  arivu hackernews search_stories rust 5
  arivu slack list_channels

  # Advanced / Scripting (JSON Args):
  arivu call youtube search_videos --args '{\"query\": \"rust\", \"limit\": 10}'
  arivu call github search_code --args '{\"query\": \"async fn\", \"repo\": \"tokio-rs/tokio\"}'

\x1b[1;36mTip:\x1b[0m Use `arivu tools <connector>` to see all available tools.")]
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
