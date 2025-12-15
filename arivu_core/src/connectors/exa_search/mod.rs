use crate::capabilities::{ConnectorConfigSchema, Field, FieldType};
use crate::error::ConnectorError;
use crate::utils::structured_result_with_text;
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
            .user_agent("arivu/0.2.4")
            .build()
            .map_err(|e| ConnectorError::Other(e.to_string()))?;
        let api_key = auth
            .get("api_key")
            .cloned()
            .or_else(|| std::env::var("EXA_API_KEY").ok());
        Ok(Self { client, api_key })
    }

    fn get_headers(&self) -> Result<HeaderMap, ConnectorError> {
        let key = self.api_key.as_ref().ok_or_else(|| {
            ConnectorError::InvalidInput(
                "Missing credentials: set EXA_API_KEY or run 'arivu setup exa'".into(),
            )
        })?;
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-api-key",
            HeaderValue::from_str(key).map_err(|e| ConnectorError::Other(e.to_string()))?,
        );
        headers.insert("content-type", HeaderValue::from_static("application/json"));
        Ok(headers)
    }

    async fn search_impl(
        &self,
        args: &serde_json::Map<String, Value>,
    ) -> Result<CallToolResult, ConnectorError> {
        let query = args
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ConnectorError::InvalidParams("Missing 'query'".into()))?;

        let mut body = json!({
            "query": query,
            "numResults": args.get("num_results").and_then(|v| v.as_u64()).unwrap_or(10),
        });

        // Search type (neural, auto, fast, deep)
        if let Some(search_type) = args.get("type").and_then(|v| v.as_str()) {
            body["type"] = json!(search_type);
        }

        // Use autoprompt
        if let Some(use_autoprompt) = args.get("use_autoprompt").and_then(|v| v.as_bool()) {
            body["useAutoprompt"] = json!(use_autoprompt);
        }

        // Category filter
        if let Some(category) = args.get("category").and_then(|v| v.as_str()) {
            body["category"] = json!(category);
        }

        // Date filters
        if let Some(start_crawl) = args.get("start_crawl_date").and_then(|v| v.as_str()) {
            body["startCrawlDate"] = json!(start_crawl);
        }
        if let Some(end_crawl) = args.get("end_crawl_date").and_then(|v| v.as_str()) {
            body["endCrawlDate"] = json!(end_crawl);
        }
        if let Some(start_pub) = args.get("start_published_date").and_then(|v| v.as_str()) {
            body["startPublishedDate"] = json!(start_pub);
        }
        if let Some(end_pub) = args.get("end_published_date").and_then(|v| v.as_str()) {
            body["endPublishedDate"] = json!(end_pub);
        }

        // Domain filters
        if let Some(include_domains) = args.get("include_domains").and_then(|v| v.as_array()) {
            let domains: Vec<String> = include_domains
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect();
            if !domains.is_empty() {
                body["includeDomains"] = json!(domains);
            }
        }
        if let Some(exclude_domains) = args.get("exclude_domains").and_then(|v| v.as_array()) {
            let domains: Vec<String> = exclude_domains
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect();
            if !domains.is_empty() {
                body["excludeDomains"] = json!(domains);
            }
        }

        // Text filters
        if let Some(include_text) = args.get("include_text").and_then(|v| v.as_array()) {
            let texts: Vec<String> = include_text
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect();
            if !texts.is_empty() {
                body["includeText"] = json!(texts);
            }
        }
        if let Some(exclude_text) = args.get("exclude_text").and_then(|v| v.as_array()) {
            let texts: Vec<String> = exclude_text
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect();
            if !texts.is_empty() {
                body["excludeText"] = json!(texts);
            }
        }

        // Contents options
        if let Some(contents) = args.get("contents") {
            body["contents"] = contents.clone();
        } else {
            // Simple boolean flags for backward compatibility
            let mut contents_obj = json!({});
            if let Some(text) = args.get("text").and_then(|v| v.as_bool()) {
                contents_obj["text"] = json!(text);
            }
            if let Some(highlights) = args.get("highlights") {
                contents_obj["highlights"] = highlights.clone();
            }
            if let Some(summary) = args.get("summary") {
                contents_obj["summary"] = summary.clone();
            }
            if !contents_obj.as_object().unwrap().is_empty() {
                body["contents"] = contents_obj;
            }
        }

        let headers = self.get_headers()?;
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

        let detailed = args
            .get("response_format")
            .and_then(|v| v.as_str())
            .map(|s| s == "detailed")
            .unwrap_or(false);

        let mut data = json!({
            "provider": "exa",
            "query": query,
            "results": value.get("results").cloned().unwrap_or_else(|| json!([]))
        });

        if detailed {
            data["raw"] = value.clone();
        }

        structured_result_with_text(&data, None)
    }

    async fn get_contents_impl(
        &self,
        args: &serde_json::Map<String, Value>,
    ) -> Result<CallToolResult, ConnectorError> {
        let ids = args
            .get("ids")
            .and_then(|v| v.as_array())
            .ok_or_else(|| ConnectorError::InvalidParams("Missing 'ids' array".into()))?;

        let id_strings: Vec<String> = ids
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect();

        if id_strings.is_empty() {
            return Err(ConnectorError::InvalidParams("ids array is empty".into()));
        }

        let mut body = json!({
            "ids": id_strings,
        });

        // Contents options
        if let Some(text) = args.get("text") {
            body["text"] = text.clone();
        }
        if let Some(highlights) = args.get("highlights") {
            body["highlights"] = highlights.clone();
        }
        if let Some(summary) = args.get("summary") {
            body["summary"] = summary.clone();
        }
        if let Some(livecrawl) = args.get("livecrawl").and_then(|v| v.as_str()) {
            body["livecrawl"] = json!(livecrawl);
        }
        if let Some(subpages) = args.get("subpages").and_then(|v| v.as_u64()) {
            body["subpages"] = json!(subpages);
        }

        let headers = self.get_headers()?;
        let resp = self
            .client
            .post("https://api.exa.ai/contents")
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

        let data = json!({
            "provider": "exa",
            "operation": "get_contents",
            "results": value.get("results").cloned().unwrap_or_else(|| json!([]))
        });

        structured_result_with_text(&data, None)
    }

    async fn find_similar_impl(
        &self,
        args: &serde_json::Map<String, Value>,
    ) -> Result<CallToolResult, ConnectorError> {
        let url = args
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ConnectorError::InvalidParams("Missing 'url'".into()))?;

        let mut body = json!({
            "url": url,
            "numResults": args.get("num_results").and_then(|v| v.as_u64()).unwrap_or(10),
        });

        // Category filter
        if let Some(category) = args.get("category").and_then(|v| v.as_str()) {
            body["category"] = json!(category);
        }

        // Date filters
        if let Some(start_crawl) = args.get("start_crawl_date").and_then(|v| v.as_str()) {
            body["startCrawlDate"] = json!(start_crawl);
        }
        if let Some(end_crawl) = args.get("end_crawl_date").and_then(|v| v.as_str()) {
            body["endCrawlDate"] = json!(end_crawl);
        }

        // Domain filters
        if let Some(include_domains) = args.get("include_domains").and_then(|v| v.as_array()) {
            let domains: Vec<String> = include_domains
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect();
            if !domains.is_empty() {
                body["includeDomains"] = json!(domains);
            }
        }
        if let Some(exclude_domains) = args.get("exclude_domains").and_then(|v| v.as_array()) {
            let domains: Vec<String> = exclude_domains
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect();
            if !domains.is_empty() {
                body["excludeDomains"] = json!(domains);
            }
        }

        // Exclude source domain
        if let Some(exclude_source) = args.get("exclude_source_domain").and_then(|v| v.as_bool()) {
            body["excludeSourceDomain"] = json!(exclude_source);
        }

        // Contents options
        if let Some(contents) = args.get("contents") {
            body["contents"] = contents.clone();
        }

        let headers = self.get_headers()?;
        let resp = self
            .client
            .post("https://api.exa.ai/findSimilar")
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

        let data = json!({
            "provider": "exa",
            "operation": "find_similar",
            "url": url,
            "results": value.get("results").cloned().unwrap_or_else(|| json!([]))
        });

        structured_result_with_text(&data, None)
    }

    async fn answer_impl(
        &self,
        args: &serde_json::Map<String, Value>,
    ) -> Result<CallToolResult, ConnectorError> {
        let query = args
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ConnectorError::InvalidParams("Missing 'query'".into()))?;

        let mut body = json!({
            "query": query,
        });

        // Answer mode
        if let Some(mode) = args.get("mode").and_then(|v| v.as_str()) {
            body["mode"] = json!(mode);
        }

        // Number of search results to use
        if let Some(num_results) = args.get("num_results").and_then(|v| v.as_u64()) {
            body["numResults"] = json!(num_results);
        }

        // Category filter
        if let Some(category) = args.get("category").and_then(|v| v.as_str()) {
            body["category"] = json!(category);
        }

        // Include citations
        if let Some(include_citations) = args.get("include_citations").and_then(|v| v.as_bool()) {
            body["includeCitations"] = json!(include_citations);
        }

        let headers = self.get_headers()?;
        let resp = self
            .client
            .post("https://api.exa.ai/answer")
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

        let data = json!({
            "provider": "exa",
            "operation": "answer",
            "query": query,
            "answer": value.get("answer").cloned().unwrap_or(Value::Null),
            "citations": value.get("citations").cloned().unwrap_or_else(|| json!([])),
            "search_results": value.get("searchResults").cloned()
        });

        structured_result_with_text(&data, None)
    }
}

#[async_trait]
impl Connector for ExaSearchConnector {
    fn name(&self) -> &'static str {
        "exa"
    }
    fn description(&self) -> &'static str {
        "Exa.ai - Advanced AI search with neural search, similarity finding, content extraction, and answer generation."
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
                version: "0.2.0".into(),
                title: None,
                icons: None,
                website_url: None,
            },
            instructions: Some(
                "Exa tools: search (neural/fast/deep), get_contents (extract page content), find_similar (discover related pages), answer (get direct answers)".into(),
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
        let search_tool = Tool {
            name: Cow::Borrowed("search"),
            title: None,
            description: Some(Cow::Borrowed(
                "Intelligent web search using embeddings-based neural search. Supports auto, fast, and deep search modes with advanced filtering."
            )),
            input_schema: Arc::new(json!({
                "type": "object",
                "properties": {
                    "query": {"type": "string", "description": "Search query"},
                    "num_results": {"type": "integer", "default": 10, "maximum": 100, "description": "Number of results to return"},
                    "type": {"type": "string", "enum": ["neural", "auto", "fast", "deep"], "default": "auto", "description": "Search type: auto (intelligent), fast (<500ms), deep (comprehensive)"},
                    "use_autoprompt": {"type": "boolean", "description": "Let Exa rewrite query for better results"},
                    "category": {"type": "string", "enum": ["research paper", "news", "pdf", "github", "tweet", "company", "linkedin profile", "financial report"], "description": "Filter by content category"},
                    "start_crawl_date": {"type": "string", "description": "ISO 8601 datetime - results crawled after this date"},
                    "end_crawl_date": {"type": "string", "description": "ISO 8601 datetime - results crawled before this date"},
                    "start_published_date": {"type": "string", "description": "ISO 8601 datetime - content published after this date"},
                    "end_published_date": {"type": "string", "description": "ISO 8601 datetime - content published before this date"},
                    "include_domains": {"type": "array", "items": {"type": "string"}, "description": "Restrict to specific domains"},
                    "exclude_domains": {"type": "array", "items": {"type": "string"}, "description": "Exclude specific domains"},
                    "include_text": {"type": "array", "items": {"type": "string"}, "description": "Strings that must appear in results"},
                    "exclude_text": {"type": "array", "items": {"type": "string"}, "description": "Strings to exclude"},
                    "text": {"type": "boolean", "description": "Include page text content"},
                    "highlights": {"type": "object", "description": "Get highlighted snippets with similarity scores"},
                    "summary": {"type": "object", "description": "Get AI-generated summaries"},
                    "contents": {"type": "object", "description": "Advanced contents configuration"},
                    "response_format": {"type": "string", "enum": ["concise", "detailed"], "default": "concise"}
                },
                "required": ["query"]
            }).as_object().expect("Schema object").clone()),
            output_schema: None,
            annotations: None,
            icons: None,
        };

        let get_contents_tool = Tool {
            name: Cow::Borrowed("get_contents"),
            title: None,
            description: Some(Cow::Borrowed(
                "Retrieve clean, parsed content from URLs. Get text, highlights, summaries, and optionally crawl subpages."
            )),
            input_schema: Arc::new(json!({
                "type": "object",
                "properties": {
                    "ids": {"type": "array", "items": {"type": "string"}, "description": "Array of Exa result IDs or URLs"},
                    "text": {"type": ["boolean", "object"], "description": "Return page text. Can be boolean or object with maxCharacters, includeHtmlTags"},
                    "highlights": {"type": "object", "description": "Get highlighted snippets"},
                    "summary": {"type": "object", "description": "Get AI summaries"},
                    "livecrawl": {"type": "string", "enum": ["never", "fallback", "preferred", "always"], "description": "Livecrawl strategy"},
                    "subpages": {"type": "integer", "description": "Number of subpages to crawl"}
                },
                "required": ["ids"]
            }).as_object().expect("Schema object").clone()),
            output_schema: None,
            annotations: None,
            icons: None,
        };

        let find_similar_tool = Tool {
            name: Cow::Borrowed("find_similar"),
            title: None,
            description: Some(Cow::Borrowed(
                "Find webpages semantically similar to a given URL. Great for discovery and research."
            )),
            input_schema: Arc::new(json!({
                "type": "object",
                "properties": {
                    "url": {"type": "string", "description": "URL to find similar pages for"},
                    "num_results": {"type": "integer", "default": 10, "maximum": 100, "description": "Number of results"},
                    "category": {"type": "string", "description": "Filter by category"},
                    "start_crawl_date": {"type": "string", "description": "Results crawled after date"},
                    "end_crawl_date": {"type": "string", "description": "Results crawled before date"},
                    "include_domains": {"type": "array", "items": {"type": "string"}},
                    "exclude_domains": {"type": "array", "items": {"type": "string"}},
                    "exclude_source_domain": {"type": "boolean", "description": "Exclude the source domain from results"},
                    "contents": {"type": "object", "description": "Request page contents"}
                },
                "required": ["url"]
            }).as_object().expect("Schema object").clone()),
            output_schema: None,
            annotations: None,
            icons: None,
        };

        let answer_tool = Tool {
            name: Cow::Borrowed("answer"),
            title: None,
            description: Some(Cow::Borrowed(
                "Get direct LLM-generated answers informed by Exa search results. Supports streaming and citations."
            )),
            input_schema: Arc::new(json!({
                "type": "object",
                "properties": {
                    "query": {"type": "string", "description": "Question to answer"},
                    "mode": {"type": "string", "enum": ["precise", "detailed"], "description": "Answer mode: precise for facts, detailed for summaries"},
                    "num_results": {"type": "integer", "description": "Number of search results to use"},
                    "category": {"type": "string", "description": "Filter source category"},
                    "include_citations": {"type": "boolean", "default": true, "description": "Include source citations"}
                },
                "required": ["query"]
            }).as_object().expect("Schema object").clone()),
            output_schema: None,
            annotations: None,
            icons: None,
        };

        Ok(ListToolsResult {
            tools: vec![
                search_tool,
                get_contents_tool,
                find_similar_tool,
                answer_tool,
            ],
            next_cursor: None,
        })
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParam,
    ) -> Result<CallToolResult, ConnectorError> {
        let args = request.arguments.unwrap_or_default();

        match request.name.as_ref() {
            "search" => self.search_impl(&args).await,
            "get_contents" => self.get_contents_impl(&args).await,
            "find_similar" => self.find_similar_impl(&args).await,
            "answer" => self.answer_impl(&args).await,
            _ => Err(ConnectorError::ToolNotFound),
        }
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
                description: Some("Get your API key from https://dashboard.exa.ai/api-keys".into()),
                options: None,
            }],
        }
    }
}
