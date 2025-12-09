//! X Connector Example
//! This example demonstrates how to use the X (Twitter) connector with authentication and basic operations.

use agent_twitter_client::timeline::v1::QueryTweetsResponse;
use arivu_core::auth::AuthDetails;
use arivu_core::connectors::x::XConnector;
use arivu_core::error::ConnectorError;
use arivu_core::ProviderRegistry;
use async_mcp::types::{CallToolRequest, ListRequest, ReadResourceRequest, ToolResponseContent};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::Mutex;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Initialize Provider Registry
    let provider_registry = Arc::new(Mutex::new(ProviderRegistry::new()));

    // 2. Create and Configure X Connector
    let mut auth_details = AuthDetails::new();
    auth_details.insert("browser".to_string(), "chrome".to_string());
    let x_connector = XConnector::new(auth_details).await?;

    // 3. Register Connector
    {
        let mut registry = provider_registry.lock().await;
        registry.register_provider(Box::new(x_connector));
    }

    // 4. Get Connector Instance
    let connector = {
        let mut registry = provider_registry.lock().await;
        registry
            .get_provider("x")
            .expect("X Connector not registered")
            .clone()
    };

    // 5. Test Authentication
    let test_auth_response = connector.test_auth().await?;
    println!("Test Auth Response:\n{:#?}\n", test_auth_response);

    // 6. List Available Resources
    let list_resources_request = ListRequest {
        cursor: None,
        meta: None,
    };
    let list_resources_response = connector.list_resources(list_resources_request).await?;
    println!("List Resources Response:\n{:#?}\n", list_resources_response);

    // 7. List Available Prompts
    let list_prompts_request = ListRequest {
        cursor: None,
        meta: None,
    };
    let list_prompts_response = connector.list_prompts(list_prompts_request).await?;
    println!("List Prompts Response:\n{:#?}\n", list_prompts_response);

    // 8. List Available Tools
    let list_tools_request = ListRequest {
        cursor: None,
        meta: None,
    };
    let list_tools_response = connector.list_tools(list_tools_request).await?;
    println!("List Tools Response:\n{:#?}\n", list_tools_response);

    // 9. Get a Specific Prompt
    let prompt = connector.get_prompt("summarize_user_tweets").await?;
    println!("Prompt:\n{:#?}\n", prompt);

    // 10. Search Tweets Example
    let search_tweets_request = CallToolRequest {
        name: "search_tweets".to_string(),
        arguments: Some(
            json!({
                "query": "carelesswhisper.app",
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
    let search_tweets_response = connector.call_tool(search_tweets_request).await?;

    // Handle Search Response
    match search_tweets_response.content.first() {
        Some(ToolResponseContent::Text { text }) => {
            let tweets: QueryTweetsResponse =
                serde_json::from_str(text).map_err(|e| ConnectorError::SerdeJson(e))?;
            println!("Search Tweets Response:\n{:#?}\n", tweets);
        }
        _ => {
            println!("Unexpected response type");
        }
    };

    // 11. Get a Specific Tweet
    let get_tweet_request = ReadResourceRequest {
        uri: "twitter://tweet/1234".parse().unwrap(),
    };
    let get_tweet_response = connector.read_resource(get_tweet_request).await?;
    println!("Get Tweet Response:\n{:#?}\n", get_tweet_response);

    Ok(())
}
