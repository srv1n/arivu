use crate::cli::Cli;
use crate::commands::{copy_to_clipboard, CommandError, Result};
use crate::output::{format_output, OutputData};
use arivu_core::resolver::{PatternInfo, ResolvedAction, SmartResolver};
use arivu_core::CallToolRequestParam;
use owo_colors::OwoColorize;
use serde_json::json;
use std::io::{self, Write};

/// Run the fetch command - auto-detect input type and fetch content
pub async fn run(cli: &Cli, input: &str) -> Result<()> {
    let resolver = SmartResolver::new();

    // Get all possible matches
    let actions = resolver.resolve_all(input);

    if actions.is_empty() {
        println!();
        println!(
            "{} Could not detect the type of input: {}",
            "Error:".red().bold(),
            input.yellow()
        );
        println!();
        print_shell_quoting_hint(input);
        println!("Run {} to see supported formats.", "arivu formats".cyan());
        println!();
        return Ok(());
    }

    // Filter out low-priority matches when there's a clear winner
    // If highest priority is significantly higher than others (e.g., 100 vs 1), auto-select
    let actions = filter_ambiguous_matches(actions);

    // If only one match, use it directly
    let action = if actions.len() == 1 {
        actions.into_iter().next().unwrap()
    } else {
        // Multiple matches with similar priority - let user choose
        select_action(cli, input, actions)?
    };

    // Show what was detected
    if cli.output == crate::cli::OutputFormat::Pretty {
        println!();
        println!(
            "{} {}",
            "Detected:".bold().cyan(),
            action.description.dimmed()
        );
        println!(
            "  {} {} → {}",
            "Routing to:".dimmed(),
            action.connector.cyan().bold(),
            action.tool.green()
        );

        // Show hint if it looks like a URL that should have matched a specific connector
        // but fell back to generic web scraping
        if action.connector == "web" {
            print_shell_quoting_hint(input);
        }

        println!();
    }

    // Execute the action
    execute_action(cli, &action).await
}

/// Filter out low-priority matches when there's a clear winner
/// This prevents showing "ambiguous" when a specific pattern (e.g., YouTube URL)
/// matches alongside a generic one (e.g., web URL)
fn filter_ambiguous_matches(mut actions: Vec<ResolvedAction>) -> Vec<ResolvedAction> {
    if actions.len() <= 1 {
        return actions;
    }

    // Sort by priority descending
    actions.sort_by(|a, b| b.priority.cmp(&a.priority));

    let highest_priority = actions[0].priority;

    // Keep only actions within 20 priority points of the highest
    // This filters out generic web URL (priority 1) when YouTube URL (priority 100) matches
    // But keeps similar-priority patterns (e.g., HN ID vs PubMed ID both ~50-80)
    actions
        .into_iter()
        .filter(|a| highest_priority - a.priority <= 20)
        .collect()
}

/// Let user select from multiple matching actions
fn select_action(cli: &Cli, input: &str, actions: Vec<ResolvedAction>) -> Result<ResolvedAction> {
    match cli.output {
        crate::cli::OutputFormat::Pretty => {
            println!();
            println!(
                "{} Input '{}' matches multiple patterns:",
                "Ambiguous:".yellow().bold(),
                input.cyan()
            );
            println!();

            for (i, action) in actions.iter().enumerate() {
                println!(
                    "  [{}] {} → {} ({})",
                    (i + 1).to_string().green().bold(),
                    action.connector.cyan(),
                    action.tool.green(),
                    action.description.dimmed()
                );
            }
            println!();

            print!("Select option [1-{}]: ", actions.len());
            io::stdout().flush()?;

            let mut input = String::new();
            io::stdin().read_line(&mut input)?;

            let selection: usize = input
                .trim()
                .parse()
                .map_err(|_| CommandError::InvalidConfig("Invalid selection".to_string()))?;

            if selection < 1 || selection > actions.len() {
                return Err(CommandError::InvalidConfig(format!(
                    "Selection must be between 1 and {}",
                    actions.len()
                )));
            }

            Ok(actions.into_iter().nth(selection - 1).unwrap())
        }
        // For non-interactive output, just use the first (highest priority) match
        _ => Ok(actions.into_iter().next().unwrap()),
    }
}

/// Execute a resolved action against the registry
async fn execute_action(cli: &Cli, action: &ResolvedAction) -> Result<()> {
    let registry = crate::commands::list::create_registry().await?;

    // Check if connector exists
    let provider = registry.get_provider(&action.connector).ok_or_else(|| {
        CommandError::ConnectorNotFound(format!(
            "Connector '{}' not available. You may need to enable the feature flag.",
            action.connector
        ))
    })?;

    let connector = provider.lock().await;

    // Build the tool request
    // Convert string values to integers for parameters that are typically numeric IDs
    // but keep other string values as-is (e.g., pmid for pubmed should stay as string)
    let arguments = if action.arguments.is_empty() {
        None
    } else {
        let mut args = serde_json::Map::new();
        for (key, value) in &action.arguments {
            // Only convert to integer for specific parameter names that connectors expect as numbers
            // Keep pmid, paper_id, video_id etc. as strings since those connectors expect strings
            let should_convert_to_int =
                key == "id" || key == "number" || key == "item_id" || key == "channel_id";

            if should_convert_to_int {
                if let serde_json::Value::String(s) = value {
                    if let Ok(num) = s.parse::<i64>() {
                        args.insert(key.clone(), serde_json::Value::Number(num.into()));
                        continue;
                    }
                }
            }
            args.insert(key.clone(), value.clone());
        }
        Some(args)
    };

    let request = CallToolRequestParam {
        name: action.tool.clone().into(),
        arguments,
    };

    // Call the tool
    match connector.call_tool(request).await {
        Ok(result) => {
            // Prefer structured_content if present (most connectors use this)
            let (output, json_value) = if let Some(sc) = result.structured_content {
                (OutputData::ToolResult(sc.clone()), sc)
            } else {
                // Fall back to extracting text content from result.content
                let text_content: Vec<String> = result
                    .content
                    .iter()
                    .filter_map(|c| {
                        if let arivu_core::RawContent::Text(t) = &c.raw {
                            Some(t.text.clone())
                        } else {
                            None
                        }
                    })
                    .collect();

                let combined = text_content.join("\n");

                // Try to parse as JSON for pretty output
                if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(&combined) {
                    (OutputData::ToolResult(json_val.clone()), json_val)
                } else {
                    let val = json!({ "content": combined });
                    (OutputData::ToolResult(val.clone()), val)
                }
            };

            format_output(&output, &cli.output)?;

            // Copy to clipboard if requested
            if cli.copy {
                let text = serde_json::to_string_pretty(&json_value)?;
                copy_to_clipboard(&text)?;
            }
        }
        Err(e) => {
            let error_str = e.to_string();

            // Check for auth errors
            if error_str.to_lowercase().contains("auth")
                || error_str.to_lowercase().contains("token")
                || error_str.to_lowercase().contains("credential")
            {
                println!();
                println!(
                    "{} Authentication required for {}",
                    "Error:".red().bold(),
                    action.connector.cyan()
                );
                println!();
                println!(
                    "Run {} to configure authentication.",
                    format!("arivu setup {}", action.connector).cyan()
                );
                println!();
            } else {
                return Err(CommandError::ToolError(error_str));
            }
        }
    }

    Ok(())
}

/// Show all supported formats/patterns
pub async fn show_formats(cli: &Cli) -> Result<()> {
    let resolver = SmartResolver::new();
    let patterns = resolver.list_patterns();

    match cli.output {
        crate::cli::OutputFormat::Pretty => {
            println!();
            println!("{}", "Supported Input Formats".bold().cyan());
            println!("{}", "=======================".cyan());
            println!();
            println!(
                "Use {} to auto-detect and fetch content from these patterns:",
                "arivu fetch <input>".cyan()
            );
            println!();

            // Group by connector
            let mut by_connector: std::collections::HashMap<String, Vec<&PatternInfo>> =
                std::collections::HashMap::new();
            for pattern in &patterns {
                by_connector
                    .entry(pattern.connector.clone())
                    .or_default()
                    .push(pattern);
            }

            // Sort connectors alphabetically
            let mut connectors: Vec<_> = by_connector.keys().collect();
            connectors.sort();

            for connector in connectors {
                let connector_patterns = &by_connector[connector];
                println!("{}", connector.cyan().bold());

                for pattern in connector_patterns {
                    println!("  {} → {}", pattern.example.yellow(), pattern.tool.dimmed());
                }
                println!();
            }

            // Add note about ambiguous patterns
            println!("{}", "Note:".bold());
            println!("  Some inputs (like bare IDs) may match multiple patterns.");
            println!("  In interactive mode, you'll be prompted to choose.");
            println!();
        }
        crate::cli::OutputFormat::Json => {
            let output = OutputData::Patterns(patterns);
            format_output(&output, &cli.output)?;
        }
        _ => {
            for pattern in patterns {
                println!(
                    "{}\t{}\t{}\t{}",
                    pattern.connector, pattern.tool, pattern.example, pattern.description
                );
            }
        }
    }

    Ok(())
}

/// Print a hint about shell quoting if the input looks like a truncated URL
fn print_shell_quoting_hint(input: &str) {
    // Check if input looks like it might have been affected by shell globbing
    // Common signs: URL contains a domain that typically needs query params
    // but the query string is missing (truncated at ?)
    let looks_truncated = input.starts_with("http")
        && !input.contains('?')
        && (input.contains("youtube.com/watch")
            || input.contains("youtu.be/watch")
            || input.contains("news.ycombinator.com/item"));

    if looks_truncated {
        println!(
            "  {} URLs with {} need to be quoted in the shell:",
            "Hint:".cyan().bold(),
            "?".yellow()
        );
        println!(
            "    {} arivu fetch {}",
            "$".dimmed(),
            "\"https://youtube.com/watch?v=VIDEO_ID\"".green()
        );
        println!();
    }
}
