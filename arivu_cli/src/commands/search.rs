use crate::cli::Cli;
use crate::commands::{copy_to_clipboard, CommandError, Result};
use crate::output::{format_output, OutputData};
use arivu_core::{CallToolRequestParam, PaginatedRequestParam, ProviderRegistry};
use indicatif::{ProgressBar, ProgressStyle};
use owo_colors::OwoColorize;
use serde_json::{json, Value};

pub async fn run(cli: &Cli, connector_name: &str, query: &str) -> Result<()> {
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .expect("Invalid progress template"),
    );
    spinner.set_message(format!("Searching {} for '{}'...", connector_name, query));
    spinner.enable_steady_tick(std::time::Duration::from_millis(100));

    let registry = create_registry().await?;
    let provider = registry
        .get_provider(connector_name)
        .ok_or_else(|| CommandError::ConnectorNotFound(connector_name.to_string()))?
        .clone();

    // List available tools to find the search tool
    let c = provider.lock().await;
    let tools_response = c
        .list_tools(Some(PaginatedRequestParam { cursor: None }))
        .await?;

    // Find appropriate search tool
    let search_tool = tools_response
        .tools
        .iter()
        .find(|tool| tool.name.contains("search") || tool.name.contains("query"))
        .ok_or_else(|| {
            CommandError::ToolNotFound("search".to_string(), connector_name.to_string())
        })?;

    // Prepare search request
    let request = CallToolRequestParam {
        name: search_tool.name.clone(),
        arguments: Some(
            json!({
                "query": query,
                "limit": get_search_limit(&cli.command).unwrap_or(10)
            })
            .as_object()
            .expect("JSON object")
            .clone(),
        ),
    };

    let response = c.call_tool(request).await?;
    spinner.finish_and_clear();

    // Extract response data
    let results = if let Some(val) = &response.structured_content {
        val.clone()
    } else {
        json!({})
    };

    let output_data = OutputData::SearchResults {
        connector: connector_name.to_string(),
        query: query.to_string(),
        results: results.clone(),
    };

    match cli.output {
        crate::cli::OutputFormat::Pretty => {
            format_pretty_search_results(connector_name, query, &results)?;
        }
        _ => {
            format_output(&output_data, &cli.output)?;
        }
    }

    // Copy to clipboard if requested
    if cli.copy {
        let text = serde_json::to_string_pretty(&results)?;
        copy_to_clipboard(&text)?;
    }

    Ok(())
}

fn format_pretty_search_results(connector: &str, query: &str, results: &Value) -> Result<()> {
    println!("{} {}", "Search Results:".bold().cyan(), query.yellow());
    println!("{} {}", "Connector:".bold(), connector.cyan());
    println!();

    // Handle different result formats based on connector
    match connector {
        "youtube" => format_youtube_results(results)?,
        "reddit" => format_reddit_results(results)?,
        "hackernews" => format_hackernews_results(results)?,
        "wikipedia" => format_wikipedia_results(results)?,
        _ => {
            // Generic formatting
            println!("{}", serde_json::to_string_pretty(results)?);
        }
    }

    Ok(())
}

fn format_youtube_results(results: &Value) -> Result<()> {
    if let Some(videos) = results.get("videos").and_then(|v| v.as_array()) {
        for (i, video) in videos.iter().enumerate() {
            if i > 0 {
                println!();
            }

            if let (Some(title), Some(url)) = (video.get("title"), video.get("url")) {
                println!(
                    "{} {}",
                    format!("{}.", i + 1).cyan().bold(),
                    title.as_str().unwrap_or("").bold()
                );
                println!("   {}", url.as_str().unwrap_or("").blue());

                if let Some(desc) = video.get("description") {
                    let desc_str = desc.as_str().unwrap_or("");
                    if !desc_str.is_empty() {
                        let truncated = if desc_str.len() > 100 {
                            format!("{}...", &desc_str[..100])
                        } else {
                            desc_str.to_string()
                        };
                        println!("   {}", truncated.dimmed());
                    }
                }
            }
        }
    } else {
        println!("{}", serde_json::to_string_pretty(results)?);
    }
    Ok(())
}

fn format_reddit_results(results: &Value) -> Result<()> {
    // TODO: Implement Reddit-specific formatting
    println!("{}", serde_json::to_string_pretty(results)?);
    Ok(())
}

fn format_hackernews_results(results: &Value) -> Result<()> {
    // TODO: Implement HackerNews-specific formatting
    println!("{}", serde_json::to_string_pretty(results)?);
    Ok(())
}

fn format_wikipedia_results(results: &Value) -> Result<()> {
    // TODO: Implement Wikipedia-specific formatting
    println!("{}", serde_json::to_string_pretty(results)?);
    Ok(())
}

async fn create_registry() -> Result<ProviderRegistry> {
    // Reuse the registry creation logic from list.rs
    crate::commands::list::create_registry().await
}

fn get_search_limit(command: &Option<crate::cli::Commands>) -> Option<u32> {
    match command {
        Some(crate::cli::Commands::Search { limit, .. }) => Some(*limit),
        _ => None,
    }
}
