use arivu_core::connectors::hackernews::HackerNewsConnector;
use arivu_core::{CallToolRequestParam, Connector};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let connector = HackerNewsConnector::new();
    connector.test_auth().await?;

    let search_response = connector
        .call_tool(CallToolRequestParam {
            name: "search_stories".into(),
            arguments: Some(
                json!({
                    "query": "rust cli",
                    "hitsPerPage": 5
                })
                .as_object()
                .unwrap()
                .clone(),
            ),
        })
        .await?;

    let structured = search_response
        .structured_content
        .unwrap_or_else(|| json!({}));
    println!(
        "Search response:\n{}",
        serde_json::to_string_pretty(&structured)?
    );

    // Try to fetch the first result (if present)
    let first_id = structured
        .get("hits")
        .and_then(|v| v.as_array())
        .and_then(|hits| hits.first())
        .and_then(|hit| hit.get("objectID"))
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<i64>().ok());

    if let Some(id) = first_id {
        let post_response = connector
            .call_tool(CallToolRequestParam {
                name: "get_post".into(),
                arguments: Some(
                    json!({ "id": id, "flatten": true })
                        .as_object()
                        .unwrap()
                        .clone(),
                ),
            })
            .await?;
        let post_structured = post_response
            .structured_content
            .unwrap_or_else(|| json!({}));
        println!(
            "First post:\n{}",
            serde_json::to_string_pretty(&post_structured)?
        );
    }

    Ok(())
}
