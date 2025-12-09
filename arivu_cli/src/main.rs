use clap::Parser;
use owo_colors::OwoColorize;
use std::process;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod cli;
mod commands;
mod output;

#[cfg(feature = "tui")]
mod tui;

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
    // "arivu <connector> ..." -> "arivu call <connector> ..."
    let mut args: Vec<String> = std::env::args().collect();
    let built_in_commands = [
        "list",
        "ls",
        "setup",
        "init",
        "search",
        "get",
        "config",
        "connectors",
        "tools",
        "call",
        "help",
        "--help",
        "-h",
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
            // Check if there is a second positional argument (the tool name)
            // We scan from the argument *after* the connector
            let has_tool_arg = args
                .iter()
                .skip(real_idx + 1)
                .any(|arg| !arg.starts_with('-'));

            if has_tool_arg {
                // Case: `arivu hackernews get_stories` -> `arivu call hackernews get_stories`
                args.insert(real_idx, "call".to_string());
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
    let result = match &cli.command {
        None => {
            // No command provided - show quick overview
            show_overview().await
        }
        Some(Commands::List) => list::run(&cli).await,
        Some(Commands::Setup { connector }) => setup::run(&cli, connector.as_deref()).await,
        Some(Commands::Search {
            connector, query, ..
        }) => search::run(&cli, connector, query).await,
        Some(Commands::Get { connector, id }) => get::run(&cli, connector, id).await,
        Some(Commands::Config { action }) => config::run(&cli, action.clone()).await,
        Some(Commands::Connectors) => connectors::run(&cli).await,
        Some(Commands::Tools { connector }) => tools::run(&cli, connector.as_deref()).await,
        Some(Commands::Call {
            connector,
            tool,
            args,
            params,
        }) => call::run(&cli, connector, tool, args.as_deref(), params).await,
    };

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
