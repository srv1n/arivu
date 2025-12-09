use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, error, info};

use crate::{
    auth::AuthDetails,
    capabilities::{ConnectorConfigSchema, FieldType},
    utils::structured_result_with_text,
    ConnectorError, ProviderRegistry,
};
use rmcp::model::*;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct AuthState {
    authorized: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    authorized_at: Option<String>,
}

/// MCP Server implementation that wraps the ProviderRegistry
pub struct McpServer {
    registry: Arc<Mutex<ProviderRegistry>>,
    auth_status: Arc<Mutex<std::collections::HashMap<String, AuthState>>>,
}

impl McpServer {
    pub fn new(registry: Arc<Mutex<ProviderRegistry>>) -> Self {
        Self {
            registry,
            auth_status: Arc::new(Mutex::new(std::collections::HashMap::new())),
        }
    }

    /// Get aggregated capabilities from all connectors
    pub async fn get_capabilities(&self) -> ServerCapabilities {
        let registry = self.registry.lock().await;
        let mut capabilities = ServerCapabilities::default();

        // Check if any connector supports tools
        for (_name, connector) in registry.providers.iter() {
            let conn = connector.lock().await;
            let conn_caps = conn.capabilities().await;
            if conn_caps.tools.is_some() {
                capabilities.tools = conn_caps.tools;
            }
            if conn_caps.resources.is_some() {
                capabilities.resources = conn_caps.resources;
            }
            if conn_caps.prompts.is_some() {
                capabilities.prompts = conn_caps.prompts;
            }
        }

        capabilities
    }

    /// Handle initialize request
    pub async fn handle_initialize(
        &self,
        _request: InitializeRequestParam,
    ) -> Result<InitializeResult, ConnectorError> {
        info!("MCP Server initializing");

        Ok(InitializeResult {
            protocol_version: ProtocolVersion::LATEST,
            capabilities: self.get_capabilities().await,
            server_info: Implementation {
                name: "rzn_datasourcer".to_string(),
                title: None,
                version: env!("CARGO_PKG_VERSION").to_string(),
                icons: None,
                website_url: None,
            },
            instructions: Some("Multi-connector data sourcing server supporting various data sources including academic papers, social media, search engines, and more.".to_string()),
        })
    }

    /// Handle list_resources request - aggregates from all connectors
    pub async fn handle_list_resources(
        &self,
        request: Option<PaginatedRequestParam>,
    ) -> Result<ListResourcesResult, ConnectorError> {
        let registry = self.registry.lock().await;
        let mut all_resources = Vec::new();

        // Collect resources from all connectors
        for (_name, connector) in registry.providers.iter() {
            let c = connector.lock().await;
            match c.list_resources(request.clone()).await {
                Ok(response) => {
                    all_resources.extend(response.resources);
                }
                Err(e) => {
                    error!("Error listing resources from connector: {:?}", e);
                }
            }
        }

        Ok(ListResourcesResult {
            resources: all_resources,
            next_cursor: None,
        })
    }

    /// Handle read_resource request - routes to appropriate connector
    pub async fn handle_read_resource(
        &self,
        request: ReadResourceRequestParam,
    ) -> Result<Vec<ResourceContents>, ConnectorError> {
        let registry = self.registry.lock().await;

        // Try each connector until one handles the resource
        for (_name, connector) in registry.providers.iter() {
            let c = connector.lock().await;
            match c.read_resource(request.clone()).await {
                Ok(contents) => return Ok(contents),
                Err(ConnectorError::ResourceNotFound) => continue,
                Err(e) => return Err(e),
            }
        }

        Err(ConnectorError::ResourceNotFound)
    }

    /// Handle list_tools request - aggregates from all connectors
    pub async fn handle_list_tools(
        &self,
        request: Option<PaginatedRequestParam>,
    ) -> Result<ListToolsResult, ConnectorError> {
        let registry = self.registry.lock().await;
        let mut all_tools = Vec::new();

        // Collect tools from all connectors
        for (connector_name, connector) in registry.providers.iter() {
            let c = connector.lock().await;
            match c.list_tools(request.clone()).await {
                Ok(response) => {
                    // Prefix tool names with connector name to avoid conflicts
                    let prefixed_tools: Vec<Tool> = response
                        .tools
                        .into_iter()
                        .map(|mut tool| {
                            tool.name = format!("{}/{}", connector_name, tool.name).into();
                            tool
                        })
                        .collect();
                    all_tools.extend(prefixed_tools);
                }
                Err(e) => {
                    error!(
                        "Error listing tools from connector {}: {:?}",
                        connector_name, e
                    );
                }
            }
        }

        // Add generic auth tools per connector following MCP tool semantics
        for (connector_name, connector) in registry.providers.iter() {
            let c = connector.lock().await;
            let schema = c.config_schema();
            drop(c);

            // auth/<provider>/set
            let set_tool = Tool {
                name: format!("auth/{}/set", connector_name).into(),
                title: None,
                description: Some(format!(
                    "Set credentials for '{}' (tokens, OAuth results, or basic credentials) following MCP tool flow.",
                    connector_name
                ).into()),
                input_schema: Arc::new(config_schema_to_jsonschema(&schema)),
                output_schema: None,
                annotations: None,
                icons: None,
            };
            all_tools.push(set_tool);

            // auth/<provider>/test
            let test_tool = Tool {
                name: format!("auth/{}/test", connector_name).into(),
                title: None,
                description: Some("Test authentication for the connector.".into()),
                input_schema: Arc::new(
                    serde_json::json!({"type":"object","properties":{}})
                        .as_object()
                        .expect("Schema must be an object")
                        .clone(),
                ),
                output_schema: None,
                annotations: None,
                icons: None,
            };
            all_tools.push(test_tool);

            // auth/<provider>/get_schema
            let schema_tool = Tool {
                name: format!("auth/{}/get_schema", connector_name).into(),
                title: None,
                description: Some(
                    "Return JSON schema for connector credentials (fields/types).".into(),
                ),
                input_schema: Arc::new(
                    serde_json::json!({"type":"object","properties":{}})
                        .as_object()
                        .expect("Schema must be an object")
                        .clone(),
                ),
                output_schema: None,
                annotations: None,
                icons: None,
            };
            all_tools.push(schema_tool);

            // Provider-specific OAuth device-code helpers
            match connector_name.as_str() {
                "microsoft-graph" => {
                    // start_device
                    let start = Tool {
                        name: format!("auth/{}/start_device", connector_name).into(),
                        title: None,
                        description: Some("Start Microsoft device authorization (returns user_code and verify URL).".into()),
                        input_schema: Arc::new(serde_json::json!({
                            "type":"object",
                            "properties":{
                                "tenant_id": {"type":"string"},
                                "client_id": {"type":"string"},
                                "scopes": {"type":"string", "description":"space-separated scopes, e.g. Mail.Read Calendars.Read"}
                            },
                            "required":["client_id","scopes"]
                        }).as_object().unwrap().clone()),
                        output_schema: None,
                        annotations: None,
                        icons: None,
                    };
                    all_tools.push(start);
                    // poll_device
                    let poll = Tool {
                        name: format!("auth/{}/poll_device", connector_name).into(),
                        title: None,
                        description: Some(
                            "Poll token endpoint for device flow using device_code (Microsoft)."
                                .into(),
                        ),
                        input_schema: Arc::new(
                            serde_json::json!({
                                "type":"object",
                                "properties":{
                                    "tenant_id": {"type":"string"},
                                    "client_id": {"type":"string"},
                                    "device_code": {"type":"string"}
                                },
                                "required":["client_id","device_code"]
                            })
                            .as_object()
                            .unwrap()
                            .clone(),
                        ),
                        output_schema: None,
                        annotations: None,
                        icons: None,
                    };
                    all_tools.push(poll);
                }
                "github" => {
                    let start = Tool {
                        name: format!("auth/{}/start_device", connector_name).into(),
                        title: None,
                        description: Some("Start GitHub device flow (returns user_code and verify URL).".into()),
                        input_schema: Arc::new(serde_json::json!({
                            "type":"object",
                            "properties":{
                                "client_id": {"type":"string"},
                                "scope": {"type":"string", "description":"space-separated scopes, e.g. repo read:org"}
                            },
                            "required":["client_id"]
                        }).as_object().unwrap().clone()),
                        output_schema: None,
                        annotations: None,
                        icons: None,
                    };
                    all_tools.push(start);
                    let poll = Tool {
                        name: format!("auth/{}/poll_device", connector_name).into(),
                        title: None,
                        description: Some("Poll GitHub for access token using device_code.".into()),
                        input_schema: Arc::new(serde_json::json!({
                            "type":"object",
                            "properties":{
                                "client_id": {"type":"string"},
                                "device_code": {"type":"string"},
                                "client_secret": {"type":"string", "description":"optional for OAuth App"}
                            },
                            "required":["client_id","device_code"]
                        }).as_object().unwrap().clone()),
                        output_schema: None,
                        annotations: None,
                        icons: None,
                    };
                    all_tools.push(poll);
                }
                _ => {}
            }
        }

        Ok(ListToolsResult {
            tools: all_tools,
            next_cursor: None,
        })
    }

    /// Handle call_tool request - routes to appropriate connector
    pub async fn handle_call_tool(
        &self,
        request: CallToolRequestParam,
    ) -> Result<CallToolResult, ConnectorError> {
        // Support auth tools: auth/<provider>/set|test|get_schema
        if request.name.starts_with("auth/") {
            let parts: Vec<&str> = request.name.split('/').collect();
            if parts.len() != 3 {
                return Err(ConnectorError::InvalidInput(
                    "Auth tool must be 'auth/<provider>/<action>'".into(),
                ));
            }
            let provider = parts[1];
            let action = parts[2];

            let registry = self.registry.lock().await;
            let connector = registry
                .providers
                .get(provider)
                .ok_or_else(|| {
                    ConnectorError::InvalidInput(format!("Unknown connector: {}", provider))
                })?
                .clone();

            match action {
                "set" => {
                    // Accept arbitrary object matching connector config schema
                    let args_map = request.arguments.unwrap_or_default();
                    let mut details: AuthDetails = AuthDetails::new();
                    for (k, v) in args_map.into_iter() {
                        if let Some(s) = v.as_str() {
                            details.insert(k, s.to_string());
                        } else {
                            // best-effort stringify primitive values
                            if v.is_number() || v.is_boolean() || v.is_null() {
                                details.insert(k, v.to_string());
                            }
                        }
                    }
                    let mut c = connector.lock().await;
                    c.set_auth_details(details).await?;
                    return structured_result_with_text(&serde_json::json!({"ok": true}), None);
                }
                "test" => {
                    let c = connector.lock().await;
                    c.test_auth().await?;
                    return structured_result_with_text(&serde_json::json!({"ok": true}), None);
                }
                "get_schema" => {
                    let c = connector.lock().await;
                    let schema = c.config_schema();
                    let js = config_schema_to_jsonschema(&schema);
                    return structured_result_with_text(&serde_json::json!({"schema": js}), None);
                }
                // Device flow helpers: forward to connector tools
                "start_device" => {
                    let mut req = request.clone();
                    req.name = "auth_start".into();
                    let c = connector.lock().await;
                    return c.call_tool(req).await;
                }
                "poll_device" => {
                    let mut req = request.clone();
                    req.name = "auth_poll".into();
                    let c = connector.lock().await;
                    return c.call_tool(req).await;
                }
                _ => return Err(ConnectorError::ToolNotFound),
            }
        }

        // Parse connector name from tool name (format: "connector/tool")
        let parts: Vec<&str> = request.name.split('/').collect();
        if parts.len() != 2 {
            return Err(ConnectorError::InvalidInput(format!(
                "Tool name must be in format 'connector/tool' or 'auth/<provider>/<action>', got: {}",
                request.name
            )));
        }

        let connector_name = parts[0];
        let tool_name = parts[1];

        let registry = self.registry.lock().await;

        if let Some(connector) = registry.providers.get(connector_name) {
            // Create a new request with the unprefixed tool name
            let unprefixed_request = CallToolRequestParam {
                name: tool_name.to_string().into(),
                arguments: request.arguments,
            };

            let c = connector.lock().await;
            c.call_tool(unprefixed_request).await
        } else {
            Err(ConnectorError::InvalidInput(format!(
                "Unknown connector: {}",
                connector_name
            )))
        }
    }

    /// Handle list_prompts request - aggregates from all connectors
    pub async fn handle_list_prompts(
        &self,
        request: Option<PaginatedRequestParam>,
    ) -> Result<ListPromptsResult, ConnectorError> {
        let registry = self.registry.lock().await;
        let mut all_prompts = Vec::new();

        // Collect prompts from all connectors
        for (connector_name, connector) in registry.providers.iter() {
            let c = connector.lock().await;
            match c.list_prompts(request.clone()).await {
                Ok(response) => {
                    // Prefix prompt names with connector name
                    let prefixed_prompts: Vec<Prompt> = response
                        .prompts
                        .into_iter()
                        .map(|mut prompt| {
                            prompt.name = format!("{}/{}", connector_name, prompt.name);
                            prompt
                        })
                        .collect();
                    all_prompts.extend(prefixed_prompts);
                }
                Err(e) => {
                    error!(
                        "Error listing prompts from connector {}: {:?}",
                        connector_name, e
                    );
                }
            }
        }

        Ok(ListPromptsResult {
            prompts: all_prompts,
            next_cursor: None,
        })
    }

    /// Handle get_prompt request - routes to appropriate connector
    pub async fn handle_get_prompt(&self, name: &str) -> Result<Prompt, ConnectorError> {
        // Parse connector name from prompt name
        let parts: Vec<&str> = name.split('/').collect();
        if parts.len() != 2 {
            return Err(ConnectorError::InvalidInput(format!(
                "Prompt name must be in format 'connector/prompt', got: {}",
                name
            )));
        }

        let connector_name = parts[0];
        let prompt_name = parts[1];

        let registry = self.registry.lock().await;

        if let Some(connector) = registry.providers.get(connector_name) {
            let c = connector.lock().await;
            let mut prompt = c.get_prompt(prompt_name).await?;
            // Re-prefix the name in the response
            prompt.name = name.to_string();
            Ok(prompt)
        } else {
            Err(ConnectorError::InvalidInput(format!(
                "Unknown connector: {}",
                connector_name
            )))
        }
    }
}

fn config_schema_to_jsonschema(
    schema: &ConnectorConfigSchema,
) -> serde_json::Map<String, serde_json::Value> {
    use serde_json::json;
    let mut props = serde_json::Map::new();
    let mut required: Vec<String> = Vec::new();
    for f in &schema.fields {
        let (ty, extra) = match &f.field_type {
            FieldType::Text => ("string", json!({})),
            FieldType::Secret => ("string", json!({"format":"password"})),
            FieldType::Number => ("number", json!({})),
            FieldType::Boolean => ("boolean", json!({})),
            FieldType::Select { options } => {
                let opts = options.clone();
                ("string", json!({"enum": opts}))
            }
        };
        let mut obj = serde_json::Map::new();
        obj.insert("type".to_string(), json!(ty));
        if let Some(desc) = &f.description {
            obj.insert("description".to_string(), json!(desc));
        }
        for (k, v) in extra
            .as_object()
            .expect("Schema extra properties must be an object")
            .iter()
        {
            obj.insert(k.clone(), v.clone());
        }
        props.insert(f.name.clone(), serde_json::Value::Object(obj));
        if f.required {
            required.push(f.name.clone());
        }
    }
    let mut root = serde_json::Map::new();
    root.insert("type".to_string(), json!("object"));
    root.insert("properties".to_string(), serde_json::Value::Object(props));
    if !required.is_empty() {
        root.insert("required".to_string(), json!(required));
    }
    root
}

/// JSON-RPC message handler for the MCP server
pub struct JsonRpcHandler {
    server: McpServer,
}

impl JsonRpcHandler {
    pub fn new(server: McpServer) -> Self {
        Self { server }
    }

    /// Process a JSON-RPC request and return a response
    pub async fn handle_request(&self, request: Value) -> Value {
        debug!("Handling JSON-RPC request: {:?}", request);

        let id = request.get("id").cloned();
        let method = request.get("method").and_then(|m| m.as_str()).unwrap_or("");
        let params = request.get("params").cloned().unwrap_or(json!({}));

        let result = match method {
            "authorization/describe" => {
                // Static scheme map based on current connectors
                let schemes = json!({
                    "schemes": [
                        {
                            "provider": "reddit",
                            "type": "basic",
                            "fields": [
                                {"name": "client_id", "label": "Client ID", "kind": "string", "required": true},
                                {"name": "client_secret", "label": "Client Secret", "kind": "secret", "required": true},
                                {"name": "username", "label": "Username", "kind": "string", "required": true},
                                {"name": "password", "label": "Password", "kind": "secret", "required": true}
                            ],
                            "notes": "Uses Reddit 'script' OAuth internally; public endpoints still work anonymously.",
                            "requires_auth": "optional"
                        },
                        {
                            "provider": "x",
                            "type": "basic",
                            "fields": [
                                {"name": "username", "label": "Username", "kind": "string", "required": true},
                                {"name": "password", "label": "Password", "kind": "secret", "required": true}
                            ],
                            "hints": {"browser_cookies": true},
                            "notes": "Login or import browser cookies for higher reliability.",
                            "requires_auth": "optional"
                        },
                        {
                            "provider": "semantic_scholar",
                            "type": "api_key",
                            "fields": [
                                {"name": "SEMANTIC_SCHOLAR_API_KEY", "label": "API Key", "kind": "secret", "required": true}
                            ],
                            "requires_auth": "optional"
                        },
                        {"provider": "youtube", "type": "none", "hints": {"browser_cookies": true}},
                        {"provider": "web", "type": "none", "hints": {"browser_cookies": true}},
                        {"provider": "arxiv", "type": "none"},
                        {"provider": "pubmed", "type": "none"},
                        {"provider": "wikipedia", "type": "none"},
                        {"provider": "hackernews", "type": "none"},
                        {"provider": "scihub", "type": "none"}
                    ]
                });
                Ok(schemes)
            }
            "authorization/status" => {
                let map = self.server.auth_status.lock().await.clone();
                let providers: Vec<Value> = map
                    .into_iter()
                    .map(|(provider, st)| {
                        json!({
                            "provider": provider,
                            "authorized": st.authorized,
                            "authorized_at": st.authorized_at,
                        })
                    })
                    .collect();
                Ok(json!({"providers": providers}))
            }
            "secrets/set" => {
                // params: { provider: string, secrets: object }
                let provider = params
                    .get("provider")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let secrets = params
                    .get("secrets")
                    .and_then(|v| v.as_object())
                    .cloned()
                    .unwrap_or_default();
                if provider.is_empty() {
                    return json!({"error": "Missing provider"});
                } else {
                    // Map secrets (Value map) -> AuthDetails
                    let mut details = AuthDetails::new();
                    for (k, v) in secrets {
                        if let Some(s) = v.as_str() {
                            details.insert(k, s.to_string());
                        } else {
                            details.insert(k, v.to_string());
                        }
                    }
                    // Apply to connector and test
                    let registry = self.server.registry.lock().await;
                    match registry.providers.get(&provider) {
                        Some(conn) => {
                            let mut c = conn.lock().await;
                            // Map JSON secrets into AuthDetails for the connector
                            if let Err(e) = c.set_auth_details(details).await {
                                return json!(e.to_jsonrpc_error());
                            }
                            if let Err(e) = c.test_auth().await {
                                return json!(e.to_jsonrpc_error());
                            }
                            drop(c);
                            drop(registry);
                            let mut status = self.server.auth_status.lock().await;
                            status.insert(
                                provider.clone(),
                                AuthState {
                                    authorized: true,
                                    authorized_at: Some(chrono::Utc::now().to_rfc3339()),
                                },
                            );
                            Ok(json!({"ok": true}))
                        }
                        None => Err(ConnectorError::InvalidInput(format!(
                            "Unknown provider: {}",
                            provider
                        ))
                        .to_jsonrpc_error()),
                    }
                }
            }
            "secrets/delete" => {
                let provider = params
                    .get("provider")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                if provider.is_empty() {
                    Err(
                        ConnectorError::InvalidParams("Missing provider".to_string())
                            .to_jsonrpc_error(),
                    )
                } else {
                    let mut status = self.server.auth_status.lock().await;
                    status.insert(
                        provider,
                        AuthState {
                            authorized: false,
                            authorized_at: None,
                        },
                    );
                    Ok(json!({"ok": true}))
                }
            }
            "initialize" => match serde_json::from_value::<InitializeRequestParam>(params) {
                Ok(req) => self
                    .server
                    .handle_initialize(req)
                    .await
                    .and_then(|r| serde_json::to_value(r).map_err(ConnectorError::SerdeJson))
                    .map_err(|e| e.to_jsonrpc_error()),
                Err(e) => Err(ConnectorError::SerdeJson(e).to_jsonrpc_error()),
            },
            "resources/list" => {
                match serde_json::from_value::<Option<PaginatedRequestParam>>(params) {
                    Ok(req) => self
                        .server
                        .handle_list_resources(req)
                        .await
                        .and_then(|r| serde_json::to_value(r).map_err(ConnectorError::SerdeJson))
                        .map_err(|e| e.to_jsonrpc_error()),
                    Err(e) => Err(ConnectorError::SerdeJson(e).to_jsonrpc_error()),
                }
            }
            "resources/read" => match serde_json::from_value::<ReadResourceRequestParam>(params) {
                Ok(req) => self
                    .server
                    .handle_read_resource(req)
                    .await
                    .and_then(|r| serde_json::to_value(r).map_err(ConnectorError::SerdeJson))
                    .map_err(|e| e.to_jsonrpc_error()),
                Err(e) => Err(ConnectorError::SerdeJson(e).to_jsonrpc_error()),
            },
            "tools/list" => match serde_json::from_value::<Option<PaginatedRequestParam>>(params) {
                Ok(req) => self
                    .server
                    .handle_list_tools(req)
                    .await
                    .and_then(|r| serde_json::to_value(r).map_err(ConnectorError::SerdeJson))
                    .map_err(|e| e.to_jsonrpc_error()),
                Err(e) => Err(ConnectorError::SerdeJson(e).to_jsonrpc_error()),
            },
            "tools/call" => match serde_json::from_value::<CallToolRequestParam>(params) {
                Ok(req) => self
                    .server
                    .handle_call_tool(req)
                    .await
                    .and_then(|r| serde_json::to_value(r).map_err(ConnectorError::SerdeJson))
                    .map_err(|e| e.to_jsonrpc_error()),
                Err(e) => Err(ConnectorError::SerdeJson(e).to_jsonrpc_error()),
            },
            "prompts/list" => {
                match serde_json::from_value::<Option<PaginatedRequestParam>>(params) {
                    Ok(req) => self
                        .server
                        .handle_list_prompts(req)
                        .await
                        .and_then(|r| serde_json::to_value(r).map_err(ConnectorError::SerdeJson))
                        .map_err(|e| e.to_jsonrpc_error()),
                    Err(e) => Err(ConnectorError::SerdeJson(e).to_jsonrpc_error()),
                }
            }
            "prompts/get" => match params.get("name").and_then(|n| n.as_str()) {
                Some(name) => self
                    .server
                    .handle_get_prompt(name)
                    .await
                    .and_then(|r| serde_json::to_value(r).map_err(ConnectorError::SerdeJson))
                    .map_err(|e| e.to_jsonrpc_error()),
                None => Err(
                    ConnectorError::InvalidInput("Missing 'name' parameter".to_string())
                        .to_jsonrpc_error(),
                ),
            },
            _ => Err(ConnectorError::MethodNotFound.to_jsonrpc_error()),
        };

        match result {
            Ok(result) => json!({
                "jsonrpc": "2.0",
                "result": result,
                "id": id,
            }),
            Err(error) => json!({
                "jsonrpc": "2.0",
                "error": error,
                "id": id,
            }),
        }
    }
}
