use arivu_core::auth::AuthDetails;
use arivu_core::connectors::scihub::SciHubConnector;
use arivu_core::error::ConnectorError;
use arivu_core::ProviderRegistry;
use async_mcp::types::{CallToolRequest, ListRequest};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize Provider Registry
    let provider_registry = Arc::new(Mutex::new(ProviderRegistry::new()));

    // Create SciHub Connector
    let auth_details = AuthDetails::new(); // SciHub doesn't require auth for basic searches
    let scihub_connector = SciHubConnector::new(auth_details).await?;

    // Register Connector
    {
        let mut registry = provider_registry.lock().await;
        registry.register_provider(Box::new(scihub_connector));
    }

    // Get Connector Instance
    let connector = {
        let registry = provider_registry.lock().await;
        registry
            .get_provider("scihub")
            .expect("SciHub Connector not registered")
            .clone()
    };

    // Test Authentication
    println!("Testing connectivity...");
    connector.test_auth().await?;
    println!("Connectivity test successful!");

    // List Available Tools
    println!("\nListing available tools:");
    let list_tools_request = ListRequest {
        cursor: None,
        meta: None,
    };
    let list_tools_response = connector.list_tools(list_tools_request).await?;
    for tool in &list_tools_response.tools {
        println!(
            "- {}: {}",
            tool.name,
            tool.description
                .as_ref()
                .unwrap_or(&"No description".to_string())
        );
    }

    // Example DOIs to test
    let dois = vec![
        "10.1046/j.1365-2125.2003.02007.x", // Age-related changes in pharmacokinetics and pharmacodynamics
        "10.1038/s41586-020-2649-2",        // Some Nature paper
        "10.1016/j.cell.2020.04.011",       // Some Cell paper
    ];

    for doi in dois {
        println!("\nSearching SciHub for DOI: {}", doi);
        let search_args: HashMap<String, Value> = json!({
            "doi": doi
        })
        .as_object()
        .unwrap()
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();

        let search_request = CallToolRequest {
            name: "get_paper".to_string(),
            arguments: Some(search_args),
            meta: None,
        };

        let search_response = connector.call_tool(search_request).await?;

        if let Some(content) = search_response.content.first() {
            match content {
                async_mcp::types::ToolResponseContent::Text { text } => {
                    let result: serde_json::Value = serde_json::from_str(text)?;

                    println!("Result for DOI {}:", doi);
                    println!("Success: {}", result["success"]);
                    println!("Message: {}", result["message"]);

                    if let Some(pdf_url) = result["pdf_url"].as_str() {
                        println!("PDF URL: {}", pdf_url);
                    } else {
                        println!("PDF URL: Not available");
                    }

                    if let Some(title) = result["title"].as_str() {
                        println!("Title: {}", title);
                    }

                    if let Some(authors) = result["authors"].as_str() {
                        println!("Authors: {}", authors);
                    }

                    if let Some(journal) = result["journal"].as_str() {
                        println!("Journal: {}", journal);
                    }

                    if let Some(year) = result["year"].as_str() {
                        println!("Year: {}", year);
                    }
                }
                _ => println!("Unexpected response type"),
            }
        }
    }

    Ok(())
}
