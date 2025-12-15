use crate::cli::Cli;
use crate::commands::{copy_to_clipboard, CommandError, Result};
use crate::output::{format_output, format_pretty, OutputData};
use arivu_core::{CallToolRequestParam, PaginatedRequestParam, ProviderRegistry};
use indicatif::{ProgressBar, ProgressStyle};
use owo_colors::OwoColorize;
use serde_json::{json, Value};

pub async fn run(cli: &Cli, connector_name: &str, id: &str) -> Result<()> {
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .expect("Invalid progress template"),
    );
    spinner.set_message(format!("Fetching {} from {}...", id, connector_name));
    spinner.enable_steady_tick(std::time::Duration::from_millis(100));

    let registry = create_registry().await?;
    let provider = registry
        .get_provider(connector_name)
        .ok_or_else(|| CommandError::ConnectorNotFound(connector_name.to_string()))?
        .clone();

    // List available tools to find the get/details tool
    let c = provider.lock().await;
    let tools_response = c
        .list_tools(Some(PaginatedRequestParam { cursor: None }))
        .await?;

    // Find appropriate get tool (varies by connector)
    let get_tool = tools_response
        .tools
        .iter()
        .find(|tool| {
            tool.name.contains("get")
                || tool.name.contains("details")
                || tool.name.contains("fetch")
                || tool.name.contains("article")
                || tool.name.contains("post")
        })
        .ok_or_else(|| CommandError::ToolNotFound("get".to_string(), connector_name.to_string()))?;

    // Prepare request based on connector type
    let arguments = match connector_name {
        "youtube" => json!({ "video_id": id }),
        "reddit" => json!({ "post_id": id }),
        "hackernews" => json!({ "item_id": id }),
        "wikipedia" => json!({ "title": id }),
        "arxiv" => json!({ "paper_id": id }),
        "pubmed" => json!({ "pmid": id }),
        _ => json!({ "id": id }),
    };

    let request = CallToolRequestParam {
        name: get_tool.name.clone(),
        arguments: Some(arguments.as_object().expect("JSON object").clone()),
    };

    let response = c.call_tool(request).await?;
    spinner.finish_and_clear();

    // Extract response data
    let data = if let Some(val) = &response.structured_content {
        val.clone()
    } else {
        json!({})
    };

    let output_data = OutputData::ResourceData {
        connector: connector_name.to_string(),
        id: id.to_string(),
        data: data.clone(),
    };

    match cli.output {
        crate::cli::OutputFormat::Pretty => {
            format_pretty_resource_data(connector_name, id, &data)?;
        }
        _ => {
            format_output(&output_data, &cli.output)?;
        }
    }

    // Copy to clipboard if requested
    if cli.copy {
        let text = serde_json::to_string_pretty(&data)?;
        copy_to_clipboard(&text)?;
    }

    Ok(())
}

fn format_pretty_resource_data(connector: &str, id: &str, data: &Value) -> Result<()> {
    println!(
        "{} {} {}",
        "Resource:".bold().cyan(),
        id.yellow(),
        format!("({})", connector).dimmed()
    );
    println!();

    match connector {
        "youtube" => format_youtube_data(data)?,
        "reddit" => format_reddit_data(data)?,
        "hackernews" => format_hackernews_data(data)?,
        "wikipedia" => format_wikipedia_data(data)?,
        "arxiv" => format_arxiv_data(data)?,
        "pubmed" => format_pubmed_data(data)?,
        _ => {
            // Generic smart formatting for other connectors
            println!("{}", format_pretty(data));
        }
    }

    Ok(())
}

fn format_youtube_data(data: &Value) -> Result<()> {
    // Title as main heading
    if let Some(title) = data.get("title") {
        println!("# {}", title.as_str().unwrap_or("").bold());
        println!();
    }

    // Description as first paragraph
    if let Some(description) = data.get("description") {
        let desc = description.as_str().unwrap_or("");
        if !desc.is_empty() {
            println!("{}", desc);
            println!();
        }
    }

    // Full transcript if available
    // if let Some(transcript) = data.get("transcript") {
    //     if let Some(transcript_str) = transcript.as_str() {
    //         if !transcript_str.is_empty() {
    //             // println!("## {}", "Full Transcript".bold());
    //             println!("{}", transcript_str);
    //             println!();
    //         }
    //     }
    // }

    // Chapters with full content
    if let Some(chapters) = data.get("chapters").and_then(|c| c.as_array()) {
        if !chapters.is_empty() {
            println!("## {}", "Chapters".bold());
            println!();

            for chapter in chapters {
                if let (Some(heading), Some(start_time)) =
                    (chapter.get("heading"), chapter.get("start_time"))
                {
                    let time = start_time.as_i64().unwrap_or(0);
                    let mins = time / 60;
                    let secs = time % 60;

                    println!(
                        "### {} ({}:{:02})",
                        heading.as_str().unwrap_or("").bold(),
                        mins,
                        secs
                    );

                    if let Some(content) = chapter.get("content") {
                        let content_str = content.as_str().unwrap_or("");
                        if !content_str.is_empty() {
                            println!("{}", content_str);
                            println!();
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

fn format_reddit_data(data: &Value) -> Result<()> {
    // TODO: Implement Reddit-specific formatting
    println!("{}", serde_json::to_string_pretty(data)?);
    Ok(())
}

fn format_hackernews_data(data: &Value) -> Result<()> {
    // TODO: Implement HackerNews-specific formatting
    println!("{}", serde_json::to_string_pretty(data)?);
    Ok(())
}

fn format_wikipedia_data(data: &Value) -> Result<()> {
    // TODO: Implement Wikipedia-specific formatting
    println!("{}", serde_json::to_string_pretty(data)?);
    Ok(())
}

fn format_arxiv_data(data: &Value) -> Result<()> {
    // TODO: Implement ArXiv-specific formatting
    println!("{}", serde_json::to_string_pretty(data)?);
    Ok(())
}

fn format_pubmed_data(data: &Value) -> Result<()> {
    // TODO: Implement PubMed-specific formatting
    println!("{}", serde_json::to_string_pretty(data)?);
    Ok(())
}

async fn create_registry() -> Result<ProviderRegistry> {
    // Reuse the registry creation logic from list.rs
    crate::commands::list::create_registry().await
}
