use arivu_core::auth::AuthDetails;
use arivu_core::connectors::arxiv::ArxivConnector;
use arivu_core::Connector;
use async_mcp::types::{CallToolRequest, ListRequest};
use serde_json::{json, Value};
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the arXiv connector
    let auth_details = AuthDetails::new(); // arXiv doesn't require authentication
    let arxiv_connector = ArxivConnector::new(auth_details).await?;

    println!("Initialized arXiv connector: {}", arxiv_connector.name());

    // List available tools
    let tools_response = arxiv_connector
        .list_tools(ListRequest {
            cursor: None,
            meta: None,
        })
        .await?;

    println!("Available tools:");
    for tool in tools_response.tools {
        println!(
            "  - {}: {}",
            tool.name,
            tool.description.unwrap_or_default()
        );
    }

    // Search for papers about "quantum computing"
    println!("\nSearching for papers about 'quantum computing'...");
    let mut args = HashMap::new();
    args.insert("query".to_string(), json!("quantum computing"));
    args.insert("max_results".to_string(), json!(5));

    let search_response = arxiv_connector
        .call_tool(CallToolRequest {
            name: "search_papers".to_string(),
            arguments: Some(args),
            meta: None,
        })
        .await?;

    // Parse the search results
    if let Some(content) = search_response.content.first() {
        if let async_mcp::types::ToolResponseContent::Text { text } = content {
            let papers: Vec<HashMap<String, Value>> = serde_json::from_str(text)?;

            println!("\nFound {} papers:", papers.len());
            for (i, paper) in papers.iter().enumerate() {
                println!("{}. {}", i + 1, paper["title"]);
                println!("   Authors: {}", serde_json::to_string(&paper["authors"])?);
                println!("   ID: {}", paper["id"]);
                println!();
            }

            // Get details for the first paper
            if !papers.is_empty() {
                let first_paper_id = papers[0]["id"].as_str().unwrap_or_default();
                println!("Getting details for paper ID: {}", first_paper_id);

                let mut detail_args = HashMap::new();
                detail_args.insert("paper_id".to_string(), json!(first_paper_id));

                let detail_response = arxiv_connector
                    .call_tool(CallToolRequest {
                        name: "get_paper_details".to_string(),
                        arguments: Some(detail_args),
                        meta: None,
                    })
                    .await?;

                if let Some(detail_content) = detail_response.content.first() {
                    if let async_mcp::types::ToolResponseContent::Text { text } = detail_content {
                        let paper_details: HashMap<String, Value> = serde_json::from_str(text)?;

                        println!("\nPaper Details:");
                        println!("Title: {}", paper_details["title"]);
                        println!(
                            "Authors: {}",
                            serde_json::to_string(&paper_details["authors"])?
                        );
                        println!("Published: {}", paper_details["published"]);
                        println!(
                            "Categories: {}",
                            serde_json::to_string(&paper_details["categories"])?
                        );
                        println!("Abstract URL: {}", paper_details["abstract_url"]);
                        println!("PDF URL: {}", paper_details["pdf_url"]);
                        println!("\nSummary:");
                        println!("{}", paper_details["summary"]);
                    }
                }
            }
        }
    }

    Ok(())
}
