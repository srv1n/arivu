use crate::cli::Cli;
use crate::commands::{copy_to_clipboard, CommandError, Result};
use crate::output::{format_output, OutputData};
use arivu_core::federated::{FederatedSearch, MergeMode, ProfileStore, SearchProfile};
use arivu_core::{CallToolRequestParam, PaginatedRequestParam, ProviderRegistry};
use indicatif::{ProgressBar, ProgressStyle};
use owo_colors::OwoColorize;
use serde_json::{json, Value};
use std::sync::Arc;

/// Run a search command - either single connector or federated.
///
/// # Arguments
/// - `connector_or_query`: Either a connector name (single search) or the query (federated search)
/// - `query`: The search query (only used for single connector search)
/// - `limit`: Maximum results per source
/// - `profile`: Named profile for federated search (research, enterprise, social, code, web)
/// - `connectors`: Comma-separated list of connectors for ad-hoc federated search
/// - `merge`: Merge mode (grouped or interleaved)
/// - `add`: Additional connectors to add to profile
/// - `exclude`: Connectors to exclude from profile
#[allow(clippy::too_many_arguments)]
pub async fn run(
    cli: &Cli,
    connector_or_query: &str,
    query: Option<&str>,
    limit: u32,
    profile: Option<&str>,
    connectors: Option<&str>,
    merge: &str,
    add: Option<&str>,
    exclude: Option<&str>,
) -> Result<()> {
    // Determine if this is a federated search or single connector search
    let is_federated = profile.is_some() || connectors.is_some();

    if is_federated {
        // Federated search: connector_or_query is the query
        run_federated_search(
            cli,
            connector_or_query,
            limit,
            profile,
            connectors,
            merge,
            add,
            exclude,
        )
        .await
    } else {
        // Single connector search: connector_or_query is the connector name
        let query = query.ok_or_else(|| {
            CommandError::InvalidInput(
                "Missing search query. Usage: arivu search <connector> \"<query>\"".to_string(),
            )
        })?;
        run_single_search(cli, connector_or_query, query, limit).await
    }
}

/// Run a single connector search.
async fn run_single_search(cli: &Cli, connector_name: &str, query: &str, limit: u32) -> Result<()> {
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
                "limit": limit
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

/// Run a federated search across multiple connectors.
#[allow(clippy::too_many_arguments)]
async fn run_federated_search(
    cli: &Cli,
    query: &str,
    limit: u32,
    profile: Option<&str>,
    connectors: Option<&str>,
    merge: &str,
    add: Option<&str>,
    exclude: Option<&str>,
) -> Result<()> {
    let merge_mode = match merge {
        "interleaved" => MergeMode::Interleaved,
        _ => MergeMode::Grouped,
    };

    // Determine sources
    let source_description = if let Some(profile_name) = profile {
        format!("profile '{}'", profile_name)
    } else if let Some(connector_list) = connectors {
        format!("connectors: {}", connector_list)
    } else {
        "default profile".to_string()
    };

    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .expect("Invalid progress template"),
    );
    spinner.set_message(format!(
        "Searching {} for '{}'...",
        source_description, query
    ));
    spinner.enable_steady_tick(std::time::Duration::from_millis(100));

    let registry = Arc::new(create_registry().await?);
    let engine = FederatedSearch::new(&registry);

    let result = if let Some(profile_name) = profile {
        // Profile-based search
        let profile_store = ProfileStore::new_default();
        let mut search_profile = profile_store.load(profile_name).ok_or_else(|| {
            CommandError::InvalidInput(format!(
                "Profile '{}' not found. Available built-in profiles: {}",
                profile_name,
                ProfileStore::list_builtin_names().join(", ")
            ))
        })?;

        // Apply limit override
        search_profile.defaults.limit = limit;

        // Apply add/exclude modifiers
        if let Some(add_connectors) = add {
            let additional: Vec<String> = add_connectors
                .split(',')
                .map(|s| s.trim().to_string())
                .collect();
            search_profile.connectors.extend(additional);
        }

        if let Some(exclude_connectors) = exclude {
            let to_exclude: Vec<&str> = exclude_connectors.split(',').map(|s| s.trim()).collect();
            search_profile
                .connectors
                .retain(|c| !to_exclude.contains(&c.as_str()));
        }

        engine
            .search_with_profile(query, &search_profile, Some(merge_mode))
            .await
    } else if let Some(connector_list) = connectors {
        // Ad-hoc connector list
        let connector_names: Vec<String> = connector_list
            .split(',')
            .map(|s| s.trim().to_string())
            .collect();

        engine
            .search_adhoc(query, &connector_names, merge_mode)
            .await
    } else {
        // Default to research profile
        let search_profile = SearchProfile::get_builtin("research")
            .ok_or_else(|| CommandError::InvalidInput("Default profile not found".to_string()))?;

        engine
            .search_with_profile(query, &search_profile, Some(merge_mode))
            .await
    };

    spinner.finish_and_clear();

    // Convert to JSON for output
    let result_json =
        serde_json::to_value(&result).map_err(|e| CommandError::Other(e.to_string()))?;

    match cli.output {
        crate::cli::OutputFormat::Pretty => {
            format_pretty_federated_results(&result)?;
        }
        _ => {
            let output_data = OutputData::FederatedResults {
                query: query.to_string(),
                profile: profile.map(|s| s.to_string()),
                results: result_json.clone(),
            };
            format_output(&output_data, &cli.output)?;
        }
    }

    // Copy to clipboard if requested
    if cli.copy {
        let text = serde_json::to_string_pretty(&result_json)?;
        copy_to_clipboard(&text)?;
    }

    Ok(())
}

/// Format federated search results for pretty output.
fn format_pretty_federated_results(
    result: &arivu_core::federated::FederatedSearchResult,
) -> Result<()> {
    use arivu_core::federated::FederatedResults;

    println!(
        "{} {}",
        "Federated Search:".bold().cyan(),
        result.query.yellow()
    );
    if let Some(ref profile) = result.profile {
        println!("{} {}", "Profile:".bold(), profile.cyan());
    }
    println!();

    match &result.results {
        FederatedResults::Grouped { sources } => {
            for source in sources {
                println!(
                    "{} {} {}",
                    "━━".cyan(),
                    source.source.bold().green(),
                    format!("({} results)", source.count).dimmed()
                );

                if source.results.is_empty() {
                    println!("   {}", "No results".dimmed());
                } else {
                    for (i, r) in source.results.iter().enumerate() {
                        println!(
                            "   {} {}",
                            format!("{}.", i + 1).cyan().bold(),
                            r.title.bold()
                        );

                        if let Some(ref url) = r.url {
                            println!("      {}", url.blue());
                        }

                        if let Some(ref snippet) = r.snippet {
                            let truncated = if snippet.len() > 150 {
                                format!("{}...", &snippet[..150])
                            } else {
                                snippet.clone()
                            };
                            println!("      {}", truncated.dimmed());
                        }
                    }
                }
                println!();
            }
        }
        FederatedResults::Interleaved { results } => {
            println!(
                "{} {}",
                "Results:".bold(),
                format!("({} total, interleaved)", results.len()).dimmed()
            );
            println!();

            for (i, r) in results.iter().enumerate() {
                println!(
                    "{} {} {}",
                    format!("{}.", i + 1).cyan().bold(),
                    r.title.bold(),
                    format!("({})", r.source).dimmed()
                );

                if let Some(ref url) = r.url {
                    println!("   {}", url.blue());
                }

                if let Some(ref snippet) = r.snippet {
                    let truncated = if snippet.len() > 150 {
                        format!("{}...", &snippet[..150])
                    } else {
                        snippet.clone()
                    };
                    println!("   {}", truncated.dimmed());
                }
                println!();
            }
        }
    }

    // Show errors if any
    if result.partial && !result.errors.is_empty() {
        println!("{}", "Partial results. Some sources failed:".yellow());
        for err in &result.errors {
            let timeout_marker = if err.is_timeout { " (timeout)" } else { "" };
            println!(
                "  {} {}: {}{}",
                "⚠".yellow(),
                err.source.yellow(),
                err.error,
                timeout_marker
            );
        }
        println!();
    }

    // Show timing
    if let Some(duration) = result.duration_ms {
        println!("{}", format!("Completed in {}ms", duration).dimmed());
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
