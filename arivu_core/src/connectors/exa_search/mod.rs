use crate::capabilities::{ConnectorConfigSchema, Field, FieldType};
use crate::error::ConnectorError;
use crate::utils::{resolve_search_filters, structured_result_with_text};
use crate::{auth::AuthDetails, Connector};
use async_trait::async_trait;
use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::Client;
use rmcp::model::*;
use serde_json::{json, Value};
use std::borrow::Cow;
use std::sync::Arc;

pub struct ExaSearchConnector {
    client: Client,
    api_key: Option<String>,
}

impl ExaSearchConnector {
    pub async fn new(auth: AuthDetails) -> Result<Self, ConnectorError> {
        let client = Client::builder()
            .user_agent("rzn_datasourcer/0.1.0")
            .build()
            .map_err(|e| ConnectorError::Other(e.to_string()))?;
        let api_key = auth
            .get("api_key")
            .cloned()
            .or_else(|| std::env::var("EXA_API_KEY").ok());
        Ok(Self { client, api_key })
    }
}

#[async_trait]
impl Connector for ExaSearchConnector {
    fn name(&self) -> &'static str {
        "exa-search"
    }
    fn description(&self) -> &'static str {
        "Exa.ai web search and retrieval (fast, AI-friendly SERP)."
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
            instructions: Some(
                "Use 'search' to query Exa; set livecrawl=true to crawl fresh pages.".into(),
            ),
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
            description: Some(Cow::Borrowed("Exa.ai web search. Prefer concise questions; avoid over-constraining. By default returns minimal fields; set response_format='detailed' to include raw payload.")),
            input_schema: Arc::new(json!({
                "type": "object",
                "properties": {
                    "query": {"type": "string"},
                    "max_results": {"type": "integer", "default": 10},
                    "livecrawl": {"type": "boolean", "default": false, "description": "Crawl pages live for freshness"},
                    "include_text": {"type": "boolean", "default": false, "description": "Return page text content"},
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
        let limit = args
            .get("max_results")
            .or_else(|| args.get("limit"))
            .and_then(|v| v.as_u64())
            .unwrap_or(10) as usize;
        let livecrawl = args
            .get("livecrawl")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let include_text = args
            .get("include_text")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let detailed = args
            .get("response_format")
            .and_then(|v| v.as_str())
            .map(|s| s == "detailed")
            .unwrap_or(false);

        let key = self.api_key.as_ref().ok_or_else(|| ConnectorError::InvalidInput("Missing credentials: set EXA_API_KEY or use rzn config set exa-search {\"api_key\":\"...\"}".into()))?;
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-api-key",
            HeaderValue::from_str(key).map_err(|e| ConnectorError::Other(e.to_string()))?,
        );

        let mut body = json!({ "query": query, "num_results": limit });
        let filters = resolve_search_filters(&args);
        let include_domains = filters.include_domains.clone();
        let exclude_domains = filters.exclude_domains.clone();
        if livecrawl {
            body["livecrawl"] = json!(true);
        }
        if include_text {
            body["include_text"] = json!(true);
        }
        if !include_domains.is_empty() {
            body["includeDomains"] = json!(include_domains);
        }
        if !exclude_domains.is_empty() {
            body["excludeDomains"] = json!(exclude_domains);
        }

        let resp = self
            .client
            .post("https://api.exa.ai/search")
            .headers(headers)
            .json(&body)
            .send()
            .await
            .map_err(ConnectorError::HttpRequest)?;
        let status = resp.status();
        let value: Value = resp.json().await.map_err(ConnectorError::HttpRequest)?;
        if !status.is_success() {
            return Err(ConnectorError::Other(format!(
                "Exa API error: {} - {}",
                status, value
            )));
        }

        let mut data = json!({
            "provider": "exa",
            "query": query,
            "limit_hint": limit,
            "livecrawl": livecrawl,
            "results": value.get("results").cloned().unwrap_or_else(|| json!([]))
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
        Ok(auth)
    }
    async fn set_auth_details(&mut self, details: AuthDetails) -> Result<(), ConnectorError> {
        self.api_key = details
            .get("api_key")
            .cloned()
            .or_else(|| std::env::var("EXA_API_KEY").ok());
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
            fields: vec![Field {
                name: "api_key".into(),
                label: "Exa API Key".into(),
                field_type: FieldType::Secret,
                required: true,
                description: Some("Set EXA_API_KEY or provide here".into()),
                options: None,
            }],
        }
    }
}
