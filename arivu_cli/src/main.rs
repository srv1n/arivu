use clap::Parser;
use owo_colors::OwoColorize;
use std::process;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod cli;
mod commands;
mod output;

#[cfg(feature = "tui")]
mod tui;

use arivu_core::UsageContext;
use cli::{Cli, Commands};
use commands::*;

#[tokio::main]
async fn main() {
    // Initialize logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "arivu_cli=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Pre-process arguments to support shorthand syntax:
    // "arivu <connector> ..." -> "arivu call <connector> ..." or "arivu search <connector> ..."
    let mut args: Vec<String> = std::env::args().collect();
    let built_in_commands = [
        // Core commands
        "list",
        "ls",
        "setup",
        "init",
        "search",
        "get",
        "fetch",
        "f",
        "formats",
        "patterns",
        "config",
        "connectors",
        "tools",
        "pricing",
        "usage",
        "call",
        "help",
        "--help",
        "-h",
        // Connector-specific subcommands with proper CLI flags
        "localfs",
        "fs",
        "file",
        "youtube",
        "yt",
        "hackernews",
        "hn",
        "arxiv",
        "github",
        "gh",
        "reddit",
        "web",
        "wikipedia",
        "wiki",
        "pubmed",
        "semantic-scholar",
        "scholar",
        "slack",
        "x",
        "twitter",
        "discord",
        "rss",
        "biorxiv",
        "scihub",
        "macos",
        "spotlight",
        // Google connectors
        "google-calendar",
        "gcal",
        "google-drive",
        "gdrive",
        "google-gmail",
        "gmail",
        "google-people",
        "gpeople",
        "google-scholar",
        "gscholar",
        // LLM Search connectors
        "openai-search",
        "anthropic-search",
        "gemini-search",
        "perplexity-search",
        "xai-search",
        "exa",
        "tavily-search",
        "serper-search",
        "serpapi-search",
        "firecrawl-search",
        "parallel-search",
        // Productivity connectors
        "atlassian",
        "jira",
        "microsoft-graph",
        "msgraph",
        "imap",
        "email",
    ];

    // Find the index of the subcommand (first non-flag argument)
    // We skip index 0 (the binary name)
    let subcommand_idx = args.iter().skip(1).position(|arg| !arg.starts_with('-'));

    if let Some(idx) = subcommand_idx {
        // adjust index because we skipped 1
        let real_idx = idx + 1;
        let potential_command = &args[real_idx];

        // If it's not a built-in command, assume it's a connector name
        if !built_in_commands.contains(&potential_command.as_str()) {
            // Find the second positional argument (after the connector)
            let second_pos_idx = args
                .iter()
                .enumerate()
                .skip(real_idx + 1)
                .find(|(_, arg)| !arg.starts_with('-'))
                .map(|(i, _)| i);

            if let Some(second_idx) = second_pos_idx {
                let second_arg = &args[second_idx];

                // Heuristic: if the second argument looks like a search query rather than a tool name:
                // - Contains spaces (already expanded by shell from quotes)
                // - Contains special chars like ?, !, etc.
                // - Is all lowercase with no underscores (tool names typically use snake_case)
                // - Doesn't match common tool name patterns (get_*, search_*, list_*, etc.)
                let looks_like_query = second_arg.contains(' ')
                    || second_arg.contains('?')
                    || second_arg.contains('!')
                    || (!second_arg.contains('_')
                        && !second_arg.starts_with("get")
                        && !second_arg.starts_with("search")
                        && !second_arg.starts_with("list")
                        && !second_arg.starts_with("fetch")
                        && !second_arg.starts_with("create")
                        && !second_arg.starts_with("update")
                        && !second_arg.starts_with("delete")
                        && !second_arg.starts_with("test"));

                if looks_like_query {
                    // Case: `arivu google-scholar "crispr"` -> `arivu search google-scholar "crispr"`
                    args.insert(real_idx, "search".to_string());
                } else {
                    // Case: `arivu hackernews get_stories` -> `arivu call hackernews get_stories`
                    args.insert(real_idx, "call".to_string());
                }
            } else {
                // Case: `arivu hackernews` -> `arivu tools hackernews`
                args.insert(real_idx, "tools".to_string());
            }
        }
    }

    let cli = Cli::parse_from(args);

    // Handle TUI mode
    #[cfg(feature = "tui")]
    if cli.tui {
        if let Err(e) = tui::run().await {
            eprintln!("{}: {}", "Error".red().bold(), e);
            process::exit(1);
        }
        return;
    }

    // Handle regular CLI commands
    let usage_ctx = match std::env::var("ARIVU_RUN_ID") {
        Ok(id) => UsageContext::new(id),
        Err(_) => UsageContext::new_random(),
    };

    let result = usage_ctx
        .scope(|| async {
            match &cli.command {
                None => {
                    // No command provided - show quick overview
                    show_overview().await
                }
                Some(Commands::List) => list::run(&cli).await,
                Some(Commands::Setup { connector }) => setup::run(&cli, connector.as_deref()).await,
                Some(Commands::Search {
                    connector_or_query,
                    query,
                    limit,
                    profile,
                    connectors,
                    merge,
                    add,
                    exclude,
                }) => {
                    search::run(
                        &cli,
                        connector_or_query,
                        query.as_deref(),
                        *limit,
                        profile.as_deref(),
                        connectors.as_deref(),
                        merge,
                        add.as_deref(),
                        exclude.as_deref(),
                        false, // web flag removed
                    )
                    .await
                }
                Some(Commands::Get { connector, id }) => get::run(&cli, connector, id).await,
                Some(Commands::Fetch { input }) => fetch::run(&cli, input).await,
                Some(Commands::Formats) => fetch::show_formats(&cli).await,
                Some(Commands::Config { action }) => config::run(&cli, action.clone()).await,
                Some(Commands::Connectors) => connectors::run(&cli).await,
                Some(Commands::Tools { connector }) => tools::run(&cli, connector.as_deref()).await,
                Some(Commands::Pricing {
                    connector,
                    tool,
                    model,
                }) => {
                    pricing::run(
                        &cli,
                        connector.as_deref(),
                        tool.as_deref(),
                        model.as_deref(),
                    )
                    .await
                }
                Some(Commands::Usage {
                    connector,
                    tool,
                    run,
                    last,
                }) => {
                    usage::run(
                        &cli,
                        connector.as_deref(),
                        tool.as_deref(),
                        run.as_deref(),
                        *last,
                    )
                    .await
                }
                Some(Commands::Call {
                    connector,
                    tool,
                    args,
                    params,
                }) => call::run(&cli, connector, tool, args.as_deref(), params).await,

                // Google connectors
                Some(Commands::GoogleCalendar { tool }) => {
                    connectors::handle_google_calendar(&cli, tool.clone()).await
                }
                Some(Commands::GoogleDrive { tool }) => {
                    connectors::handle_google_drive(&cli, tool.clone()).await
                }
                Some(Commands::GoogleGmail { tool }) => {
                    connectors::handle_google_gmail(&cli, tool.clone()).await
                }
                Some(Commands::GooglePeople { tool }) => {
                    connectors::handle_google_people(&cli, tool.clone()).await
                }
                Some(Commands::GoogleScholar { tool }) => {
                    connectors::handle_google_scholar(&cli, tool.clone()).await
                }

                // LLM Search connectors
                Some(Commands::OpenaiSearch { tool }) => {
                    connectors::handle_openai_search(&cli, tool.clone()).await
                }
                Some(Commands::AnthropicSearch { tool }) => {
                    connectors::handle_anthropic_search(&cli, tool.clone()).await
                }
                Some(Commands::GeminiSearch { tool }) => {
                    connectors::handle_gemini_search(&cli, tool.clone()).await
                }
                Some(Commands::PerplexitySearch { tool }) => {
                    connectors::handle_perplexity_search(&cli, tool.clone()).await
                }
                Some(Commands::XaiSearch { tool }) => {
                    connectors::handle_xai_search(&cli, tool.clone()).await
                }
                Some(Commands::Exa { tool }) => connectors::handle_exa(&cli, tool.clone()).await,
                Some(Commands::TavilySearch { tool }) => {
                    connectors::handle_tavily_search(&cli, tool.clone()).await
                }
                Some(Commands::SerperSearch { tool }) => {
                    connectors::handle_serper_search(&cli, tool.clone()).await
                }
                Some(Commands::SerpapiSearch { tool }) => {
                    connectors::handle_serpapi_search(&cli, tool.clone()).await
                }
                Some(Commands::FirecrawlSearch { tool }) => {
                    connectors::handle_firecrawl_search(&cli, tool.clone()).await
                }
                Some(Commands::ParallelSearch { tool }) => {
                    connectors::handle_parallel_search(&cli, tool.clone()).await
                }

                // Productivity connectors
                Some(Commands::Atlassian { tool }) => {
                    connectors::handle_atlassian(&cli, tool.clone()).await
                }
                Some(Commands::MicrosoftGraph { tool }) => {
                    connectors::handle_microsoft_graph(&cli, tool.clone()).await
                }
                Some(Commands::Imap { tool }) => connectors::handle_imap(&cli, tool.clone()).await,

                // For now, other connectors fall back to the call command
                // Connector-specific subcommands with proper CLI flags
                Some(Commands::Localfs { tool }) => {
                    connectors::handle_localfs(&cli, tool.clone()).await
                }
                Some(Commands::Youtube { tool }) => {
                    connectors::handle_youtube(&cli, tool.clone()).await
                }
                Some(Commands::Hackernews { tool }) => {
                    connectors::handle_hackernews(&cli, tool.clone()).await
                }
                Some(Commands::Arxiv { tool }) => {
                    connectors::handle_arxiv(&cli, tool.clone()).await
                }
                Some(Commands::Github { tool }) => {
                    connectors::handle_github(&cli, tool.clone()).await
                }
                Some(Commands::Reddit { tool }) => {
                    connectors::handle_reddit(&cli, tool.clone()).await
                }
                Some(Commands::Web { tool }) => connectors::handle_web(&cli, tool.clone()).await,
                Some(Commands::Wikipedia { tool }) => {
                    connectors::handle_wikipedia(&cli, tool.clone()).await
                }
                Some(Commands::Pubmed { tool }) => {
                    connectors::handle_pubmed(&cli, tool.clone()).await
                }
                Some(Commands::SemanticScholar { tool }) => {
                    connectors::handle_semantic_scholar(&cli, tool.clone()).await
                }
                Some(Commands::Slack { tool }) => {
                    connectors::handle_slack(&cli, tool.clone()).await
                }
                Some(Commands::X { tool }) => connectors::handle_x(&cli, tool.clone()).await,
                Some(Commands::Discord { tool }) => {
                    connectors::handle_discord(&cli, tool.clone()).await
                }
                Some(Commands::Rss { tool }) => connectors::handle_rss(&cli, tool.clone()).await,
                Some(Commands::Biorxiv { tool }) => {
                    connectors::handle_biorxiv(&cli, tool.clone()).await
                }
                Some(Commands::Scihub { tool }) => {
                    connectors::handle_scihub(&cli, tool.clone()).await
                }
                Some(Commands::Macos { tool }) => {
                    connectors::handle_macos(&cli, tool.clone()).await
                }
                Some(Commands::Spotlight { tool }) => {
                    connectors::handle_spotlight(&cli, tool.clone()).await
                }
            }
        })
        .await;

    if let Err(e) = result {
        eprintln!("{}: {}", "Error".red().bold(), e);
        process::exit(1);
    }
}

async fn show_overview() -> commands::Result<()> {
    println!();
    println!(
        "{}  {}",
        "Arivu".bold().cyan(),
        "- Unified Data Access CLI".dimmed()
    );
    println!();

    // Show quick stats
    let registry = list::create_registry().await?;
    let providers = registry.list_providers();

    // Count tools and categorize connectors
    let mut total_tools = 0;
    let mut no_auth_count = 0;
    let mut auth_count = 0;

    for provider_info in &providers {
        if let Some(provider) = registry.get_provider(&provider_info.name) {
            let c = provider.lock().await;
            if let Ok(tools) = c
                .list_tools(Some(arivu_core::PaginatedRequestParam { cursor: None }))
                .await
            {
                total_tools += tools.tools.len();
            }
            let schema = c.config_schema();
            if schema.fields.is_empty() {
                no_auth_count += 1;
            } else {
                auth_count += 1;
            }
        }
    }

    println!(
        "  {} connectors available ({} ready to use, {} need auth)",
        providers.len().to_string().green().bold(),
        no_auth_count.to_string().green(),
        auth_count.to_string().yellow()
    );
    println!(
        "  {} tools across all connectors",
        total_tools.to_string().green().bold()
    );
    println!();

    // Quick start section
    println!("{}", "Quick Start:".bold().cyan());
    println!(
        "  {}{}",
        "arivu tools".cyan(),
        "                Show all tools with auth requirements".dimmed()
    );
    println!(
        "  {}{}",
        "arivu search youtube \"query\"".cyan(),
        "  Search YouTube videos".dimmed()
    );
    println!(
        "  {}{}",
        "arivu setup".cyan(),
        "                Interactive setup wizard".dimmed()
    );
    println!();

    // Ready to use section
    println!("{}", "Ready to use (no auth required):".bold().green());
    let ready: Vec<_> = providers
        .iter()
        .filter(|p| {
            // Check if connector exists and is a known no-auth connector
            registry.get_provider(&p.name).is_some()
                && matches!(
                    p.name.as_str(),
                    "youtube"
                        | "hackernews"
                        | "arxiv"
                        | "pubmed"
                        | "wikipedia"
                        | "semantic_scholar"
                        | "web"
                )
        })
        .collect();

    if !ready.is_empty() {
        let names: Vec<_> = ready.iter().map(|p| p.name.cyan().to_string()).collect();
        println!("  {}", names.join(", "));
    }
    println!();

    println!(
        "{}",
        "Need auth (run 'arivu setup <name>'):".bold().yellow()
    );
    let need_auth: Vec<_> = providers
        .iter()
        .filter(|p| {
            matches!(
                p.name.as_str(),
                "slack"
                    | "github"
                    | "atlassian"
                    | "reddit"
                    | "microsoft-graph"
                    | "google-drive"
                    | "google-gmail"
                    | "google-calendar"
                    | "openai-search"
                    | "anthropic-search"
                    | "perplexity-search"
            )
        })
        .collect();

    if !need_auth.is_empty() {
        let names: Vec<_> = need_auth
            .iter()
            .map(|p| p.name.yellow().to_string())
            .collect();
        println!("  {}", names.join(", "));
    }
    println!();

    println!(
        "{} Use {} for full help",
        "Tip:".dimmed(),
        "arivu --help".cyan()
    );
    println!();

    Ok(())
}
