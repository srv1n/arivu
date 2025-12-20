use crate::capabilities::{ConnectorConfigSchema, Field, FieldType};
use crate::error::ConnectorError;
use crate::utils::structured_result_with_text;
use crate::{auth::AuthDetails, Connector};
use async_trait::async_trait;
use reqwest::Client;
use rmcp::model::*;
use serde::Deserialize;
use serde_json::{json, Value};
use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::Arc;

/// Response format for controlling output verbosity
#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ResponseFormat {
    #[default]
    Concise,
    Detailed,
}

// Define the structs for search arguments
#[derive(Debug, Deserialize)]
struct SearchArgs {
    query: String,
    #[serde(default = "default_limit")]
    limit: u32,
    #[serde(default)]
    response_format: ResponseFormat,
}

#[derive(Debug, Deserialize)]
struct GeoSearchArgs {
    latitude: f64,
    longitude: f64,
    #[serde(default = "default_radius")]
    radius: u16,
}

#[derive(Debug, Deserialize)]
struct GetArticleArgs {
    title: String,
    #[serde(default)]
    response_format: ResponseFormat,
}

fn default_limit() -> u32 {
    10
}

fn default_radius() -> u16 {
    1000
}

// Define the Wikipedia connector
pub struct WikipediaConnector {
    client: Client,
    language: String,
    search_limit: u32,
}

impl WikipediaConnector {
    pub async fn new(auth: AuthDetails) -> Result<Self, ConnectorError> {
        let client = Client::builder()
            .user_agent("rzn_datasourcer/0.1.0")
            .build()
            .map_err(|e| ConnectorError::Other(e.to_string()))?;

        let language = auth.get("language").unwrap_or(&"en".to_string()).clone();
        let search_limit = auth
            .get("search_limit")
            .and_then(|l| l.parse::<u32>().ok())
            .unwrap_or(10);

        Ok(WikipediaConnector {
            client,
            language,
            search_limit,
        })
    }

    // Helper method to get the base API URL
    fn base_url(&self) -> String {
        format!("https://{}.wikipedia.org/w/api.php", self.language)
    }

    // Helper method to format article content
    fn format_article(
        &self,
        title: &str,
        content: &str,
        summary: Option<&str>,
    ) -> HashMap<String, Value> {
        let mut result = HashMap::new();

        result.insert("title".to_string(), json!(title));
        result.insert("content".to_string(), json!(content));

        if let Some(summary) = summary {
            result.insert("summary".to_string(), json!(summary));
        }

        result
    }

    // Search for articles
    async fn search_articles(
        &self,
        query: &str,
        limit: u32,
    ) -> Result<Vec<String>, ConnectorError> {
        let params = [
            ("list", "search"),
            ("srprop", ""),
            ("srlimit", &limit.to_string()),
            ("srsearch", query),
            ("format", "json"),
            ("action", "query"),
        ];

        let response = self
            .client
            .get(self.base_url())
            .query(&params)
            .send()
            .await
            .map_err(ConnectorError::HttpRequest)?;

        let data: Value = response.json().await.map_err(ConnectorError::HttpRequest)?;

        let results = data
            .get("query")
            .and_then(|q| q.get("search"))
            .and_then(|s| s.as_array())
            .ok_or_else(|| ConnectorError::Other("Invalid response format".to_string()))?;

        let titles = results
            .iter()
            .filter_map(|item| {
                item.get("title")
                    .and_then(|t| t.as_str())
                    .map(|s| s.to_string())
            })
            .collect();

        Ok(titles)
    }

    // Geo search for articles
    async fn geo_search(
        &self,
        latitude: f64,
        longitude: f64,
        radius: u16,
    ) -> Result<Vec<String>, ConnectorError> {
        if !(-90.0..=90.0).contains(&latitude) {
            return Err(ConnectorError::InvalidParams(
                "latitude must be between -90 and 90".to_string(),
            ));
        }
        if !(-180.0..=180.0).contains(&longitude) {
            return Err(ConnectorError::InvalidParams(
                "longitude must be between -180 and 180".to_string(),
            ));
        }
        if !(10..=10000).contains(&radius) {
            return Err(ConnectorError::InvalidParams(
                "radius must be between 10 and 10000".to_string(),
            ));
        }

        let params = [
            ("list", "geosearch"),
            ("gsradius", &radius.to_string()),
            ("gscoord", &format!("{}|{}", latitude, longitude)),
            ("gslimit", &self.search_limit.to_string()),
            ("format", "json"),
            ("action", "query"),
        ];

        let response = self
            .client
            .get(self.base_url())
            .query(&params)
            .send()
            .await
            .map_err(ConnectorError::HttpRequest)?;

        let data: Value = response.json().await.map_err(ConnectorError::HttpRequest)?;

        let results = data
            .get("query")
            .and_then(|q| q.get("geosearch"))
            .and_then(|s| s.as_array())
            .ok_or_else(|| ConnectorError::Other("Invalid response format".to_string()))?;

        let titles = results
            .iter()
            .filter_map(|item| {
                item.get("title")
                    .and_then(|t| t.as_str())
                    .map(|s| s.to_string())
            })
            .collect();

        Ok(titles)
    }

    // Get article content
    async fn get_article_content(&self, title: &str) -> Result<String, ConnectorError> {
        let params = [
            ("prop", "extracts"),
            ("explaintext", ""),
            ("redirects", ""),
            ("titles", title),
            ("format", "json"),
            ("action", "query"),
        ];

        let response = self
            .client
            .get(self.base_url())
            .query(&params)
            .send()
            .await
            .map_err(ConnectorError::HttpRequest)?;

        let data: Value = response.json().await.map_err(ConnectorError::HttpRequest)?;

        let pages = data
            .get("query")
            .and_then(|q| q.get("pages"))
            .and_then(|p| p.as_object())
            .ok_or_else(|| ConnectorError::Other("Invalid response format".to_string()))?;

        // Get the first page (there should only be one)
        let page = pages
            .values()
            .next()
            .ok_or_else(|| ConnectorError::ResourceNotFound)?;

        // Check if the page has a "missing" field, which indicates the article doesn't exist
        if page.get("missing").is_some() {
            return Err(ConnectorError::ResourceNotFound);
        }

        // Try to get the extract, or return a default message if not found
        let content = page
            .get("extract")
            .and_then(|e| e.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("No content found for article: {}", title));

        Ok(content)
    }

    // Get article summary
    async fn get_article_summary(&self, title: &str) -> Result<String, ConnectorError> {
        let params = [
            ("prop", "extracts"),
            ("explaintext", ""),
            ("exintro", ""),
            ("redirects", ""),
            ("titles", title),
            ("format", "json"),
            ("action", "query"),
        ];

        let response = self
            .client
            .get(self.base_url())
            .query(&params)
            .send()
            .await
            .map_err(ConnectorError::HttpRequest)?;

        let data: Value = response.json().await.map_err(ConnectorError::HttpRequest)?;

        let pages = data
            .get("query")
            .and_then(|q| q.get("pages"))
            .and_then(|p| p.as_object())
            .ok_or_else(|| ConnectorError::Other("Invalid response format".to_string()))?;

        // Get the first page (there should only be one)
        let page = pages
            .values()
            .next()
            .ok_or_else(|| ConnectorError::ResourceNotFound)?;

        let summary = page
            .get("extract")
            .and_then(|e| e.as_str())
            .ok_or_else(|| ConnectorError::Other("No summary found".to_string()))?
            .to_string();

        Ok(summary)
    }
}

#[async_trait]
impl Connector for WikipediaConnector {
    fn name(&self) -> &'static str {
        "wikipedia"
    }

    fn description(&self) -> &'static str {
        "A connector for searching and retrieving content from Wikipedia."
    }

    async fn capabilities(&self) -> ServerCapabilities {
        ServerCapabilities {
            tools: None,
            ..Default::default()
        }
    }

    async fn get_auth_details(&self) -> Result<AuthDetails, ConnectorError> {
        let mut auth = AuthDetails::new();
        auth.insert("language".to_string(), self.language.clone());
        auth.insert("search_limit".to_string(), self.search_limit.to_string());
        Ok(auth)
    }

    async fn set_auth_details(&mut self, details: AuthDetails) -> Result<(), ConnectorError> {
        if let Some(language) = details.get("language") {
            self.language = language.clone();
        }

        if let Some(limit) = details
            .get("search_limit")
            .and_then(|l| l.parse::<u32>().ok())
        {
            self.search_limit = limit;
        }

        Ok(())
    }

    async fn test_auth(&self) -> Result<(), ConnectorError> {
        // Simple test to check if the API is accessible
        tracing::debug!("Testing Wikipedia connector auth");
        self.search_articles("test", 1).await?;
        tracing::debug!("Wikipedia auth test succeeded");
        Ok(())
    }

    fn config_schema(&self) -> ConnectorConfigSchema {
        ConnectorConfigSchema {
            fields: vec![
                Field {
                    name: "language".to_string(),
                    label: "Language".to_string(),
                    field_type: FieldType::Text,
                    required: false,
                    description: Some(
                        "Wikipedia language code (e.g., 'en' for English, 'es' for Spanish)"
                            .to_string(),
                    ),
                    options: None,
                },
                Field {
                    name: "search_limit".to_string(),
                    label: "Search Results Limit".to_string(),
                    field_type: FieldType::Number,
                    required: false,
                    description: Some("Maximum number of search results to return".to_string()),
                    options: None,
                },
            ],
        }
    }

    async fn initialize(
        &self,
        _request: InitializeRequestParam,
    ) -> Result<InitializeResult, ConnectorError> {
        Ok(InitializeResult {
            protocol_version: ProtocolVersion::LATEST,
            capabilities: self.capabilities().await,
            server_info: Implementation {
                name: self.name().to_string(),
                title: None,
                version: "0.1.0".to_string(),
                icons: None,
                website_url: None,
            },
            instructions: Some("MCP connector for various data sources".to_string()),
        })
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParam>,
    ) -> Result<ListResourcesResult, ConnectorError> {
        let resources = vec![Resource {
            raw: RawResource {
                uri: "wikipedia://article/{title}".to_string(),
                name: "Wikipedia Article".to_string(),
                title: None,
                description: Some("Represents a Wikipedia article.".to_string()),
                mime_type: Some("application/vnd.wikipedia.article+json".to_string()),
                size: None,
                icons: None,
            },
            annotations: None,
        }];

        Ok(ListResourcesResult {
            resources,
            next_cursor: None,
        })
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParam,
    ) -> Result<Vec<ResourceContents>, ConnectorError> {
        let uri_str = request.uri.as_str();

        if uri_str.starts_with("wikipedia://article/") {
            let parts: Vec<&str> = uri_str.split('/').collect();
            if parts.len() < 4 {
                return Err(ConnectorError::InvalidInput(format!(
                    "Invalid resource URI: {}",
                    uri_str
                )));
            }
            let title = parts[3];

            let content = self.get_article_content(title).await?;
            let article_data = self.format_article(title, &content, None);
            let _json_content = serde_json::to_string(&article_data)?;

            let content_text = serde_json::to_string(&article_data)?;
            Ok(vec![ResourceContents::text(content_text, uri_str)])
        } else {
            Err(ConnectorError::ResourceNotFound)
        }
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParam>,
    ) -> Result<ListToolsResult, ConnectorError> {
        let tools =
            vec![
            Tool {
                name: Cow::Borrowed("search"),
                title: None,
                description: Some(Cow::Borrowed("Keyword search for articles.")),
                input_schema: Arc::new(json!({
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "The search query (e.g., 'quantum computing')"
                        },
                        "limit": {
                            "type": "integer",
                            "description": "Maximum number of results to return (default: 10)"
                        },
                        "response_format": {
                            "type": "string",
                            "enum": ["concise", "detailed"],
                            "description": "Response verbosity: 'concise' returns only article titles, 'detailed' includes query metadata",
                            "default": "concise"
                        }
                    },
                    "required": ["query"]
                }).as_object().expect("Schema object").clone()),
                output_schema: None,
                annotations: None,
                icons: None,
            },
            Tool {
                name: Cow::Borrowed("geosearch"),
                title: None,
                description: Some(Cow::Borrowed("Articles near a lat/lon.")),
                input_schema: Arc::new(json!({
                    "type": "object",
                    "properties": {
                        "latitude": {
                            "type": "number",
                            "description": "Latitude coordinate."
                        },
                        "longitude": {
                            "type": "number",
                            "description": "Longitude coordinate."
                        },
                        "radius": {
                            "type": "integer",
                            "description": "Search radius in meters (default: 1000)."
                        }
                    },
                    "required": ["latitude", "longitude"]
                }).as_object().expect("Schema object").clone()),
                output_schema: None,
                annotations: None,
                icons: None,
            },
            Tool {
                name: Cow::Borrowed("get_article"),
                title: None,
                description: Some(Cow::Borrowed("Article content by title.")),
                input_schema: Arc::new(json!({
                    "type": "object",
                    "properties": {
                        "title": {
                            "type": "string",
                            "description": "The title of the article (e.g., 'Rust (programming language)')"
                        },
                        "response_format": {
                            "type": "string",
                            "enum": ["concise", "detailed"],
                            "description": "Response verbosity: 'concise' returns only title and summary (first paragraph), 'detailed' includes full content",
                            "default": "concise"
                        }
                    },
                    "required": ["title"]
                }).as_object().expect("Schema object").clone()),
                output_schema: None,
                annotations: None,
                icons: None,
            },
        ];

        Ok(ListToolsResult {
            tools,
            next_cursor: None,
        })
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParam,
    ) -> Result<CallToolResult, ConnectorError> {
        let name = request.name.as_ref();
        let args = request.arguments.unwrap_or_default();

        match name {
            "search" => {
                let args: SearchArgs = serde_json::from_value(json!(args)).map_err(|e| {
                    ConnectorError::InvalidParams(format!("Invalid arguments: {}", e))
                })?;

                let results = self.search_articles(&args.query, args.limit).await?;

                // Return concise or detailed based on response_format
                let data = if args.response_format == ResponseFormat::Concise {
                    json!({ "results": results })
                } else {
                    json!({
                        "query": args.query,
                        "limit": args.limit,
                        "results": results,
                        "count": results.len()
                    })
                };
                let text = serde_json::to_string(&data)?;
                Ok(structured_result_with_text(&data, Some(text))?)
            }
            "geosearch" => {
                let args: GeoSearchArgs = serde_json::from_value(json!(args)).map_err(|e| {
                    ConnectorError::InvalidParams(format!("Invalid arguments: {}", e))
                })?;

                let results = self
                    .geo_search(args.latitude, args.longitude, args.radius)
                    .await?;
                let data = json!({
                    "latitude": args.latitude,
                    "longitude": args.longitude,
                    "radius": args.radius,
                    "results": results,
                    "count": results.len()
                });
                let text = serde_json::to_string(&data)?;
                Ok(structured_result_with_text(&data, Some(text))?)
            }
            "get_article" => {
                let args: GetArticleArgs = serde_json::from_value(json!(args)).map_err(|e| {
                    ConnectorError::InvalidParams(format!("Invalid arguments: {}", e))
                })?;

                match self.get_article_content(&args.title).await {
                    Ok(content) => {
                        let summary = self.get_article_summary(&args.title).await.ok();

                        // Return concise or detailed based on response_format
                        let article_data = if args.response_format == ResponseFormat::Concise {
                            // Concise: just title and summary (first paragraph)
                            let mut result = HashMap::new();
                            result.insert("title".to_string(), json!(args.title));
                            if let Some(ref s) = summary {
                                result.insert("summary".to_string(), json!(s));
                            }
                            result
                        } else {
                            self.format_article(&args.title, &content, summary.as_deref())
                        };
                        let text = serde_json::to_string(&article_data)?;
                        Ok(structured_result_with_text(&article_data, Some(text))?)
                    }
                    Err(ConnectorError::ResourceNotFound) => {
                        let payload = json!({
                            "title": args.title,
                            "content": serde_json::Value::Null,
                            "summary": serde_json::Value::Null,
                        });
                        let text = serde_json::to_string(&payload)?;
                        Ok(structured_result_with_text(&payload, Some(text))?)
                    }
                    Err(err) => Err(err),
                }
            }
            _ => Err(ConnectorError::ToolNotFound),
        }
    }

    async fn list_prompts(
        &self,
        _request: Option<PaginatedRequestParam>,
    ) -> Result<ListPromptsResult, ConnectorError> {
        let prompts = vec![Prompt {
            name: "summarize_article".to_string(),
            title: None,
            description: Some("Summarizes a Wikipedia article.".to_string()),
            arguments: Some(vec![PromptArgument {
                name: "title".to_string(),
                title: None,
                description: Some("The title of the article to summarize.".to_string()),
                required: Some(true),
            }]),
            icons: None,
        }];

        Ok(ListPromptsResult {
            prompts,
            next_cursor: None,
        })
    }

    async fn get_prompt(&self, name: &str) -> Result<Prompt, ConnectorError> {
        match name {
            "summarize_article" => Ok(Prompt {
                name: "summarize_article".to_string(),
                title: None,
                description: Some("Summarizes a Wikipedia article.".to_string()),
                arguments: Some(vec![PromptArgument {
                    name: "title".to_string(),
                    title: None,
                    description: Some("The title of the article to summarize.".to_string()),
                    required: Some(true),
                }]),
                icons: None,
            }),
            _ => Err(ConnectorError::InvalidParams(format!(
                "Prompt with name {} not found",
                name
            ))),
        }
    }
}
