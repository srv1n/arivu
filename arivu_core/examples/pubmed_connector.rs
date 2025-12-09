use arivu_core::auth::AuthDetails;
use arivu_core::connectors::pubmed::PubMedConnector;
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

    // Search PubMed for "Age-related changes in pharmacokinetics"
    println!("\nSearching PubMed for 'Age-related changes in pharmacokinetics':");
    let search_args: HashMap<String, Value> = json!({
        "query": "Age-related changes in pharmacokinetics",
        "limit": 10
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

                println!("Search Results Summary:");
                println!("Total Results: {}", search_result["total_results"]);
                println!("Page: {}", search_result["page"]);
                if let Some(total_pages) = search_result["total_pages"].as_u64() {
                    println!("Total Pages: {}", total_pages);
                }

                // Extract the first article's details
                if let Some(first_article) = search_result["articles"]
                    .as_array()
                    .and_then(|arr| arr.first())
                {
                    println!("\nFirst Article Details:");
                    println!("Title: {}", first_article["title"]);
                    println!("Authors: {}", first_article["authors"]);
                    println!("Journal: {}", first_article["citation"]);
                    println!("PMID: {}", first_article["pmid"]);
                    println!("URL: {}", first_article["url"]);

                    // Extract the PMID from the first article
                    let pmid = first_article["pmid"].as_str().unwrap_or("14678335"); // Default to a known PMID if not found

                    // Get Abstract using the PMID from search results
                    println!("\nGetting detailed abstract for PMID '{}':", pmid);
                    let abstract_args: HashMap<String, Value> = json!({
                        "pmid": pmid
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
                                let abstract_data: serde_json::Value = serde_json::from_str(text)?;

                                // Print detailed abstract information
                                println!("\n=== DETAILED ARTICLE INFORMATION ===");
                                println!("Title: {}", abstract_data["title"]);
                                println!("Authors: {}", abstract_data["authors"]);
                                println!("Journal: {}", abstract_data["journal"]);
                                println!("Publication Date: {}", abstract_data["publication_date"]);

                                if let Some(publication_type) =
                                    abstract_data["publication_type"].as_str()
                                {
                                    println!("Publication Type: {}", publication_type);
                                }

                                if let Some(doi) = abstract_data["doi"].as_str() {
                                    println!("DOI: {}", doi);
                                } else {
                                    println!("DOI: Not available");
                                }

                                if let Some(citation_count) =
                                    abstract_data["citation_count"].as_u64()
                                {
                                    println!("Citation Count: {}", citation_count);
                                }

                                println!("\nAbstract:");
                                println!("{}", abstract_data["abstract_text"]);

                                // Print affiliations if available
                                if let Some(affiliations) = abstract_data["affiliations"].as_array()
                                {
                                    if !affiliations.is_empty() {
                                        println!("\nAffiliations:");
                                        for (i, affiliation) in affiliations.iter().enumerate() {
                                            println!("  {}. {}", i + 1, affiliation);
                                        }
                                    }
                                }

                                // Print keywords if available
                                if let Some(keywords) = abstract_data["keywords"].as_array() {
                                    if !keywords.is_empty() {
                                        println!("\nKeywords:");
                                        for keyword in keywords {
                                            println!("  - {}", keyword);
                                        }
                                    }
                                }

                                // Print similar articles if available
                                if let Some(similar_articles) =
                                    abstract_data["similar_articles"].as_array()
                                {
                                    if !similar_articles.is_empty() {
                                        println!("\nSimilar Articles:");
                                        for (i, article) in similar_articles.iter().enumerate() {
                                            println!("  {}. {}", i + 1, article["title"]);
                                            println!("     Authors: {}", article["authors"]);
                                            println!("     Journal: {}", article["journal"]);
                                            println!("     PMID: {}", article["pmid"]);
                                            if let Some(pub_type) =
                                                article["publication_type"].as_str()
                                            {
                                                println!("     Type: {}", pub_type);
                                            }
                                            println!();
                                        }
                                    }
                                }

                                // Format citation
                                println!("\nCitation:");
                                println!(
                                    "{}, {}, {}, PMID: {}",
                                    abstract_data["authors"],
                                    abstract_data["title"],
                                    abstract_data["journal"],
                                    abstract_data["pmid"]
                                );

                                if let Some(doi) = abstract_data["doi"].as_str() {
                                    println!("DOI: {}", doi);
                                }

                                // Print URL to access the article
                                println!("\nAccess Article:");
                                println!(
                                    "https://pubmed.ncbi.nlm.nih.gov/{}/",
                                    abstract_data["pmid"]
                                );

                                if let Some(doi) = abstract_data["doi"].as_str() {
                                    println!("https://doi.org/{}", doi);
                                }
                            }
                            _ => println!("Unexpected response type"),
                        }
                    }
                } else {
                    println!("No articles found in the search results.");
                }
            }
            _ => println!("Unexpected response type"),
        }
    }

    // Search for a specific article by PMID (using the example from the HTML)
    println!("\n\nRetrieving specific article by PMID '14678335' (Age-related changes in pharmacokinetics and pharmacodynamics):");
    let specific_abstract_args: HashMap<String, Value> = json!({
        "pmid": "14678335"
    })
    .as_object()
    .unwrap()
    .iter()
    .map(|(k, v)| (k.clone(), v.clone()))
    .collect();

    let specific_abstract_request = CallToolRequest {
        name: "get_abstract".to_string(),
        arguments: Some(specific_abstract_args),
        meta: None,
    };

    let specific_abstract_response = connector.call_tool(specific_abstract_request).await?;

    if let Some(content) = specific_abstract_response.content.first() {
        match content {
            async_mcp::types::ToolResponseContent::Text { text } => {
                let abstract_data: serde_json::Value = serde_json::from_str(text)?;

                // Print detailed abstract information for the specific article
                println!("\n=== DETAILED ARTICLE INFORMATION ===");
                println!("Title: {}", abstract_data["title"]);
                println!("Authors: {}", abstract_data["authors"]);
                println!("Journal: {}", abstract_data["journal"]);
                println!("Publication Date: {}", abstract_data["publication_date"]);

                if let Some(publication_type) = abstract_data["publication_type"].as_str() {
                    println!("Publication Type: {}", publication_type);
                }

                if let Some(doi) = abstract_data["doi"].as_str() {
                    println!("DOI: {}", doi);
                } else {
                    println!("DOI: Not available");
                }

                if let Some(citation_count) = abstract_data["citation_count"].as_u64() {
                    println!("Citation Count: {}", citation_count);
                }

                println!("\nAbstract:");
                println!("{}", abstract_data["abstract_text"]);

                // Print affiliations if available
                if let Some(affiliations) = abstract_data["affiliations"].as_array() {
                    if !affiliations.is_empty() {
                        println!("\nAffiliations:");
                        for (i, affiliation) in affiliations.iter().enumerate() {
                            println!("  {}. {}", i + 1, affiliation);
                        }
                    }
                }

                // Print keywords if available
                if let Some(keywords) = abstract_data["keywords"].as_array() {
                    if !keywords.is_empty() {
                        println!("\nKeywords:");
                        for keyword in keywords {
                            println!("  - {}", keyword);
                        }
                    }
                }

                // Print similar articles if available
                if let Some(similar_articles) = abstract_data["similar_articles"].as_array() {
                    if !similar_articles.is_empty() {
                        println!("\nSimilar Articles:");
                        for (i, article) in similar_articles.iter().enumerate() {
                            println!("  {}. {}", i + 1, article["title"]);
                            println!("     Authors: {}", article["authors"]);
                            println!("     Journal: {}", article["journal"]);
                            println!("     PMID: {}", article["pmid"]);
                            if let Some(pub_type) = article["publication_type"].as_str() {
                                println!("     Type: {}", pub_type);
                            }
                            println!();
                        }
                    }
                }

                // Format citation
                println!("\nCitation:");
                println!(
                    "{}, {}, {}, PMID: {}",
                    abstract_data["authors"],
                    abstract_data["title"],
                    abstract_data["journal"],
                    abstract_data["pmid"]
                );

                if let Some(doi) = abstract_data["doi"].as_str() {
                    println!("DOI: {}", doi);
                }

                // Print URL to access the article
                println!("\nAccess Article:");
                println!("https://pubmed.ncbi.nlm.nih.gov/{}/", abstract_data["pmid"]);

                if let Some(doi) = abstract_data["doi"].as_str() {
                    println!("https://doi.org/{}", doi);
                }
            }
            _ => println!("Unexpected response type"),
        }
    }

    Ok(())
}
