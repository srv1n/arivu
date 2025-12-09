//! YouTube Connector Example
//! This example demonstrates how to use the YouTube connector with authentication and basic operations.

use arivu_core::auth::AuthDetails;
use arivu_core::connectors::youtube::{
    SearchVideosOutput, VideoSearchResult, YouTubeConnector, YouTubeContent,
};
use arivu_core::error::ConnectorError;
use arivu_core::ProviderRegistry;
use async_mcp::types::{CallToolRequest, ListRequest, ReadResourceRequest, ToolResponseContent};
use rusty_ytdl::search::Video;
use rusty_ytdl::VideoSearchOptions;
use serde_json::json;
use std::sync::Arc;
use tokio::sync::Mutex;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Initialize Provider Registry
    let provider_registry = Arc::new(Mutex::new(ProviderRegistry::new()));

    let youtube_connector = YouTubeConnector::new(None).await?;

    // 3. Register Connector
    {
        let mut registry = provider_registry.lock().await;
        registry.register_provider(Box::new(youtube_connector));
    }

    // 4. Get Connector Instance
    let connector = {
        let mut registry = provider_registry.lock().await;
        registry
            .get_provider("youtube")
            .expect("YouTube Connector not registered")
            .clone()
    };

    // 5. List Available Resources
    let list_resources_request = ListRequest {
        cursor: None,
        meta: None,
    };
    let list_resources_response = connector.list_resources(list_resources_request).await?;
    println!("List Resources Response:\n{:#?}\n", list_resources_response);

    // 6. List Available Tools
    let list_tools_request = ListRequest {
        cursor: None,
        meta: None,
    };
    let list_tools_response = connector.list_tools(list_tools_request).await?;
    println!("List Tools Response:\n{:#?}\n", list_tools_response);

    // 7. Get video Info Example
    let get_video_info_request = CallToolRequest {
        name: "get_video_details".to_string(),
        arguments: Some(
            json!({
                "video_id": "https://www.youtube.com/watch?v=CVpfPETFPhE"
            })
            .as_object()
            .unwrap()
            .clone()
            .into_iter()
            .collect(),
        ),
        meta: None,
    };
    let mut transcript = String::new();
    let get_video_info_response = connector.call_tool(get_video_info_request).await?;

    match get_video_info_response.content.first() {
        Some(ToolResponseContent::Text { text }) => {
            let video_info: YouTubeContent =
                serde_json::from_str(text).map_err(|e| ConnectorError::SerdeJson(e))?;
            for segment in video_info.clone().chapters {
                transcript.push_str(&segment.content);
            }
            println!("Get Video Info Response:\n{:#?}\n", video_info);
        }
        _ => {
            println!("Unexpected response type");
        }
    };

    // 8. Search Tweets Example
    let search_videos_request = CallToolRequest {
        name: "search_videos".to_string(),
        arguments: Some(
            json!({
                "query": "rust programming",
                "limit": 3,
                "search_type": "video",
                "sort": "views_desc",
                "upload_date": "this_year"
            })
            .as_object()
            .unwrap()
            .clone()
            .into_iter()
            .collect(),
        ),
        meta: None,
    };
    let search_videos_response = connector.call_tool(search_videos_request).await?;

    match search_videos_response.content.first() {
        Some(ToolResponseContent::Text { text }) => {
            let output: SearchVideosOutput =
                serde_json::from_str(text).map_err(|e| ConnectorError::SerdeJson(e))?;
            // println!("Search Videos Response:\n{:#?}\n", output);
        }
        _ => {
            println!("Unexpected response type");
        }
    }
    // println!("Transcript:\n{:#?}\n", transcript);

    Ok(())
}
