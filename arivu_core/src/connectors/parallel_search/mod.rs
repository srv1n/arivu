use crate::capabilities::{ConnectorConfigSchema, Field, FieldType};
use crate::error::ConnectorError;
use crate::utils::structured_result_with_text;
use crate::{auth::AuthDetails, Connector};
use async_trait::async_trait;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use reqwest::Client;
use rmcp::model::*;
use serde_json::{json, Value};
use std::borrow::Cow;
use std::sync::Arc;

pub struct ParallelSearchConnector {
    client: Client,
    api_key: Option<String>,
}

impl ParallelSearchConnector {
    pub async fn new(auth: AuthDetails) -> Result<Self, ConnectorError> {
        let client = Client::builder()
            .user_agent("rzn_datasourcer/0.1.0")
            .build()
            .map_err(|e| ConnectorError::Other(e.to_string()))?;
        let api_key = auth
            .get("api_key")
            .cloned()
            .or_else(|| std::env::var("PARALLEL_API_KEY").ok());
        Ok(Self { client, api_key })
    }
}

#[async_trait]
impl Connector for ParallelSearchConnector {
    fn name(&self) -> &'static str {
        "parallel-search"
    }
    fn description(&self) -> &'static str {
        "Parallel AI Search API - advanced, parallel web search with deep research capabilities."
    }
    async fn capabilities(&self) -> ServerCapabilities {
        ServerCapabilities {
            tools: None,
            ..Default::default()
        }
    }

    async fn initialize(
        &self,
        _r: InitializeRequestParam,
    ) -> Result<InitializeResult, ConnectorError> {
        Ok(InitializeResult {
            protocol_version: ProtocolVersion::LATEST,
            capabilities: self.capabilities().await,
            server_info: Implementation {
                name: self.name().into(),
                version: "0.1.0".into(),
                title: None,
                icons: None,
                website_url: None,
            },
            instructions: Some("Use 'search' for parallel web search.".into()),
        })
    }
    async fn list_resources(
        &self,
        _r: Option<PaginatedRequestParam>,
    ) -> Result<ListResourcesResult, ConnectorError> {
        Ok(ListResourcesResult {
            resources: vec![],
            next_cursor: None,
        })
    }
    async fn read_resource(
        &self,
        _r: ReadResourceRequestParam,
    ) -> Result<Vec<ResourceContents>, ConnectorError> {
        Ok(vec![])
    }

    async fn list_tools(
        &self,
        _r: Option<PaginatedRequestParam>,
    ) -> Result<ListToolsResult, ConnectorError> {
        let tool = Tool { name: Cow::Borrowed("search"), title: None, description: Some(Cow::Borrowed("Parallel AI web search. Provide a clear question for the 'objective' and optionally specific 'search_queries'.")), input_schema: Arc::new(json!({
            "type": "object",
            "properties": {
                "query": {"type": "string", "description": "The primary search query or objective"},
                "search_queries": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "Optional: Specific search queries to execute in parallel. If not provided, 'query' will be used."
                },
                "max_results": {"type": "integer", "default": 10, "description": "Maximum number of search results to return."},
                "include_domains": {"type": "array", "items": {"type": "string"}, "description": "Optional: Domains to include in the search."},
                "exclude_domains": {"type": "array", "items": {"type": "string"}, "description": "Optional: Domains to exclude from the search."}
            },
            "required": ["query"],
            "additionalProperties": false
        }).as_object().expect("Schema object").clone()), output_schema: None, annotations: None, icons: None };
        Ok(ListToolsResult {
            tools: vec![tool],
            next_cursor: None,
        })
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParam,
    ) -> Result<CallToolResult, ConnectorError> {
        if request.name.as_ref() != "search" {
            return Err(ConnectorError::ToolNotFound);
        }
        let args = request.arguments.unwrap_or_default();
        let query = args
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ConnectorError::InvalidParams("Missing 'query'".into()))?;

        let search_queries: Vec<String> = if let Some(queries_val) = args.get("search_queries") {
            if let Some(queries_array) = queries_val.as_array() {
                queries_array
                    .iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            } else {
                vec![query.to_string()]
            }
        } else {
            vec![query.to_string()]
        };

        let max_results = args
            .get("max_results")
            .and_then(|v| v.as_u64())
            .unwrap_or(10) as usize;

        let include_domains = args
            .get("include_domains")
            .and_then(|v| v.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|s| s.as_str().map(|x| x.to_string()))
                    .collect::<Vec<_>>()
            });
        let exclude_domains = args
            .get("exclude_domains")
            .and_then(|v| v.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|s| s.as_str().map(|x| x.to_string()))
                    .collect::<Vec<_>>()
            });

        let key = self.api_key.as_ref().ok_or_else(|| ConnectorError::InvalidInput("Missing credentials: set PARALLEL_API_KEY or use rzn config set parallel-search {\"api_key\":\"...\"}".into()))?;
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(
            "x-api-key",
            HeaderValue::from_str(key).map_err(|e| {
                ConnectorError::InvalidInput(format!("Invalid API Key header: {}", e))
            })?,
        );

        let mut body = json!({
            "objective": query,
            "search_queries": search_queries,
            "max_results": max_results,
            "excerpts": {
                "max_chars_per_result": 10000 // Default as per documentation
            }
        });
        if let Some(v) = include_domains {
            body["include_domains"] = json!(v);
        }
        if let Some(v) = exclude_domains {
            body["exclude_domains"] = json!(v);
        }

        let resp = self
            .client
            .post("https://api.parallel.ai/v1beta/search")
            .headers(headers)
            .json(&body)
            .send()
            .await
            .map_err(ConnectorError::HttpRequest)?;
        let status = resp.status();
        let value: Value = resp.json().await.map_err(ConnectorError::HttpRequest)?;

        if !status.is_success() {
            return Err(ConnectorError::Other(format!(
                "Parallel AI API error: {} - {}",
                status, value
            )));
        }

        let mut data = json!({
            "provider": "parallel-ai",
            "objective": query,
            "search_queries": search_queries,
            "max_results": max_results,
            "results": value.get("results").cloned().unwrap_or_else(|| json!([])),
            "raw": value.clone() // Include raw response for detailed inspection
        });

        Ok(structured_result_with_text(&data, None)?)
    }

    async fn list_prompts(
        &self,
        _r: Option<PaginatedRequestParam>,
    ) -> Result<ListPromptsResult, ConnectorError> {
        Ok(ListPromptsResult {
            prompts: vec![],
            next_cursor: None,
        })
    }
    async fn get_prompt(&self, _name: &str) -> Result<Prompt, ConnectorError> {
        Err(ConnectorError::ToolNotFound)
    }
    async fn get_auth_details(&self) -> Result<AuthDetails, ConnectorError> {
        let mut a = AuthDetails::new();
        if let Some(v) = &self.api_key {
            a.insert("api_key".into(), v.clone());
        }
        Ok(a)
    }
    async fn set_auth_details(&mut self, details: AuthDetails) -> Result<(), ConnectorError> {
        self.api_key = details
            .get("api_key")
            .cloned()
            .or_else(|| std::env::var("PARALLEL_API_KEY").ok());
        Ok(())
    }
    async fn test_auth(&self) -> Result<(), ConnectorError> {
        if self
            .api_key
            .as_ref()
            .map(|s| !s.is_empty())
            .unwrap_or(false)
        {
            // For a more robust test, one might make a dummy API call, but for simplicity,
            // just checking for API key presence is often sufficient for initial auth test.
            Ok(())
        } else {
            Err(ConnectorError::InvalidInput("Missing api_key".into()))
        }
    }
    fn config_schema(&self) -> ConnectorConfigSchema {
        ConnectorConfigSchema {
            fields: vec![Field {
                name: "api_key".into(),
                label: "Parallel AI API Key".into(),
                field_type: FieldType::Secret,
                required: true,
                description: Some("Set PARALLEL_API_KEY environment variable or configure via `rzn config set parallel-search`".into()),
                options: None,
            }],
        }
    }
}
