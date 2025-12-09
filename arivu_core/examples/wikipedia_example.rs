use arivu_core::auth::AuthDetails;
use arivu_core::connectors::wikipedia::WikipediaConnector;
use arivu_core::Connector;
use arivu_core::ProviderRegistry;
use async_mcp::types::{CallToolRequest, ListRequest, ReadResourceRequest};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::Mutex;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Initialize Provider Registry
    let provider_registry = Arc::new(Mutex::new(ProviderRegistry::new()));

    // 2. Create and Configure Wikipedia Connector
    let mut auth_details = AuthDetails::new();
    auth_details.insert("language".to_string(), "en".to_string());
    auth_details.insert("search_limit".to_string(), "10".to_string());
    let wikipedia_connector = WikipediaConnector::new(auth_details).await?;

    // 3. Register Connector
    {
        let mut registry = provider_registry.lock().await;
        registry.register_provider(Box::new(wikipedia_connector));
    }

    // 4. Get Connector Instance
    let connector = {
        let registry = provider_registry.lock().await;
        registry
            .get_provider("wikipedia")
            .expect("Wikipedia Connector not registered")
            .clone()
    };

    println!("Connector: ");

    // 5. Test Authentication
    connector.test_auth().await?;
    println!("Authentication test successful");

    // 6. List Available Resources
    let list_resources_request = ListRequest {
        cursor: None,
        meta: None,
    };
    let list_resources_response = connector.list_resources(list_resources_request).await?;
    println!("Available Resources:\n{:#?}\n", list_resources_response);

    // 7. List Available Tools
    let list_tools_request = ListRequest {
        cursor: None,
        meta: None,
    };
    let list_tools_response = connector.list_tools(list_tools_request).await?;
    println!("Available Tools:\n{:#?}\n", list_tools_response);

    // 8. Search for Articles
    let search_request = CallToolRequest {
        name: "search".to_string(),
        arguments: Some(
            json!({
                "query": "Rust programming language",
                "limit": 5
            })
            .as_object()
            .unwrap()
            .clone()
            .into_iter()
            .collect(),
        ),
        meta: None,
    };
    let search_response = connector.call_tool(search_request).await?;
    println!("Search Results:\n{:#?}\n", search_response);

    // 9. Geo Search for Articles
    let geosearch_request = CallToolRequest {
        name: "geosearch".to_string(),
        arguments: Some(
            json!({
                "latitude": 40.750556,
                "longitude": -73.993611,
                "radius": 1000
            })
            .as_object()
            .unwrap()
            .clone()
            .into_iter()
            .collect(),
        ),
        meta: None,
    };
    let geosearch_response = connector.call_tool(geosearch_request).await?;
    println!("Geo Search Results:\n{:#?}\n", geosearch_response);

    // 10. Get Article Content
    let get_article_request = CallToolRequest {
        name: "get_article".to_string(),
        arguments: Some(
            json!({
                "title": "Rust (programming language)"
            })
            .as_object()
            .unwrap()
            .clone()
            .into_iter()
            .collect(),
        ),
        meta: None,
    };
    let get_article_response = connector.call_tool(get_article_request).await?;
    println!("Article Content:\n{:#?}\n", get_article_response);

    // 11. Read Resource
    let read_resource_request = ReadResourceRequest {
        uri: "wikipedia://article/Rust (programming language)"
            .parse()
            .unwrap(),
    };
    let read_resource_response = connector.read_resource(read_resource_request).await?;
    println!("Resource Content:\n{:#?}\n", read_resource_response);

    Ok(())
}
