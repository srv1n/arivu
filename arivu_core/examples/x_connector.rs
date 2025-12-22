use arivu_core::auth::AuthDetails;
use arivu_core::connectors::x::XConnector;
use arivu_core::{CallToolRequestParam, Connector, PaginatedRequestParam};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // NOTE: The X connector typically requires browser cookies.
    // Set auth details as needed for your environment (e.g., browser="chrome").
    let mut auth = AuthDetails::new();
    auth.insert("browser".to_string(), "chrome".to_string());

    let connector = XConnector::new(auth).await?;
    let tools = connector
        .list_tools(Some(PaginatedRequestParam { cursor: None }))
        .await?
        .tools;
    println!("Tools:");
    for t in tools {
        println!("  - {}: {}", t.name, t.description.unwrap_or_default());
    }

    let resp = connector
        .call_tool(CallToolRequestParam {
            name: "search".into(),
            arguments: Some(
                json!({
                    "query": "rust lang",
                    "limit": 5
                })
                .as_object()
                .unwrap()
                .clone(),
            ),
        })
        .await?;

    let structured = resp.structured_content.unwrap_or_else(|| json!({}));
    println!("{}", serde_json::to_string_pretty(&structured)?);

    Ok(())
}
