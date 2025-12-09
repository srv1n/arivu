use crate::capabilities::{ConnectorConfigSchema, Field, FieldType};
use crate::error::ConnectorError;
use crate::utils::{build_filters_clause, resolve_search_filters, structured_result_with_text};
use crate::{auth::AuthDetails, Connector};
use async_trait::async_trait;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use reqwest::Client;
use rmcp::model::*;
use serde_json::{json, Value};
use std::borrow::Cow;
use std::sync::Arc;

#[derive(Clone)]
pub struct XaiSearchConnector {
    client: Client,
    api_key: Option<String>,
    default_model: String,
}

impl XaiSearchConnector {
    pub async fn new(auth: AuthDetails) -> Result<Self, ConnectorError> {
        let client = Client::builder()
            .user_agent("rzn_datasourcer/0.1.0")
            .build()
            .map_err(|e| ConnectorError::Other(e.to_string()))?;
        let api_key = auth
            .get("api_key")
            .cloned()
            .or_else(|| std::env::var("XAI_API_KEY").ok());
        let default_model = auth
            .get("model")
            .cloned()
            .unwrap_or_else(|| "grok-4-fast".to_string());

        Ok(Self {
            client,
            api_key,
            default_model,
        })
    }
}

#[async_trait]
impl Connector for XaiSearchConnector {
    fn name(&self) -> &'static str {
        "xai-search"
    }
    fn description(&self) -> &'static str {
        "Search the web and X (Twitter) via xAI Grok with live search and citations."
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
            instructions: Some("Use 'search' to query with live web/X sources via xAI.".into()),
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
        let tool = Tool {
            name: Cow::Borrowed("search"),
            title: None,
            description: Some(Cow::Borrowed("Live search across web and/or X via xAI. Provide a clear question; avoid adding years unless specified. response_format='concise' omits raw payload; use 'detailed' only when needed.")),
            input_schema: Arc::new(json!({
                "type": "object",
                "properties": {
                    "query": {"type": "string", "description": "User query"},
                    "sources": {
                        "type": "array",
                        "description": "Sources to search: web, x",
                        "items": {"type": "string", "enum": ["web", "x"]},
                        "default": ["web"]
                    },
                    "mode": {"type": "string", "enum": ["auto", "on", "off"], "default": "auto", "description": "Search mode"},
                    "max_results": {"type": "integer", "default": 5, "description": "Approximate citations to include"},
                    "model": {"type": "string", "description": "xAI model (e.g., grok-4-fast)"},
                    "language": {"type": "string", "description": "BCP-47 language hint (e.g., en)"},
                    "region": {"type": "string", "description": "Region/country code (e.g., US)"},
                    "since": {"type": "string", "description": "Earliest date (YYYY-MM-DD)"},
                    "until": {"type": "string", "description": "Latest date (YYYY-MM-DD)"},
                    "include_domains": {"type": "array", "items": {"type": "string"}},
                    "exclude_domains": {"type": "array", "items": {"type": "string"}},
                    "date_preset": {"type": "string", "description": "last_24_hours|last_7_days|last_30_days|this_month|past_year"},
                    "locale": {"type": "string", "description": "Locale like en-US or fr-FR"},
                    "response_format": {"type": "string", "enum": ["concise","detailed"], "default": "concise"}
                },
                "required": ["query"],
                "additionalProperties": false
            }).as_object().expect("Schema object").clone()),
            output_schema: None,
            annotations: None,
            icons: None,
        };
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
        let sources: Vec<String> = args
            .get("sources")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|s| s.as_str().map(|x| x.to_string()))
                    .collect()
            })
            .unwrap_or_else(|| vec!["web".to_string()]);
        let mode = args.get("mode").and_then(|v| v.as_str()).unwrap_or("auto");
        let limit = args
            .get("max_results")
            .or_else(|| args.get("limit"))
            .and_then(|v| v.as_u64())
            .unwrap_or(5) as usize;
        let model = args
            .get("model")
            .and_then(|v| v.as_str())
            .unwrap_or(&self.default_model);
        let detailed = args
            .get("response_format")
            .and_then(|v| v.as_str())
            .map(|s| s == "detailed")
            .unwrap_or(false);

        let key = self.api_key.as_ref().ok_or_else(|| ConnectorError::InvalidInput("Missing credentials: set XAI_API_KEY or use rzn config set xai-search {\"api_key\":\"...\"}".into()))?;
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", key))
                .map_err(|e| ConnectorError::Other(e.to_string()))?,
        );
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        let src_objs: Vec<Value> = sources.iter().map(|s| json!({"type": s})).collect();
        let filters = resolve_search_filters(&args);
        let include_domains = filters.include_domains.clone();
        let exclude_domains = filters.exclude_domains.clone();
        let filters_clause = build_filters_clause(&filters);

        let mut search_params = json!({
            "mode": mode,
            "return_citations": true,
            "max_search_results": limit,
            "sources": src_objs
        });
        if !include_domains.is_empty() {
            search_params["allowed_domains"] = json!(include_domains);
        }
        if !exclude_domains.is_empty() {
            search_params["excluded_domains"] = json!(exclude_domains);
        }

        let body = json!({
            "model": model,
            "messages": [ { "role": "user", "content": format!("Use live search to answer and cite ~{} sources. Question: {}{}", limit, query, filters_clause) } ],
            "search_parameters": search_params
        });

        let resp = self
            .client
            .post("https://api.x.ai/v1/chat/completions")
            .headers(headers)
            .json(&body)
            .send()
            .await
            .map_err(ConnectorError::HttpRequest)?;
        let status = resp.status();
        let value: Value = resp.json().await.map_err(ConnectorError::HttpRequest)?;
        if !status.is_success() {
            return Err(ConnectorError::Other(format!(
                "xAI API error: {} - {}",
                status, value
            )));
        }

        let answer = value
            .get("choices")
            .and_then(|c| c.as_array())
            .and_then(|arr| arr.first())
            .and_then(|c| c.get("message"))
            .and_then(|m| m.get("content"))
            .and_then(|t| t.as_str())
            .unwrap_or("")
            .to_string();

        let citations = value.get("citations").cloned().unwrap_or_else(|| json!([]));

        let mut data = json!({
            "provider": "xai",
            "model": model,
            "query": query,
            "sources": sources,
            "limit_hint": limit,
            "answer": answer,
            "citations": citations
        });
        if detailed {
            data["raw"] = value.clone();
        }
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
        let mut auth = AuthDetails::new();
        if let Some(v) = &self.api_key {
            auth.insert("api_key".into(), v.clone());
        }
        auth.insert("model".into(), self.default_model.clone());
        Ok(auth)
    }
    async fn set_auth_details(&mut self, details: AuthDetails) -> Result<(), ConnectorError> {
        self.api_key = details
            .get("api_key")
            .cloned()
            .or_else(|| std::env::var("XAI_API_KEY").ok());
        if let Some(m) = details.get("model").cloned() {
            self.default_model = m;
        }
        Ok(())
    }
    async fn test_auth(&self) -> Result<(), ConnectorError> {
        if self
            .api_key
            .as_ref()
            .map(|s| !s.is_empty())
            .unwrap_or(false)
        {
            Ok(())
        } else {
            Err(ConnectorError::InvalidInput("Missing api_key".into()))
        }
    }
    fn config_schema(&self) -> ConnectorConfigSchema {
        ConnectorConfigSchema {
            fields: vec![
                Field {
                    name: "api_key".into(),
                    label: "xAI API Key".into(),
                    field_type: FieldType::Secret,
                    required: true,
                    description: Some("Set XAI_API_KEY".into()),
                    options: None,
                },
                Field {
                    name: "model".into(),
                    label: "Default Model".into(),
                    field_type: FieldType::Text,
                    required: false,
                    description: Some("e.g., grok-4-fast".into()),
                    options: None,
                },
            ],
        }
    }
}
