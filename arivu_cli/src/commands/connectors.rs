use crate::cli::Cli;
use crate::commands::Result;
use crate::output::{format_output, OutputData};
use arivu_core::PaginatedRequestParam;
use comfy_table::{modifiers::UTF8_ROUND_CORNERS, presets::UTF8_FULL, ContentArrangement, Table};
use owo_colors::OwoColorize;
use serde_json::{json, Value};

/// Get the terminal width, defaulting to 80 if detection fails
fn get_terminal_width() -> u16 {
    terminal_size::terminal_size()
        .map(|(w, _)| w.0)
        .unwrap_or(80)
}

/// Truncate text to fit within a given width, adding "..." if truncated
fn truncate_text(text: &str, max_width: usize) -> String {
    if text.len() <= max_width {
        text.to_string()
    } else if max_width > 3 {
        format!("{}...", &text[..max_width - 3])
    } else {
        text.chars().take(max_width).collect()
    }
}

pub async fn run(cli: &Cli) -> Result<()> {
    let registry = crate::commands::list::create_registry().await?;
    let providers = registry.list_providers();

    if providers.is_empty() {
        println!("{}", "No connectors available".yellow());
        return Ok(());
    }

    let mut detailed_info = Vec::new();

    // Gather detailed information about each connector
    for provider_info in &providers {
        if let Some(provider) = registry.get_provider(&provider_info.name) {
            let c = provider.lock().await;
            let mut connector_details = json!({
                "name": provider_info.name,
                "description": provider_info.description,
                "status": "unknown",
                "auth_required": false,
                "tools": [],
                "capabilities": {}
            });

            // Test authentication status
            match c.test_auth().await {
                Ok(_) => {
                    connector_details["status"] = json!("ready");
                }
                Err(_) => {
                    // Mark as needs_auth only if any field is actually required
                    let config_schema = c.config_schema();
                    let requires_any = config_schema.fields.iter().any(|f| f.required);
                    if requires_any {
                        connector_details["status"] = json!("needs_auth");
                        connector_details["auth_required"] = json!(true);
                    } else {
                        // Optional auth: surface as ready to avoid false alarms
                        connector_details["status"] = json!("ready");
                        connector_details["auth_required"] = json!(false);
                    }
                }
            }

            // Get available tools
            if let Ok(tools_response) = c
                .list_tools(Some(PaginatedRequestParam { cursor: None }))
                .await
            {
                let tool_names: Vec<String> = tools_response
                    .tools
                    .iter()
                    .map(|tool| tool.name.to_string())
                    .collect();
                connector_details["tools"] = json!(tool_names);
            }

            // Get capabilities
            let capabilities = c.capabilities().await;
            connector_details["capabilities"] = json!({
                "tools": capabilities.tools.is_some(),
                "resources": capabilities.resources.is_some(),
                "prompts": capabilities.prompts.is_some(),
            });

            detailed_info.push(connector_details);
        }
    }

    let output_data = OutputData::ConnectorList(providers.clone());

    match cli.output {
        crate::cli::OutputFormat::Pretty => {
            format_pretty_connectors(&detailed_info)?;
        }
        _ => {
            format_output(&output_data, &cli.output)?;
        }
    }

    Ok(())
}

fn format_pretty_connectors(connectors: &[Value]) -> Result<()> {
    let term_width = get_terminal_width() as usize;

    println!("{}", "Connector Details".bold().cyan());
    println!();

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_width(term_width as u16)
        .set_header(vec!["Name", "Status", "Tools", "Auth", "Description"]);

    // Calculate max description width
    let desc_width = term_width.saturating_sub(55);

    for connector in connectors {
        let name = connector
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        let status = connector
            .get("status")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        let status_display = match status {
            "ready" => "✓ Ready",
            "needs_auth" => "⚠ Setup",
            _ => "? Unknown",
        };

        let auth_required = connector
            .get("auth_required")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let auth_display = if auth_required { "Required" } else { "None" };

        let tools = connector
            .get("tools")
            .and_then(|v| v.as_array())
            .map(|arr| arr.len().to_string())
            .unwrap_or_else(|| "0".to_string());

        let description = connector
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        table.add_row(vec![
            name.to_string(),
            status_display.to_string(),
            tools,
            auth_display.to_string(),
            truncate_text(description, desc_width.max(30)),
        ]);
    }

    println!("{}", table);
    println!();

    // Show categorized connectors
    print_connector_categories(connectors)?;

    // Show usage tips
    println!("{}", "Usage Tips:".bold().green());
    println!(
        "  {} - List available tools for a connector",
        "arivu tools <connector>".cyan()
    );
    println!(
        "  {} - Configure authentication",
        "arivu config set <connector>".cyan()
    );
    println!(
        "  {} - Test authentication",
        "arivu config test <connector>".cyan()
    );
    println!(
        "  {} - Search using a connector",
        "arivu search <connector> <query>".cyan()
    );

    Ok(())
}

fn print_connector_categories(connectors: &[Value]) -> Result<()> {
    let categories = vec![
        ("🎥 Media & Entertainment", vec!["youtube", "reddit"]),
        (
            "🔍 Search & Discovery",
            vec![
                "bing_search",
                "openai-search",
                "anthropic-search",
                "gemini-search",
                "perplexity-search",
                "xai-search",
                "exa-search",
                "firecrawl-search",
                "serper-search",
                "tavily-search",
                "serpapi-search",
            ],
        ),
        (
            "📚 Academic & Research",
            vec!["arxiv", "pubmed", "semantic_scholar", "scihub"],
        ),
        ("🌐 Web & Social", vec!["x", "hackernews", "wikipedia"]),
        ("🛠️ Web Scraping", vec!["web", "web_chrome"]),
        (
            "🗂️ Productivity & Cloud",
            vec![
                "microsoft-graph",
                "google-drive",
                "google-gmail",
                "google-calendar",
                "google-people",
            ],
        ),
    ];

    for (category, connector_names) in categories {
        let mut found_connectors = Vec::new();

        for connector in connectors {
            if let Some(name) = connector.get("name").and_then(|v| v.as_str()) {
                if connector_names.contains(&name) {
                    let status = connector
                        .get("status")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown");

                    let status_icon = match status {
                        "ready" => "✓",
                        "needs_auth" => "⚠",
                        _ => "?",
                    };

                    found_connectors.push((name, status_icon));
                }
            }
        }

        if !found_connectors.is_empty() {
            println!("{}", category.bold());
            for (name, status) in found_connectors {
                println!("  {} {}", status, name.cyan());
            }
            println!();
        }
    }

    Ok(())
}
