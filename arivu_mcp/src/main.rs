use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{error, info};

use arivu_core::{
    mcp_server::{JsonRpcHandler, McpServer},
    transport::StdioTransport,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt().init();

    info!("Starting Arivu MCP Server");

    // Create provider registry with only feature-enabled connectors (with usage metering)
    let registry = match arivu_core::UsageManager::new_default() {
        Ok(usage) => arivu_core::build_registry_enabled_only_with_usage(Arc::new(usage)).await,
        Err(err) => {
            error!(
                "Usage manager init failed, continuing without metering: {}",
                err
            );
            arivu_core::build_registry_enabled_only().await
        }
    };

    // Note: Set authentication at runtime via the MCP methods if needed.

    // Create Arc<Mutex<ProviderRegistry>> for thread-safe access
    let registry = Arc::new(Mutex::new(registry));

    // Create MCP server
    let server = McpServer::new(registry);

    // Create JSON-RPC handler
    let handler = JsonRpcHandler::new(server);

    // Create and run stdio transport
    let transport = StdioTransport::new(handler);

    info!("MCP Server ready, listening on stdio");

    // Run the transport
    if let Err(e) = transport.run().await {
        error!("Transport error: {}", e);
        return Err(e.into());
    }

    Ok(())
}
