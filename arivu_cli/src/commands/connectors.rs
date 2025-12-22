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

// ============================================================================
// Connector-specific command handlers with proper CLI flags
// ============================================================================

use crate::cli::{
    AnthropicSearchTools, ArxivTools, AtlassianTools, BiorxivTools, DiscordTools, ExaTools,
    FirecrawlSearchTools, GeminiSearchTools, GithubTools, GoogleCalendarTools, GoogleDriveTools,
    GoogleGmailTools, GooglePeopleTools, GoogleScholarTools, HackernewsTools, ImapTools,
    LocalfsTools, MacosTools, MicrosoftGraphTools, OpenaiSearchTools, ParallelSearchTools,
    PerplexitySearchTools, PubmedTools, RedditTools, RssTools, ScihubTools, SemanticScholarTools,
    SerpapiSearchTools, SerperSearchTools, SlackTools, SpotlightTools, TavilySearchTools, WebTools,
    WikipediaTools, XTools, XaiSearchTools, YoutubeArgs, YoutubeTools,
};
use crate::commands::copy_to_clipboard;
use crate::commands::usage_helpers::print_cost_summary;
use arivu_core::CallToolRequestParam;
use serde_json::Map;

async fn call_tool_raw(
    connector: &str,
    tool: &str,
    args: Map<String, Value>,
) -> Result<(Value, Option<Value>)> {
    let registry = crate::commands::list::create_registry().await?;
    let provider = registry
        .get_provider(connector)
        .ok_or_else(|| crate::commands::CommandError::ConnectorNotFound(connector.to_string()))?;

    let c = provider.lock().await;

    // Validate tool exists and required arguments are present.
    // This prevents the CLI wrappers from silently drifting away from core tool names/schemas.
    let tools_response = c
        .list_tools(Some(PaginatedRequestParam { cursor: None }))
        .await?;
    let tool_def = tools_response
        .tools
        .iter()
        .find(|t| t.name.as_ref() == tool)
        .ok_or_else(|| {
            crate::commands::CommandError::ToolNotFound(tool.to_string(), connector.to_string())
        })?;

    if let Some(required) = tool_def
        .input_schema
        .get("required")
        .and_then(|v| v.as_array())
    {
        let missing: Vec<String> = required
            .iter()
            .filter_map(|v| v.as_str())
            .filter(|k| !args.contains_key(*k))
            .map(ToString::to_string)
            .collect();
        if !missing.is_empty() {
            return Err(crate::commands::CommandError::InvalidInput(format!(
                "Missing required args for {}.{}: {}",
                connector,
                tool,
                missing.join(", ")
            )));
        }
    }

    let request = CallToolRequestParam {
        name: tool.to_string().into(),
        arguments: Some(args.into_iter().collect()),
    };

    let result = c.call_tool(request).await?;

    let meta_value = result
        .meta
        .as_ref()
        .and_then(|m| serde_json::to_value(m).ok());

    let payload = if let Some(sc) = result.structured_content {
        sc
    } else {
        serde_json::to_value(&result).unwrap_or_else(|_| json!({"ok": true}))
    };

    Ok((payload, meta_value))
}

fn output_tool_result(
    cli: &Cli,
    connector: &str,
    tool: &str,
    payload: &Value,
    meta_value: Option<&Value>,
) -> Result<()> {
    match cli.output {
        crate::cli::OutputFormat::Pretty => {
            println!(
                "{} {}.{}",
                "Tool".bold().cyan(),
                connector.yellow(),
                tool.cyan()
            );
            println!();
            println!("{}", crate::output::format_pretty(payload));
        }
        _ => {
            let data = OutputData::CallResult {
                connector: connector.to_string(),
                tool: tool.to_string(),
                result: payload.clone(),
                meta: meta_value.cloned(),
            };
            format_output(&data, &cli.output)?;
        }
    }

    if cli.copy {
        let text = serde_json::to_string_pretty(payload)?;
        copy_to_clipboard(&text)?;
    }

    print_cost_summary(&cli.output, meta_value);

    Ok(())
}

/// Helper to call a connector tool with JSON args
async fn call_tool(cli: &Cli, connector: &str, tool: &str, args: Map<String, Value>) -> Result<()> {
    let (payload, meta_value) = call_tool_raw(connector, tool, args).await?;
    output_tool_result(cli, connector, tool, &payload, meta_value.as_ref())
}

/// Handle OpenAI Search commands
pub async fn handle_openai_search(cli: &Cli, tool: OpenaiSearchTools) -> Result<()> {
    let (tool_name, args) = match tool {
        OpenaiSearchTools::Search {
            query,
            limit,
            model,
            response_format,
        } => {
            let mut args = Map::new();
            args.insert("query".to_string(), json!(query));
            args.insert("limit".to_string(), json!(limit));
            if let Some(m) = model {
                args.insert("model".to_string(), json!(m));
            }
            args.insert("response_format".to_string(), json!(response_format));
            ("search", args)
        }
    };

    call_tool(cli, "openai-search", tool_name, args).await
}

/// Handle Anthropic Search commands
pub async fn handle_anthropic_search(cli: &Cli, tool: AnthropicSearchTools) -> Result<()> {
    let (tool_name, args) = match tool {
        AnthropicSearchTools::Search {
            query,
            limit,
            model,
            response_format,
        } => {
            let mut args = Map::new();
            args.insert("query".to_string(), json!(query));
            args.insert("limit".to_string(), json!(limit));
            if let Some(m) = model {
                args.insert("model".to_string(), json!(m));
            }
            args.insert("response_format".to_string(), json!(response_format));
            ("search", args)
        }
    };

    call_tool(cli, "anthropic-search", tool_name, args).await
}

/// Handle Gemini Search commands
pub async fn handle_gemini_search(cli: &Cli, tool: GeminiSearchTools) -> Result<()> {
    let (tool_name, args) = match tool {
        GeminiSearchTools::Search {
            query,
            limit,
            model,
            response_format,
        } => {
            let mut args = Map::new();
            args.insert("query".to_string(), json!(query));
            args.insert("limit".to_string(), json!(limit));
            if let Some(m) = model {
                args.insert("model".to_string(), json!(m));
            }
            args.insert("response_format".to_string(), json!(response_format));
            ("search", args)
        }
    };

    call_tool(cli, "gemini-search", tool_name, args).await
}

/// Handle Perplexity Search commands
pub async fn handle_perplexity_search(cli: &Cli, tool: PerplexitySearchTools) -> Result<()> {
    let (tool_name, args) = match tool {
        PerplexitySearchTools::Search {
            query,
            limit,
            model,
            response_format,
        } => {
            let mut args = Map::new();
            args.insert("query".to_string(), json!(query));
            args.insert("limit".to_string(), json!(limit));
            if let Some(m) = model {
                args.insert("model".to_string(), json!(m));
            }
            args.insert("response_format".to_string(), json!(response_format));
            ("search", args)
        }
    };

    call_tool(cli, "perplexity-search", tool_name, args).await
}

/// Handle xAI Search commands
pub async fn handle_xai_search(cli: &Cli, tool: XaiSearchTools) -> Result<()> {
    let (tool_name, args) = match tool {
        XaiSearchTools::Search {
            query,
            limit,
            model,
            response_format,
        } => {
            let mut args = Map::new();
            args.insert("query".to_string(), json!(query));
            args.insert("limit".to_string(), json!(limit));
            if let Some(m) = model {
                args.insert("model".to_string(), json!(m));
            }
            args.insert("response_format".to_string(), json!(response_format));
            ("search", args)
        }
    };

    call_tool(cli, "xai-search", tool_name, args).await
}

/// Handle Exa commands
pub async fn handle_exa(cli: &Cli, tool: ExaTools) -> Result<()> {
    let (tool_name, args) = match tool {
        ExaTools::Search {
            query,
            limit,
            type_,
            response_format,
        } => {
            let mut args = Map::new();
            args.insert("query".to_string(), json!(query));
            args.insert("limit".to_string(), json!(limit));
            args.insert("type".to_string(), json!(type_));
            args.insert("response_format".to_string(), json!(response_format));
            ("search", args)
        }
        ExaTools::GetContents { ids } => {
            let mut args = Map::new();
            let ids_array: Vec<String> = ids.split(',').map(|s| s.trim().to_string()).collect();
            args.insert("ids".to_string(), json!(ids_array));
            ("get_contents", args)
        }
        ExaTools::FindSimilar { url, limit } => {
            let mut args = Map::new();
            args.insert("url".to_string(), json!(url));
            args.insert("limit".to_string(), json!(limit));
            ("find_similar", args)
        }
        ExaTools::Answer { query, mode } => {
            let mut args = Map::new();
            args.insert("query".to_string(), json!(query));
            if let Some(m) = mode {
                args.insert("mode".to_string(), json!(m));
            }
            ("answer", args)
        }
    };

    call_tool(cli, "exa", tool_name, args).await
}

/// Handle Tavily Search commands
pub async fn handle_tavily_search(cli: &Cli, tool: TavilySearchTools) -> Result<()> {
    let (tool_name, args) = match tool {
        TavilySearchTools::Search {
            query,
            limit,
            depth,
            response_format,
        } => {
            let mut args = Map::new();
            args.insert("query".to_string(), json!(query));
            args.insert("limit".to_string(), json!(limit));
            args.insert("depth".to_string(), json!(depth));
            args.insert("response_format".to_string(), json!(response_format));
            ("search", args)
        }
    };

    call_tool(cli, "tavily-search", tool_name, args).await
}

/// Handle Serper Search commands
pub async fn handle_serper_search(cli: &Cli, tool: SerperSearchTools) -> Result<()> {
    let (tool_name, args) = match tool {
        SerperSearchTools::Search {
            query,
            limit,
            response_format,
        } => {
            let mut args = Map::new();
            args.insert("query".to_string(), json!(query));
            args.insert("limit".to_string(), json!(limit));
            args.insert("response_format".to_string(), json!(response_format));
            ("search", args)
        }
    };

    call_tool(cli, "serper-search", tool_name, args).await
}

/// Handle SerpAPI Search commands
pub async fn handle_serpapi_search(cli: &Cli, tool: SerpapiSearchTools) -> Result<()> {
    let (tool_name, args) = match tool {
        SerpapiSearchTools::Search {
            query,
            limit,
            engine,
            response_format,
        } => {
            let mut args = Map::new();
            args.insert("query".to_string(), json!(query));
            args.insert("limit".to_string(), json!(limit));
            args.insert("engine".to_string(), json!(engine));
            args.insert("response_format".to_string(), json!(response_format));
            ("search", args)
        }
    };

    call_tool(cli, "serpapi-search", tool_name, args).await
}

/// Handle Firecrawl Search commands
pub async fn handle_firecrawl_search(cli: &Cli, tool: FirecrawlSearchTools) -> Result<()> {
    let (tool_name, args) = match tool {
        FirecrawlSearchTools::Search {
            query,
            limit,
            scrape,
            response_format,
        } => {
            let mut args = Map::new();
            args.insert("query".to_string(), json!(query));
            args.insert("limit".to_string(), json!(limit));
            args.insert("scrape".to_string(), json!(scrape));
            args.insert("response_format".to_string(), json!(response_format));
            ("search", args)
        }
    };

    call_tool(cli, "firecrawl-search", tool_name, args).await
}

/// Handle Parallel Search commands
pub async fn handle_parallel_search(cli: &Cli, tool: ParallelSearchTools) -> Result<()> {
    let (tool_name, args) = match tool {
        ParallelSearchTools::Search { query, limit } => {
            let mut args = Map::new();
            args.insert("query".to_string(), json!(query));
            args.insert("limit".to_string(), json!(limit));
            ("search", args)
        }
    };

    call_tool(cli, "parallel-search", tool_name, args).await
}

/// Handle Google Calendar commands
pub async fn handle_google_calendar(cli: &Cli, tool: GoogleCalendarTools) -> Result<()> {
    let (tool_name, args) = match tool {
        GoogleCalendarTools::ListEvents {
            max_results,
            time_min,
            response_format,
        } => {
            let mut args = Map::new();
            args.insert("max_results".to_string(), json!(max_results));
            if let Some(time) = time_min {
                args.insert("time_min".to_string(), json!(time));
            }
            args.insert("response_format".to_string(), json!(response_format));
            ("list_events", args)
        }
        GoogleCalendarTools::CreateEvent {
            summary,
            start,
            end,
        } => {
            let mut args = Map::new();
            args.insert("summary".to_string(), json!(summary));
            args.insert("start".to_string(), json!(start));
            args.insert("end".to_string(), json!(end));
            ("create_event", args)
        }
        GoogleCalendarTools::SyncEvents {
            sync_token,
            max_results,
        } => {
            let mut args = Map::new();
            args.insert("sync_token".to_string(), json!(sync_token));
            args.insert("max_results".to_string(), json!(max_results));
            ("sync_events", args)
        }
        GoogleCalendarTools::UpdateEvent {
            event_id,
            summary,
            start,
            end,
        } => {
            let mut args = Map::new();
            args.insert("event_id".to_string(), json!(event_id));
            if let Some(s) = summary {
                args.insert("summary".to_string(), json!(s));
            }
            if let Some(st) = start {
                args.insert("start".to_string(), json!(st));
            }
            if let Some(e) = end {
                args.insert("end".to_string(), json!(e));
            }
            ("update_event", args)
        }
        GoogleCalendarTools::DeleteEvent { event_id } => {
            let mut args = Map::new();
            if let Some(id) = event_id {
                args.insert("event_id".to_string(), json!(id));
            }
            ("delete_event", args)
        }
    };

    call_tool(cli, "google-calendar", tool_name, args).await
}

/// Handle Google Drive commands
pub async fn handle_google_drive(cli: &Cli, tool: GoogleDriveTools) -> Result<()> {
    let (tool_name, args) = match tool {
        GoogleDriveTools::ListFiles {
            q,
            page_size,
            response_format,
        } => {
            let mut args = Map::new();
            if let Some(query) = q {
                args.insert("q".to_string(), json!(query));
            }
            args.insert("page_size".to_string(), json!(page_size));
            args.insert("response_format".to_string(), json!(response_format));
            ("list_files", args)
        }
        GoogleDriveTools::GetFile {
            file_id,
            response_format,
        } => {
            let mut args = Map::new();
            args.insert("file_id".to_string(), json!(file_id));
            args.insert("response_format".to_string(), json!(response_format));
            ("get_file", args)
        }
        GoogleDriveTools::DownloadFile { file_id, max_bytes } => {
            let mut args = Map::new();
            args.insert("file_id".to_string(), json!(file_id));
            if let Some(mb) = max_bytes {
                args.insert("max_bytes".to_string(), json!(mb));
            }
            ("download_file", args)
        }
        GoogleDriveTools::ExportFile { file_id, mime_type } => {
            let mut args = Map::new();
            args.insert("file_id".to_string(), json!(file_id));
            args.insert("mime_type".to_string(), json!(mime_type));
            ("export_file", args)
        }
        GoogleDriveTools::UploadFile {
            name,
            mime_type,
            data_base64,
            parents,
        } => {
            let mut args = Map::new();
            args.insert("name".to_string(), json!(name));
            args.insert("mime_type".to_string(), json!(mime_type));
            args.insert("data_base64".to_string(), json!(data_base64));
            if let Some(p) = parents {
                let parents_vec: Vec<String> = p.split(',').map(|s| s.trim().to_string()).collect();
                args.insert("parents".to_string(), json!(parents_vec));
            }
            ("upload_file", args)
        }
        GoogleDriveTools::UploadFileResumable {
            name,
            mime_type,
            data_base64,
            parents,
        } => {
            let mut args = Map::new();
            args.insert("name".to_string(), json!(name));
            args.insert("mime_type".to_string(), json!(mime_type));
            args.insert("data_base64".to_string(), json!(data_base64));
            if let Some(p) = parents {
                let parents_vec: Vec<String> = p.split(',').map(|s| s.trim().to_string()).collect();
                args.insert("parents".to_string(), json!(parents_vec));
            }
            ("upload_file_resumable", args)
        }
    };

    call_tool(cli, "google-drive", tool_name, args).await
}

/// Handle Google Gmail commands
pub async fn handle_google_gmail(cli: &Cli, tool: GoogleGmailTools) -> Result<()> {
    let (tool_name, args) = match tool {
        GoogleGmailTools::ListMessages {
            q,
            max_results,
            response_format,
        } => {
            let mut args = Map::new();
            if let Some(query) = q {
                args.insert("q".to_string(), json!(query));
            }
            args.insert("max_results".to_string(), json!(max_results));
            args.insert("response_format".to_string(), json!(response_format));
            ("list_messages", args)
        }
        GoogleGmailTools::DecodeMessageRaw { raw_base64url } => {
            let mut args = Map::new();
            args.insert("raw_base64url".to_string(), json!(raw_base64url));
            ("decode_message_raw", args)
        }
        GoogleGmailTools::GetMessage {
            id,
            format,
            response_format,
        } => {
            let mut args = Map::new();
            args.insert("id".to_string(), json!(id));
            args.insert("format".to_string(), json!(format));
            args.insert("response_format".to_string(), json!(response_format));
            ("get_message", args)
        }
        GoogleGmailTools::GetThread { id } => {
            let mut args = Map::new();
            args.insert("id".to_string(), json!(id));
            ("get_thread", args)
        }
    };

    call_tool(cli, "google-gmail", tool_name, args).await
}

/// Handle Google People commands
pub async fn handle_google_people(cli: &Cli, tool: GooglePeopleTools) -> Result<()> {
    let (tool_name, args) = match tool {
        GooglePeopleTools::ListConnections {
            page_size,
            response_format,
        } => {
            let mut args = Map::new();
            args.insert("page_size".to_string(), json!(page_size));
            args.insert("response_format".to_string(), json!(response_format));
            ("list_connections", args)
        }
        GooglePeopleTools::GetPerson {
            resource_name,
            person_fields,
            response_format,
        } => {
            let mut args = Map::new();
            args.insert("resource_name".to_string(), json!(resource_name));
            if let Some(fields) = person_fields {
                args.insert("person_fields".to_string(), json!(fields));
            }
            args.insert("response_format".to_string(), json!(response_format));
            ("get_person", args)
        }
    };

    call_tool(cli, "google-people", tool_name, args).await
}

/// Handle Google Scholar commands
pub async fn handle_google_scholar(cli: &Cli, tool: GoogleScholarTools) -> Result<()> {
    let (tool_name, args) = match tool {
        GoogleScholarTools::SearchPapers { query, limit } => {
            let mut args = Map::new();
            args.insert("query".to_string(), json!(query));
            args.insert("limit".to_string(), json!(limit));
            ("search_papers", args)
        }
    };

    call_tool(cli, "google-scholar", tool_name, args).await
}

/// Handle Atlassian commands
pub async fn handle_atlassian(cli: &Cli, tool: AtlassianTools) -> Result<()> {
    let (tool_name, args) = match tool {
        AtlassianTools::TestAuth => ("test_auth", Map::new()),
        AtlassianTools::JiraSearch {
            jql,
            start_at,
            max_results,
            fields,
        } => {
            let mut args = Map::new();
            args.insert("jql".to_string(), json!(jql));
            if start_at > 0 {
                args.insert("start_at".to_string(), json!(start_at));
            }
            if max_results != 50 {
                args.insert("max_results".to_string(), json!(max_results));
            }
            if let Some(f) = fields {
                args.insert("fields".to_string(), json!(f));
            }
            ("jira_search_issues", args)
        }
        AtlassianTools::JiraGet { key, expand } => {
            let mut args = Map::new();
            args.insert("key".to_string(), json!(key));
            if let Some(e) = expand {
                args.insert("expand".to_string(), json!(e));
            }
            ("jira_get_issue", args)
        }
        AtlassianTools::ConfSearch { cql, start, limit } => {
            let mut args = Map::new();
            args.insert("cql".to_string(), json!(cql));
            if start > 0 {
                args.insert("start".to_string(), json!(start));
            }
            if limit != 25 {
                args.insert("limit".to_string(), json!(limit));
            }
            ("conf_search_pages", args)
        }
        AtlassianTools::ConfGet { id, expand } => {
            let mut args = Map::new();
            args.insert("id".to_string(), json!(id));
            if let Some(e) = expand {
                args.insert("expand".to_string(), json!(e));
            }
            ("conf_get_page", args)
        }
    };

    call_tool(cli, "atlassian", tool_name, args).await
}

/// Handle Microsoft Graph commands
pub async fn handle_microsoft_graph(cli: &Cli, tool: MicrosoftGraphTools) -> Result<()> {
    let (tool_name, args) = match tool {
        MicrosoftGraphTools::ListMessages {
            top,
            response_format,
        } => {
            let mut args = Map::new();
            if top != 20 {
                args.insert("top".to_string(), json!(top));
            }
            if response_format != "concise" {
                args.insert("response_format".to_string(), json!(response_format));
            }
            ("list_messages", args)
        }
        MicrosoftGraphTools::ListEvents {
            days_ahead,
            response_format,
        } => {
            let mut args = Map::new();
            if days_ahead != 7 {
                args.insert("days_ahead".to_string(), json!(days_ahead));
            }
            if response_format != "concise" {
                args.insert("response_format".to_string(), json!(response_format));
            }
            ("list_events", args)
        }
        MicrosoftGraphTools::GetMessage { message_id } => {
            let mut args = Map::new();
            args.insert("message_id".to_string(), json!(message_id));
            ("get_message", args)
        }
        MicrosoftGraphTools::SendMail { to, subject, body } => {
            let mut args = Map::new();
            // Parse comma-separated emails into array
            let to_array: Vec<String> = to.split(',').map(|s| s.trim().to_string()).collect();
            args.insert("to".to_string(), json!(to_array));
            args.insert("subject".to_string(), json!(subject));
            args.insert("body_text".to_string(), json!(body));
            ("send_mail", args)
        }
        MicrosoftGraphTools::CreateDraft { to, subject, body } => {
            let mut args = Map::new();
            // Parse comma-separated emails into array
            let to_array: Vec<String> = to.split(',').map(|s| s.trim().to_string()).collect();
            args.insert("to".to_string(), json!(to_array));
            args.insert("subject".to_string(), json!(subject));
            args.insert("body_text".to_string(), json!(body));
            ("create_draft", args)
        }
        MicrosoftGraphTools::UploadAttachment {
            message_id,
            filename,
            mime_type,
            data_base64,
        } => {
            let mut args = Map::new();
            args.insert("message_id".to_string(), json!(message_id));
            args.insert("filename".to_string(), json!(filename));
            args.insert("mime_type".to_string(), json!(mime_type));
            args.insert("data_base64".to_string(), json!(data_base64));
            ("upload_attachment_large", args)
        }
        MicrosoftGraphTools::SendDraft { message_id } => {
            let mut args = Map::new();
            args.insert("message_id".to_string(), json!(message_id));
            ("send_draft", args)
        }
        MicrosoftGraphTools::UploadAttachmentFromPath {
            message_id,
            file_path,
            filename,
            mime_type,
        } => {
            let mut args = Map::new();
            args.insert("message_id".to_string(), json!(message_id));
            args.insert("file_path".to_string(), json!(file_path));
            if let Some(f) = filename {
                args.insert("filename".to_string(), json!(f));
            }
            if let Some(m) = mime_type {
                args.insert("mime_type".to_string(), json!(m));
            }
            ("upload_attachment_large_from_path", args)
        }
        MicrosoftGraphTools::AuthStart {
            tenant_id,
            client_id,
            scopes,
        } => {
            let mut args = Map::new();
            if let Some(t) = tenant_id {
                args.insert("tenant_id".to_string(), json!(t));
            }
            if let Some(c) = client_id {
                args.insert("client_id".to_string(), json!(c));
            }
            if let Some(s) = scopes {
                args.insert("scopes".to_string(), json!(s));
            }
            ("auth_start", args)
        }
        MicrosoftGraphTools::AuthPoll {
            tenant_id,
            client_id,
            device_code,
        } => {
            let mut args = Map::new();
            if let Some(t) = tenant_id {
                args.insert("tenant_id".to_string(), json!(t));
            }
            args.insert("client_id".to_string(), json!(client_id));
            args.insert("device_code".to_string(), json!(device_code));
            ("auth_poll", args)
        }
    };

    call_tool(cli, "microsoft-graph", tool_name, args).await
}

/// Handle IMAP commands
pub async fn handle_imap(cli: &Cli, tool: ImapTools) -> Result<()> {
    let (tool_name, args) = match tool {
        ImapTools::ListMailboxes {
            reference,
            pattern,
            include_subscribed,
        } => {
            let mut args = Map::new();
            if let Some(r) = reference {
                args.insert("reference".to_string(), json!(r));
            }
            if pattern != "*" {
                args.insert("pattern".to_string(), json!(pattern));
            }
            if include_subscribed {
                args.insert("include_subscribed".to_string(), json!(include_subscribed));
            }
            ("list_mailboxes", args)
        }
        ImapTools::FetchMessages { mailbox, limit } => {
            let mut args = Map::new();
            if let Some(m) = mailbox {
                args.insert("mailbox".to_string(), json!(m));
            }
            if limit != 20 {
                args.insert("limit".to_string(), json!(limit));
            }
            ("fetch_messages", args)
        }
        ImapTools::GetMessage {
            mailbox,
            uid,
            include_raw,
        } => {
            let mut args = Map::new();
            if let Some(m) = mailbox {
                args.insert("mailbox".to_string(), json!(m));
            }
            args.insert("uid".to_string(), json!(uid));
            if include_raw {
                args.insert("include_raw".to_string(), json!(include_raw));
            }
            ("get_message", args)
        }
        ImapTools::Search {
            mailbox,
            query,
            limit,
        } => {
            let mut args = Map::new();
            if let Some(m) = mailbox {
                args.insert("mailbox".to_string(), json!(m));
            }
            args.insert("query".to_string(), json!(query));
            if limit != 50 {
                args.insert("limit".to_string(), json!(limit));
            }
            ("search", args)
        }
    };

    call_tool(cli, "imap", tool_name, args).await
}

/// Handle localfs commands
pub async fn handle_localfs(cli: &Cli, tool: LocalfsTools) -> Result<()> {
    let (tool_name, args) = match tool {
        LocalfsTools::ListFiles {
            path,
            recursive,
            extensions,
            limit,
        } => {
            let mut args = Map::new();
            args.insert("path".to_string(), json!(path));
            args.insert("recursive".to_string(), json!(recursive));
            if let Some(ext) = extensions {
                args.insert("extensions".to_string(), json!(ext));
            }
            args.insert("limit".to_string(), json!(limit));
            ("list_files", args)
        }
        LocalfsTools::FileInfo { path } => {
            let mut args = Map::new();
            args.insert("path".to_string(), json!(path));
            ("get_file_info", args)
        }
        LocalfsTools::Structure { path } => {
            let mut args = Map::new();
            args.insert("path".to_string(), json!(path));
            ("get_structure", args)
        }
        LocalfsTools::ExtractText {
            path,
            format,
            max_chars,
        } => {
            let mut args = Map::new();
            args.insert("path".to_string(), json!(path));
            args.insert("format".to_string(), json!(format));
            if let Some(m) = max_chars {
                args.insert("max_chars".to_string(), json!(m));
            }
            ("extract_text", args)
        }
        LocalfsTools::Section {
            path,
            section,
            max_chars,
        } => {
            let mut args = Map::new();
            args.insert("path".to_string(), json!(path));
            args.insert("section".to_string(), json!(section));
            if let Some(m) = max_chars {
                args.insert("max_chars".to_string(), json!(m));
            }
            ("get_section", args)
        }
        LocalfsTools::Search {
            path,
            query,
            context,
        } => {
            let mut args = Map::new();
            args.insert("path".to_string(), json!(path));
            args.insert("query".to_string(), json!(query));
            args.insert("context_lines".to_string(), json!(context));
            ("search_content", args)
        }
    };

    call_tool(cli, "localfs", tool_name, args).await
}

/// Handle youtube commands
pub async fn handle_youtube(cli: &Cli, args: YoutubeArgs) -> Result<()> {
    let tool = match args.command {
        Some(t) => t,
        None => YoutubeTools::Get {
            id_or_url: args.id_or_url,
            id: None,
        },
    };

    match tool {
        YoutubeTools::Search { query, limit } => {
            let mut tool_args = Map::new();
            tool_args.insert("query".to_string(), json!(query));
            tool_args.insert("limit".to_string(), json!(limit));
            call_tool(cli, "youtube", "search", tool_args).await
        }
        YoutubeTools::List {
            channel,
            playlist,
            limit,
            within_days,
            published_after,
        } => {
            let mut tool_args = Map::new();
            tool_args.insert(
                "source".to_string(),
                json!(if channel.is_some() {
                    "channel"
                } else {
                    "playlist"
                }),
            );
            if let Some(ch) = channel {
                tool_args.insert("channel".to_string(), json!(ch));
            }
            if let Some(pl) = playlist {
                tool_args.insert("playlist".to_string(), json!(pl));
            }
            tool_args.insert("limit".to_string(), json!(limit));
            if let Some(d) = within_days {
                tool_args.insert("published_within_days".to_string(), json!(d));
            }
            if let Some(pa) = published_after {
                tool_args.insert("published_after".to_string(), json!(pa));
            }
            call_tool(cli, "youtube", "list", tool_args).await
        }
        YoutubeTools::ResolveChannel {
            query,
            channel,
            limit,
            prefer_verified,
        } => {
            let mut tool_args = Map::new();
            if let Some(q) = query {
                tool_args.insert("query".to_string(), json!(q));
            }
            if let Some(ch) = channel {
                tool_args.insert("channel".to_string(), json!(ch));
            }
            tool_args.insert("limit".to_string(), json!(limit));
            tool_args.insert("prefer_verified".to_string(), json!(prefer_verified));
            call_tool(cli, "youtube", "resolve_channel", tool_args).await
        }
        YoutubeTools::Get { id_or_url, id } => {
            let id = id_or_url.or(id).ok_or_else(|| {
                crate::commands::CommandError::InvalidInput(
                    "Missing video ID/URL. Provide `arivu youtube <ID_OR_URL>` or `arivu youtube get --id <ID_OR_URL>`.".to_string(),
                )
            })?;

            let mut tool_args = Map::new();
            tool_args.insert("video_id".to_string(), json!(id));
            tool_args.insert("response_format".to_string(), json!("detailed"));
            call_tool(cli, "youtube", "get", tool_args).await
        }
        YoutubeTools::Transcript { id_or_url, id } => {
            let id = id_or_url.or(id).ok_or_else(|| {
                crate::commands::CommandError::InvalidInput(
                    "Missing video ID/URL. Use `arivu youtube get`.".to_string(),
                )
            })?;

            let mut tool_args = Map::new();
            tool_args.insert("video_id".to_string(), json!(id));
            tool_args.insert("response_format".to_string(), json!("concise"));

            let (payload, meta_value) = call_tool_raw("youtube", "get", tool_args).await?;
            let transcript_only = payload.get("transcript").cloned().unwrap_or(Value::Null);
            output_tool_result(
                cli,
                "youtube",
                "transcript",
                &transcript_only,
                meta_value.as_ref(),
            )
        }
        YoutubeTools::Chapters { id_or_url, id } => {
            let id = id_or_url.or(id).ok_or_else(|| {
                crate::commands::CommandError::InvalidInput(
                    "Missing video ID/URL. Use `arivu youtube get`.".to_string(),
                )
            })?;

            let mut tool_args = Map::new();
            tool_args.insert("video_id".to_string(), json!(id));
            tool_args.insert("response_format".to_string(), json!("concise"));

            let (payload, meta_value) = call_tool_raw("youtube", "get", tool_args).await?;
            let chapters_only = payload
                .get("chapters")
                .cloned()
                .unwrap_or(Value::Array(Vec::new()));
            output_tool_result(
                cli,
                "youtube",
                "chapters",
                &chapters_only,
                meta_value.as_ref(),
            )
        }
    }
}

/// Handle hackernews commands
pub async fn handle_hackernews(cli: &Cli, tool: HackernewsTools) -> Result<()> {
    let (tool_name, args) = match tool {
        HackernewsTools::Search { query, limit } => {
            let mut args = Map::new();
            args.insert("query".to_string(), json!(query));
            args.insert("hitsPerPage".to_string(), json!(limit));
            ("search_stories", args)
        }
        HackernewsTools::Story { id } => {
            let mut args = Map::new();
            args.insert("id".to_string(), json!(id));
            ("get_post", args)
        }
        HackernewsTools::Top { limit } => {
            let mut args = Map::new();
            args.insert("story_type".to_string(), json!("top"));
            args.insert("limit".to_string(), json!(limit));
            ("get_stories", args)
        }
        HackernewsTools::New { limit } => {
            let mut args = Map::new();
            args.insert("story_type".to_string(), json!("new"));
            args.insert("limit".to_string(), json!(limit));
            ("get_stories", args)
        }
        HackernewsTools::Best { limit } => {
            let mut args = Map::new();
            args.insert("story_type".to_string(), json!("best"));
            args.insert("limit".to_string(), json!(limit));
            ("get_stories", args)
        }
        HackernewsTools::Comments { id, limit: _ } => {
            let mut args = Map::new();
            args.insert("id".to_string(), json!(id));
            args.insert("flatten".to_string(), json!(true));
            ("get_post", args)
        }
    };

    call_tool(cli, "hackernews", tool_name, args).await
}

/// Handle arxiv commands
pub async fn handle_arxiv(cli: &Cli, tool: ArxivTools) -> Result<()> {
    let (tool_name, args) = match tool {
        ArxivTools::Search { query, limit, sort } => {
            let mut args = Map::new();
            args.insert("query".to_string(), json!(query));
            args.insert("limit".to_string(), json!(limit));
            args.insert("sort_by".to_string(), json!(sort));
            ("search", args)
        }
        ArxivTools::Paper { id } => {
            let mut args = Map::new();
            args.insert("paper_id".to_string(), json!(id));
            args.insert("response_format".to_string(), json!("detailed"));
            ("get", args)
        }
        ArxivTools::Pdf { id } => {
            let mut tool_args = Map::new();
            tool_args.insert("paper_id".to_string(), json!(id));
            tool_args.insert("response_format".to_string(), json!("concise"));

            let (payload, meta_value) = call_tool_raw("arxiv", "get", tool_args).await?;
            let pdf_url_only = payload.get("pdf_url").cloned().unwrap_or(Value::Null);
            return output_tool_result(cli, "arxiv", "pdf", &pdf_url_only, meta_value.as_ref());
        }
    };

    call_tool(cli, "arxiv", tool_name, args).await
}

/// Handle github commands
pub async fn handle_github(cli: &Cli, tool: GithubTools) -> Result<()> {
    fn split_owner_repo(repo: &str) -> Result<(String, String)> {
        let (owner, name) = repo.split_once('/').ok_or_else(|| {
            crate::commands::CommandError::InvalidInput(
                "Invalid repo. Expected 'owner/repo' (e.g., rust-lang/rust).".to_string(),
            )
        })?;
        Ok((owner.to_string(), name.to_string()))
    }

    let (tool_name, args) = match tool {
        GithubTools::SearchRepos { query, limit } => {
            let mut args = Map::new();
            args.insert("query".to_string(), json!(query));
            args.insert("per_page".to_string(), json!(limit));
            args.insert("page".to_string(), json!(1));
            ("search_repositories", args)
        }
        GithubTools::SearchCode { query, repo, limit } => {
            let mut args = Map::new();
            let query = if let Some(r) = repo {
                format!("{} repo:{}", query, r)
            } else {
                query
            };
            args.insert("query".to_string(), json!(query));
            args.insert("per_page".to_string(), json!(limit));
            args.insert("page".to_string(), json!(1));
            ("code_search", args)
        }
        GithubTools::Issues { repo, state, limit } => {
            let (owner, name) = split_owner_repo(&repo)?;
            let mut args = Map::new();
            args.insert("owner".to_string(), json!(owner));
            args.insert("repo".to_string(), json!(name));
            args.insert("state".to_string(), json!(state));
            args.insert("per_page".to_string(), json!(limit));
            args.insert("page".to_string(), json!(1));
            ("list_issues", args)
        }
        GithubTools::Pulls { repo, state, limit } => {
            let (owner, name) = split_owner_repo(&repo)?;
            let mut args = Map::new();
            args.insert("owner".to_string(), json!(owner));
            args.insert("repo".to_string(), json!(name));
            args.insert("state".to_string(), json!(state));
            args.insert("per_page".to_string(), json!(limit));
            args.insert("page".to_string(), json!(1));
            ("list_pull_requests", args)
        }
        GithubTools::Repo { repo } => {
            let (owner, name) = split_owner_repo(&repo)?;
            let mut args = Map::new();
            args.insert("owner".to_string(), json!(owner));
            args.insert("repo".to_string(), json!(name));
            ("get_repository", args)
        }
    };

    call_tool(cli, "github", tool_name, args).await
}

/// Handle reddit commands
pub async fn handle_reddit(cli: &Cli, tool: RedditTools) -> Result<()> {
    let (tool_name, args) = match tool {
        RedditTools::Search {
            query,
            subreddit,
            sort,
            time,
            limit,
        } => {
            let mut args = Map::new();
            args.insert("query".to_string(), json!(query));
            if let Some(sub) = subreddit {
                args.insert("subreddit".to_string(), json!(sub));
            }
            if sort != "relevance" {
                args.insert("sort".to_string(), json!(sort));
            }
            if time != "all" {
                args.insert("time".to_string(), json!(time));
            }
            args.insert("limit".to_string(), json!(limit));
            ("search", args)
        }
        RedditTools::Hot { subreddit, limit } => {
            let mut args = Map::new();
            args.insert("subreddit".to_string(), json!(subreddit));
            args.insert("limit".to_string(), json!(limit));
            args.insert("sort".to_string(), json!("hot"));
            ("list", args)
        }
        RedditTools::New { subreddit, limit } => {
            let mut args = Map::new();
            args.insert("subreddit".to_string(), json!(subreddit));
            args.insert("limit".to_string(), json!(limit));
            args.insert("sort".to_string(), json!("new"));
            ("list", args)
        }
        RedditTools::Top {
            subreddit,
            time,
            limit,
        } => {
            let mut args = Map::new();
            args.insert("subreddit".to_string(), json!(subreddit));
            args.insert("limit".to_string(), json!(limit));
            args.insert("sort".to_string(), json!("top"));
            args.insert("time".to_string(), json!(time));
            ("list", args)
        }
        RedditTools::Post { id } => {
            let mut args = Map::new();
            let post_url = if id.starts_with("http://") || id.starts_with("https://") {
                id
            } else {
                format!("https://www.reddit.com/comments/{}", id)
            };
            args.insert("post_url".to_string(), json!(post_url));
            ("get", args)
        }
    };

    call_tool(cli, "reddit", tool_name, args).await
}

/// Handle web commands
pub async fn handle_web(cli: &Cli, tool: WebTools) -> Result<()> {
    let (tool_name, args) = match tool {
        WebTools::Scrape { url, format } => {
            let mut args = Map::new();
            args.insert("url".to_string(), json!(url));
            let _ = format;
            ("scrape_url", args)
        }
        WebTools::Extract { url, images, links } => {
            let _ = (images, links);
            let mut args = Map::new();
            args.insert("url".to_string(), json!(url));
            ("extract", args)
        }
        WebTools::Metadata { url } => {
            let mut args = Map::new();
            args.insert("url".to_string(), json!(url));
            ("metadata", args)
        }
    };

    match tool_name {
        "scrape_url" => call_tool(cli, "web", "scrape_url", args).await,
        "extract" => {
            let (payload, meta_value) = call_tool_raw("web", "scrape_url", args).await?;
            let extracted = payload.get("content").cloned().unwrap_or(Value::Null);
            output_tool_result(cli, "web", "extract", &extracted, meta_value.as_ref())
        }
        "metadata" => {
            let (payload, meta_value) = call_tool_raw("web", "scrape_url", args).await?;
            let extracted = payload.get("metadata").cloned().unwrap_or(Value::Null);
            output_tool_result(cli, "web", "metadata", &extracted, meta_value.as_ref())
        }
        _ => unreachable!("tool_name is constructed above"),
    }
}

/// Handle wikipedia commands
pub async fn handle_wikipedia(cli: &Cli, tool: WikipediaTools) -> Result<()> {
    let (tool_name, args) = match tool {
        WikipediaTools::Search { query, limit } => {
            let mut args = Map::new();
            args.insert("query".to_string(), json!(query));
            args.insert("limit".to_string(), json!(limit));
            ("search", args)
        }
        WikipediaTools::Article { title } => {
            let mut args = Map::new();
            args.insert("title".to_string(), json!(title));
            args.insert("response_format".to_string(), json!("detailed"));
            ("get_article", args)
        }
        WikipediaTools::Summary { title } => {
            let mut args = Map::new();
            args.insert("title".to_string(), json!(title));
            args.insert("response_format".to_string(), json!("concise"));
            ("get_article", args)
        }
    };

    call_tool(cli, "wikipedia", tool_name, args).await
}

/// Handle pubmed commands
pub async fn handle_pubmed(cli: &Cli, tool: PubmedTools) -> Result<()> {
    let (tool_name, args) = match tool {
        PubmedTools::Search { query, limit } => {
            let mut args = Map::new();
            args.insert("query".to_string(), json!(query));
            args.insert("limit".to_string(), json!(limit));
            ("search", args)
        }
        PubmedTools::Article { pmid } => {
            let mut args = Map::new();
            args.insert("pmid".to_string(), json!(pmid));
            args.insert("response_format".to_string(), json!("detailed"));
            ("get", args)
        }
    };

    call_tool(cli, "pubmed", tool_name, args).await
}

/// Handle semantic scholar commands
pub async fn handle_semantic_scholar(cli: &Cli, tool: SemanticScholarTools) -> Result<()> {
    let (tool_name, args) = match tool {
        SemanticScholarTools::Search { query, limit } => {
            let mut args = Map::new();
            args.insert("query".to_string(), json!(query));
            args.insert("page_size".to_string(), json!(limit));
            args.insert("page".to_string(), json!(1));
            ("search_papers", args)
        }
        SemanticScholarTools::Paper { id } => {
            let mut args = Map::new();
            args.insert("paper_id".to_string(), json!(id));
            ("get_paper_details", args)
        }
        SemanticScholarTools::Citations { id, limit } => {
            let mut args = Map::new();
            args.insert("paper_id".to_string(), json!(id));
            args.insert("limit".to_string(), json!(limit));
            ("get_citations", args)
        }
        SemanticScholarTools::References { id, limit } => {
            let mut args = Map::new();
            args.insert("paper_id".to_string(), json!(id));
            args.insert("limit".to_string(), json!(limit));
            ("get_references", args)
        }
    };

    call_tool(cli, "semantic-scholar", tool_name, args).await
}

/// Handle slack commands
pub async fn handle_slack(cli: &Cli, tool: SlackTools) -> Result<()> {
    let (tool_name, args) = match tool {
        SlackTools::Channels { limit } => {
            let mut args = Map::new();
            args.insert("limit".to_string(), json!(limit));
            ("list_channels", args)
        }
        SlackTools::Messages { channel, limit } => {
            let mut args = Map::new();
            args.insert("channel".to_string(), json!(channel));
            args.insert("limit".to_string(), json!(limit));
            ("list_messages", args)
        }
        SlackTools::Search { query, limit } => {
            let mut args = Map::new();
            args.insert("query".to_string(), json!(query));
            args.insert("count".to_string(), json!(limit));
            ("search_messages", args)
        }
        SlackTools::Users { limit } => {
            let mut args = Map::new();
            args.insert("limit".to_string(), json!(limit));
            ("list_users", args)
        }
    };

    call_tool(cli, "slack", tool_name, args).await
}

/// Handle X (Twitter) commands
pub async fn handle_x(cli: &Cli, tool: XTools) -> Result<()> {
    let (tool_name, args) = match tool {
        XTools::Profile { username } => {
            let mut args = Map::new();
            args.insert("username".to_string(), json!(username));
            ("get_profile", args)
        }
        XTools::SearchTweets { query, limit } => {
            let mut args = Map::new();
            args.insert("query".to_string(), json!(query));
            if let Some(l) = limit {
                args.insert("limit".to_string(), json!(l));
            }
            ("search_tweets", args)
        }
        XTools::Followers {
            username,
            limit,
            cursor,
        } => {
            let mut args = Map::new();
            args.insert("username".to_string(), json!(username));
            args.insert("limit".to_string(), json!(limit));
            if let Some(c) = cursor {
                args.insert("cursor".to_string(), json!(c));
            }
            ("get_followers", args)
        }
        XTools::Tweet { tweet_id } => {
            let mut args = Map::new();
            args.insert("tweet_id".to_string(), json!(tweet_id));
            ("get_tweet", args)
        }
        XTools::Timeline {
            count,
            exclude_replies,
        } => {
            let mut args = Map::new();
            args.insert("count".to_string(), json!(count));
            if let Some(er) = exclude_replies {
                args.insert("exclude_replies".to_string(), json!(er));
            }
            ("get_home_timeline", args)
        }
        XTools::TweetsAndReplies {
            username,
            limit,
            cursor,
        } => {
            let mut args = Map::new();
            args.insert("username".to_string(), json!(username));
            args.insert("limit".to_string(), json!(limit));
            if let Some(c) = cursor {
                args.insert("cursor".to_string(), json!(c));
            }
            ("fetch_tweets_and_replies", args)
        }
        XTools::SearchProfiles {
            query,
            limit,
            cursor,
        } => {
            let mut args = Map::new();
            args.insert("query".to_string(), json!(query));
            args.insert("limit".to_string(), json!(limit));
            if let Some(c) = cursor {
                args.insert("cursor".to_string(), json!(c));
            }
            ("search_profiles", args)
        }
        XTools::DmConversations { user_id, cursor } => {
            let mut args = Map::new();
            args.insert("user_id".to_string(), json!(user_id));
            if let Some(c) = cursor {
                args.insert("cursor".to_string(), json!(c));
            }
            ("get_direct_message_conversations", args)
        }
        XTools::SendDm {
            conversation_id,
            text,
        } => {
            let mut args = Map::new();
            args.insert("conversation_id".to_string(), json!(conversation_id));
            args.insert("text".to_string(), json!(text));
            ("send_direct_message", args)
        }
    };

    call_tool(cli, "x", tool_name, args).await
}

/// Handle Discord commands
pub async fn handle_discord(cli: &Cli, tool: DiscordTools) -> Result<()> {
    let (tool_name, args) = match tool {
        DiscordTools::Servers => ("list_servers", Map::new()),
        DiscordTools::Server { guild_id } => {
            let mut args = Map::new();
            args.insert("guild_id".to_string(), json!(guild_id));
            ("get_server_info", args)
        }
        DiscordTools::Channels { guild_id } => {
            let mut args = Map::new();
            args.insert("guild_id".to_string(), json!(guild_id));
            ("list_channels", args)
        }
        DiscordTools::Messages { channel_id, limit } => {
            let mut args = Map::new();
            args.insert("channel_id".to_string(), json!(channel_id));
            if let Some(l) = limit {
                args.insert("limit".to_string(), json!(l));
            }
            ("read_messages", args)
        }
        DiscordTools::Send {
            channel_id,
            content,
        } => {
            let mut args = Map::new();
            args.insert("channel_id".to_string(), json!(channel_id));
            args.insert("content".to_string(), json!(content));
            ("send_message", args)
        }
        DiscordTools::Search {
            channel_id,
            query,
            limit,
        } => {
            let mut args = Map::new();
            args.insert("channel_id".to_string(), json!(channel_id));
            args.insert("query".to_string(), json!(query));
            if let Some(l) = limit {
                args.insert("limit".to_string(), json!(l));
            }
            ("search_messages", args)
        }
    };

    call_tool(cli, "discord", tool_name, args).await
}

/// Handle RSS commands
pub async fn handle_rss(cli: &Cli, tool: RssTools) -> Result<()> {
    let (tool_name, args) = match tool {
        RssTools::Feed { url, limit } => {
            let mut args = Map::new();
            args.insert("url".to_string(), json!(url));
            if let Some(l) = limit {
                args.insert("limit".to_string(), json!(l));
            }
            ("get_feed", args)
        }
        RssTools::Entries { url, limit } => {
            let mut args = Map::new();
            args.insert("url".to_string(), json!(url));
            if let Some(l) = limit {
                args.insert("limit".to_string(), json!(l));
            }
            ("list_entries", args)
        }
        RssTools::Search { url, query, limit } => {
            let mut args = Map::new();
            args.insert("url".to_string(), json!(url));
            args.insert("query".to_string(), json!(query));
            if let Some(l) = limit {
                args.insert("limit".to_string(), json!(l));
            }
            ("search_feed", args)
        }
        RssTools::Discover { url } => {
            let mut args = Map::new();
            args.insert("url".to_string(), json!(url));
            ("discover_feeds", args)
        }
    };

    call_tool(cli, "rss", tool_name, args).await
}

/// Handle bioRxiv commands
pub async fn handle_biorxiv(cli: &Cli, tool: BiorxivTools) -> Result<()> {
    let (tool_name, args) = match tool {
        BiorxivTools::Recent { server, count } => {
            let mut args = Map::new();
            args.insert("server".to_string(), json!(server));
            if let Some(c) = count {
                args.insert("count".to_string(), json!(c));
            }
            ("get_recent_preprints", args)
        }
        BiorxivTools::DateRange {
            server,
            start_date,
            end_date,
        } => {
            let mut args = Map::new();
            args.insert("server".to_string(), json!(server));
            args.insert("start_date".to_string(), json!(start_date));
            args.insert("end_date".to_string(), json!(end_date));
            ("get_preprints_by_date", args)
        }
        BiorxivTools::Paper { server, doi } => {
            let mut args = Map::new();
            args.insert("server".to_string(), json!(server));
            args.insert("doi".to_string(), json!(doi));
            ("get_preprint_by_doi", args)
        }
    };

    call_tool(cli, "biorxiv", tool_name, args).await
}

/// Handle Sci-Hub commands
pub async fn handle_scihub(cli: &Cli, tool: ScihubTools) -> Result<()> {
    let (tool_name, args) = match tool {
        ScihubTools::Paper { doi } => {
            let mut args = Map::new();
            args.insert("doi".to_string(), json!(doi));
            ("get", args)
        }
    };

    call_tool(cli, "scihub", tool_name, args).await
}

/// Handle macOS commands
pub async fn handle_macos(cli: &Cli, tool: MacosTools) -> Result<()> {
    let (tool_name, args) = match tool {
        MacosTools::Script {
            language,
            script,
            params,
            max_output_chars,
        } => {
            let mut args = Map::new();
            args.insert("language".to_string(), json!(language));
            args.insert("script".to_string(), json!(script));
            if let Some(ref p) = params {
                if let Ok(parsed) = serde_json::from_str::<Value>(p) {
                    args.insert("params".to_string(), parsed);
                }
            }
            if let Some(max) = max_output_chars {
                args.insert("max_output_chars".to_string(), json!(max));
            }
            ("run_script", args)
        }
        MacosTools::Notify {
            title,
            message,
            subtitle,
        } => {
            let mut args = Map::new();
            args.insert("message".to_string(), json!(message));
            if let Some(t) = title {
                args.insert("title".to_string(), json!(t));
            }
            if let Some(s) = subtitle {
                args.insert("subtitle".to_string(), json!(s));
            }
            ("show_notification", args)
        }
        MacosTools::Reveal { path } => {
            let mut args = Map::new();
            args.insert("path".to_string(), json!(path));
            ("reveal_in_finder", args)
        }
        MacosTools::GetClipboard => ("get_clipboard", Map::new()),
        MacosTools::SetClipboard { text } => {
            let mut args = Map::new();
            args.insert("text".to_string(), json!(text));
            ("set_clipboard", args)
        }
        MacosTools::Shortcut { name, input } => {
            let mut args = Map::new();
            args.insert("name".to_string(), json!(name));
            if let Some(ref i) = input {
                if let Ok(parsed) = serde_json::from_str::<Value>(i) {
                    args.insert("input".to_string(), parsed);
                }
            }
            ("run_shortcut", args)
        }
    };

    call_tool(cli, "macos", tool_name, args).await
}

/// Handle Spotlight commands
pub async fn handle_spotlight(cli: &Cli, tool: SpotlightTools) -> Result<()> {
    let (tool_name, args) = match tool {
        SpotlightTools::SearchContent {
            query,
            directory,
            kind,
            limit,
        } => {
            let mut args = Map::new();
            args.insert("mode".to_string(), json!("content"));
            args.insert("query".to_string(), json!(query));
            if let Some(d) = directory {
                args.insert("directory".to_string(), json!(d));
            }
            if let Some(k) = kind {
                args.insert("kind".to_string(), json!(k));
            }
            args.insert("limit".to_string(), json!(limit));
            ("search", args)
        }
        SpotlightTools::SearchByName {
            name,
            directory,
            limit,
        } => {
            let mut args = Map::new();
            args.insert("mode".to_string(), json!("name"));
            args.insert("query".to_string(), json!(name));
            if let Some(d) = directory {
                args.insert("directory".to_string(), json!(d));
            }
            args.insert("limit".to_string(), json!(limit));
            ("search", args)
        }
        SpotlightTools::SearchByKind {
            kind,
            directory,
            limit,
        } => {
            let mut args = Map::new();
            args.insert("mode".to_string(), json!("kind"));
            args.insert("kind".to_string(), json!(kind));
            if let Some(d) = directory {
                args.insert("directory".to_string(), json!(d));
            }
            args.insert("limit".to_string(), json!(limit));
            ("search", args)
        }
        SpotlightTools::SearchRecent {
            days,
            kind,
            directory,
            limit,
        } => {
            let mut args = Map::new();
            args.insert("mode".to_string(), json!("recent"));
            args.insert("days".to_string(), json!(days));
            if let Some(k) = kind {
                args.insert("kind".to_string(), json!(k));
            }
            if let Some(d) = directory {
                args.insert("directory".to_string(), json!(d));
            }
            args.insert("limit".to_string(), json!(limit));
            ("search", args)
        }
        SpotlightTools::Metadata { path } => {
            let mut args = Map::new();
            args.insert("path".to_string(), json!(path));
            ("get_metadata", args)
        }
        SpotlightTools::RawQuery {
            query,
            directory,
            limit,
        } => {
            let mut args = Map::new();
            args.insert("mode".to_string(), json!("raw"));
            args.insert("query".to_string(), json!(query));
            if let Some(d) = directory {
                args.insert("directory".to_string(), json!(d));
            }
            args.insert("limit".to_string(), json!(limit));
            ("search", args)
        }
    };

    call_tool(cli, "spotlight", tool_name, args).await
}
