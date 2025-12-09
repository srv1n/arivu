use arivu_core::auth::AuthDetails;
use arivu_core::connectors::pubmed::{PubMedAbstract, PubMedConnector};
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

    // Create PubMed Connector
    // let auth_details = AuthDetails::new(); // PubMed doesn't require auth for basic searches
    let pubmed_connector = PubMedConnector::new().await?;

    // Register Connector
    {
        let mut registry = provider_registry.lock().await;
        registry.register_provider(Box::new(pubmed_connector));
    }

    // Get Connector Instance
    let connector = {
        let registry = provider_registry.lock().await;
        registry
            .get_provider("pubmed")
            .expect("PubMed Connector not registered")
            .clone()
    };

    // Test Authentication
    println!("Testing authentication...");
    connector.test_auth().await?;
    println!("Authentication successful!");

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

    // Search PubMed with date range
    println!("\nSearching PubMed for 'Kidney digestion' ");
    let search_args: HashMap<String, Value> = json!({
        "query": "valerian root and sleep",
        "page": 1,
        "limit": 50,

    })
    .as_object()
    .unwrap()
    .iter()
    .map(|(k, v)| (k.clone(), v.clone()))
    .collect();

    let search_request = CallToolRequest {
        name: "search".to_string(),
        arguments: Some(search_args),
        meta: None,
    };

    let search_response = connector.call_tool(search_request).await?;

    if let Some(content) = search_response.content.first() {
        match content {
            async_mcp::types::ToolResponseContent::Text { text } => {
                let search_result: serde_json::Value = serde_json::from_str(text)?;
                println!(
                    "Search Results: {}",
                    serde_json::to_string_pretty(&search_result)?
                );

                // Extract the first PMID from the search results for the next example
                let first_pmid = search_result["articles"][0]["pmid"]
                    .as_str()
                    .unwrap_or("12345678");

                // Get Abstract using the PMID from search results
                println!("\nGetting abstract for PMID '{}':", first_pmid);
                let abstract_args: HashMap<String, Value> = json!({
                    "pmid": first_pmid
                })
                .as_object()
                .unwrap()
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();

                let abstract_request = CallToolRequest {
                    name: "get_abstract".to_string(),
                    arguments: Some(abstract_args),
                    meta: None,
                };

                let abstract_response = connector.call_tool(abstract_request).await?;

                if let Some(content) = abstract_response.content.first() {
                    match content {
                        async_mcp::types::ToolResponseContent::Text { text } => {
                            let abstract_data: PubMedAbstract = serde_json::from_str(text)?;
                            println!(
                                "Abstract Data: {}",
                                serde_json::to_string_pretty(&abstract_data)?
                            );
                        }
                        _ => println!("Unexpected response type"),
                    }
                }
            }
            _ => println!("Unexpected response type"),
        }
    }

    // Search PubMed with pagination (page 2)
    // println!("\nSearching PubMed for 'cancer therapy' (page 2):");
    // let search_page2_args: HashMap<String, Value> = json!({
    //     "query": "cancer therapy",
    //     "page": 2,
    //     "limit": 3
    // })
    // .as_object()
    // .unwrap()
    // .iter()
    // .map(|(k, v)| (k.clone(), v.clone()))
    // .collect();

    // let search_page2_request = CallToolRequest {
    //     name: "search".to_string(),
    //     arguments: Some(search_page2_args),
    //     meta: None,
    // };

    // let search_page2_response = connector.call_tool(search_page2_request).await?;

    // if let Some(content) = search_page2_response.content.first() {
    //     match content {
    //         async_mcp::types::ToolResponseContent::Text { text } => {
    //             let search_result: serde_json::Value = serde_json::from_str(text)?;
    //             println!("Page 2 Results: {}", serde_json::to_string_pretty(&search_result)?);
    //         },
    //         _ => println!("Unexpected response type"),
    //     }
    // }

    // List Available Prompts
    println!("\nListing available prompts:");
    let list_prompts_request = ListRequest {
        cursor: None,
        meta: None,
    };
    let list_prompts_response = connector.list_prompts(list_prompts_request).await?;
    for prompt in &list_prompts_response.prompts {
        println!(
            "- {}: {}",
            prompt.name,
            prompt
                .description
                .as_ref()
                .unwrap_or(&"No description".to_string())
        );
    }

    Ok(())
}
