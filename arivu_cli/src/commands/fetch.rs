use crate::cli::Cli;
use crate::commands::{CommandError, Result};
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
        println!("Run {} to see supported formats.", "arivu formats".cyan());
        println!();
        return Ok(());
    }

    // If only one match, use it directly
    let action = if actions.len() == 1 {
        actions.into_iter().next().unwrap()
    } else {
        // Multiple matches - let user choose
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
        println!();
    }

    // Execute the action
    execute_action(cli, &action).await
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

    // Build the tool request - convert numeric strings to numbers where needed
    let arguments = if action.arguments.is_empty() {
        None
    } else {
        let mut args = serde_json::Map::new();
        for (key, value) in &action.arguments {
            // Try to parse string values as integers for numeric parameters
            if let serde_json::Value::String(s) = value {
                if let Ok(num) = s.parse::<i64>() {
                    args.insert(key.clone(), serde_json::Value::Number(num.into()));
                } else {
                    args.insert(key.clone(), value.clone());
                }
            } else {
                args.insert(key.clone(), value.clone());
            }
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
            // Extract text content from result
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
            let output =
                if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(&combined) {
                    OutputData::ToolResult(json_value)
                } else {
                    OutputData::ToolResult(json!({ "content": combined }))
                };

            format_output(&output, &cli.output)?;
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
