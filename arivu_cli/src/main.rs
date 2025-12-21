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

    let cli = Cli::parse();

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
