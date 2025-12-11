use crate::capabilities::ConnectorConfigSchema;
use crate::error::ConnectorError;
use crate::utils::structured_result_with_text;
use crate::{auth::AuthDetails, Connector};
use async_trait::async_trait;
use feed_rs::parser;
use reqwest::Client;
use rmcp::model::*;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::borrow::Cow;
use std::collections::HashMap;
use std::io::Cursor;
use std::sync::Arc;

#[derive(Debug, Deserialize)]
struct GetFeedArgs {
    url: String,
    limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct ListEntriesArgs {
    url: String,
    limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct SearchFeedArgs {
    url: String,
    query: String,
    limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct DiscoverFeedsArgs {
    url: String,
}

pub struct RssConnector {
    client: Client,
}

impl RssConnector {
    pub async fn new(_auth: AuthDetails) -> Result<Self, ConnectorError> {
        Ok(Self {
            client: Client::builder()
                .user_agent("arivu-rss-connector/0.1.0")
                .build()
                .map_err(ConnectorError::HttpRequest)?,
        })
    }

    async fn fetch_and_parse(&self, url: &str) -> Result<feed_rs::model::Feed, ConnectorError> {
        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(ConnectorError::HttpRequest)?;

        if !response.status().is_success() {
            return Err(ConnectorError::Other(format!(
                "Failed to fetch feed: {}",
                response.status()
            )));
        }

        let bytes = response
            .bytes()
            .await
            .map_err(ConnectorError::HttpRequest)?;
        let cursor = Cursor::new(bytes);

        parser::parse(cursor)
            .map_err(|e| ConnectorError::Other(format!("Failed to parse feed: {}", e)))
    }
}

#[async_trait]
impl Connector for RssConnector {
    fn name(&self) -> &'static str {
        "rss"
    }

    fn description(&self) -> &'static str {
        "Fetch and parse RSS/Atom feeds"
    }

    async fn capabilities(&self) -> ServerCapabilities {
        ServerCapabilities {
            tools: None,
            ..Default::default()
        }
    }

    async fn get_auth_details(&self) -> Result<AuthDetails, ConnectorError> {
        Ok(AuthDetails::new())
    }

    async fn set_auth_details(&mut self, _details: AuthDetails) -> Result<(), ConnectorError> {
        Ok(())
    }

    async fn test_auth(&self) -> Result<(), ConnectorError> {
        Ok(())
    }

    fn config_schema(&self) -> ConnectorConfigSchema {
        ConnectorConfigSchema { fields: Vec::new() }
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
            instructions: Some("Fetch and read RSS/Atom/JSON feeds.".to_string()),
        })
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParam>,
    ) -> Result<ListToolsResult, ConnectorError> {
        let tools = vec![
            Tool {
                name: Cow::Borrowed("get_feed"),
                title: None,
                description: Some(Cow::Borrowed(
                    "Fetch and parse a feed, returning metadata and recent entries",
                )),
                input_schema: Arc::new(
                    json!({
                        "type": "object",
                        "properties": {
                            "url": {
                                "type": "string",
                                "description": "URL of the RSS/Atom feed"
                            },
                            "limit": {
                                "type": "integer",
                                "description": "Number of entries to return (default: 5)"
                            }
                        },
                        "required": ["url"]
                    })
                    .as_object()
                    .expect("Schema object")
                    .clone(),
                ),
                output_schema: None,
                annotations: None,
                icons: None,
            },
            Tool {
                name: Cow::Borrowed("list_entries"),
                title: None,
                description: Some(Cow::Borrowed("List entries from a feed with a limit")),
                input_schema: Arc::new(
                    json!({
                        "type": "object",
                        "properties": {
                            "url": {
                                "type": "string",
                                "description": "URL of the RSS/Atom feed"
                            },
                            "limit": {
                                "type": "integer",
                                "description": "Number of entries to return (default: 10)"
                            }
                        },
                        "required": ["url"]
                    })
                    .as_object()
                    .expect("Schema object")
                    .clone(),
                ),
                output_schema: None,
                annotations: None,
                icons: None,
            },
            Tool {
                name: Cow::Borrowed("search_feed"),
                title: None,
                description: Some(Cow::Borrowed(
                    "Search entries within a feed by keyword in title or summary",
                )),
                input_schema: Arc::new(
                    json!({
                        "type": "object",
                        "properties": {
                            "url": {
                                "type": "string",
                                "description": "URL of the RSS/Atom feed"
                            },
                            "query": {
                                "type": "string",
                                "description": "Keyword to search for in entry titles or summaries"
                            },
                            "limit": {
                                "type": "integer",
                                "description": "Number of matching entries to return (default: 10)"
                            }
                        },
                        "required": ["url", "query"]
                    })
                    .as_object()
                    .expect("Schema object")
                    .clone(),
                ),
                output_schema: None,
                annotations: None,
                icons: None,
            },
            Tool {
                name: Cow::Borrowed("discover_feeds"),
                title: None,
                description: Some(Cow::Borrowed(
                    "Find RSS/Atom/JSON feeds on a given webpage by inspecting link tags",
                )),
                input_schema: Arc::new(
                    json!({
                        "type": "object",
                        "properties": {
                            "url": {
                                "type": "string",
                                "description": "URL of the webpage to inspect"
                            }
                        },
                        "required": ["url"]
                    })
                    .as_object()
                    .expect("Schema object")
                    .clone(),
                ),
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
        match request.name.as_ref() {
            "get_feed" => {
                let args: GetFeedArgs = serde_json::from_value(
                    serde_json::to_value(request.arguments.unwrap_or_default())
                        .map_err(ConnectorError::SerdeJson)?,
                )
                .map_err(|e| ConnectorError::InvalidParams(e.to_string()))?;

                let feed = self.fetch_and_parse(&args.url).await?;
                let limit = args.limit.unwrap_or(5);

                // Convert feed-rs model to JSON
                // We'll construct a simplified version to avoid huge blobs
                let entries: Vec<Value> = feed
                    .entries
                    .iter()
                    .take(limit)
                    .map(|e| {
                        json!({
                            "id": e.id,
                            "title": e.title.as_ref().map(|t| t.content.clone()),
                            "link": e.links.first().map(|l| l.href.clone()),
                            "published": e.published.map(|d| d.to_rfc3339()),
                            "summary": e.summary.as_ref().map(|s| s.content.clone()),
                        })
                    })
                    .collect();

                let data = json!({
                    "title": feed.title.as_ref().map(|t| t.content.clone()),
                    "description": feed.description.as_ref().map(|d| d.content.clone()),
                    "link": feed.links.first().map(|l| l.href.clone()),
                    "entries_count": feed.entries.len(),
                    "entries": entries // First 5
                });

                Ok(structured_result_with_text(
                    &data,
                    Some(serde_json::to_string(&data)?),
                )?)
            }
            "list_entries" => {
                let args: ListEntriesArgs = serde_json::from_value(
                    serde_json::to_value(request.arguments.unwrap_or_default())
                        .map_err(ConnectorError::SerdeJson)?,
                )
                .map_err(|e| ConnectorError::InvalidParams(e.to_string()))?;

                let feed = self.fetch_and_parse(&args.url).await?;
                let limit = args.limit.unwrap_or(10);

                let entries: Vec<Value> = feed.entries.iter().take(limit).map(|e| {
                    json!({
                        "id": e.id,
                        "title": e.title.as_ref().map(|t| t.content.clone()),
                        "link": e.links.first().map(|l| l.href.clone()),
                        "published": e.published.map(|d| d.to_rfc3339()),
                        "updated": e.updated.map(|d| d.to_rfc3339()),
                        "summary": e.summary.as_ref().map(|s| s.content.clone()),
                        "content": e.content.as_ref().map(|c| c.body.clone().unwrap_or_default()),
                        "authors": e.authors.iter().map(|a| a.name.clone()).collect::<Vec<_>>()
                    })
                }).collect();

                let data = json!({
                    "url": args.url,
                    "count": entries.len(),
                    "entries": entries
                });

                Ok(structured_result_with_text(
                    &data,
                    Some(serde_json::to_string(&data)?),
                )?)
            }
            "search_feed" => {
                let args: SearchFeedArgs = serde_json::from_value(
                    serde_json::to_value(request.arguments.unwrap_or_default())
                        .map_err(ConnectorError::SerdeJson)?,
                )
                .map_err(|e| ConnectorError::InvalidParams(e.to_string()))?;

                let feed = self.fetch_and_parse(&args.url).await?;
                let limit = args.limit.unwrap_or(10);
                let query_lower = args.query.to_lowercase();

                let entries: Vec<Value> = feed.entries.iter()
                    .filter(|e| {
                        let title_match = e.title.as_ref().map_or(false, |t| t.content.to_lowercase().contains(&query_lower));
                        let summary_match = e.summary.as_ref().map_or(false, |s| s.content.to_lowercase().contains(&query_lower));
                        title_match || summary_match
                    })
                    .take(limit)
                    .map(|e| {
                    json!({
                        "id": e.id,
                        "title": e.title.as_ref().map(|t| t.content.clone()),
                        "link": e.links.first().map(|l| l.href.clone()),
                        "published": e.published.map(|d| d.to_rfc3339()),
                        "updated": e.updated.map(|d| d.to_rfc3339()),
                        "summary": e.summary.as_ref().map(|s| s.content.clone()),
                        "content": e.content.as_ref().map(|c| c.body.clone().unwrap_or_default()),
                        "authors": e.authors.iter().map(|a| a.name.clone()).collect::<Vec<_>>()
                    })
                }).collect();

                let data = json!({
                    "url": args.url,
                    "query": args.query,
                    "count": entries.len(),
                    "results": entries
                });

                Ok(structured_result_with_text(
                    &data,
                    Some(serde_json::to_string(&data)?),
                )?)
            }
            "discover_feeds" => {
                let args: DiscoverFeedsArgs = serde_json::from_value(
                    serde_json::to_value(request.arguments.unwrap_or_default())
                        .map_err(ConnectorError::SerdeJson)?,
                )
                .map_err(|e| ConnectorError::InvalidParams(e.to_string()))?;

                let response = self
                    .client
                    .get(&args.url)
                    .send()
                    .await
                    .map_err(ConnectorError::HttpRequest)?;

                if !response.status().is_success() {
                    return Err(ConnectorError::Other(format!(
                        "Failed to fetch webpage: {}",
                        response.status()
                    )));
                }

                let html_content = response.text().await.map_err(ConnectorError::HttpRequest)?;
                let document = Html::parse_document(&html_content);

                let selector = Selector::parse("link[rel='alternate'][type*='rss'], link[rel='alternate'][type*='atom'], link[rel='alternate'][type*='json']").unwrap();

                let mut feeds = Vec::new();
                for element in document.select(&selector) {
                    if let Some(href) = element.value().attr("href") {
                        feeds.push(json!({
                            "url": href,
                            "title": element.value().attr("title"),
                            "type": element.value().attr("type"),
                        }));
                    }
                }

                let data = json!({
                    "searched_url": args.url,
                    "found_feeds": feeds,
                });

                Ok(structured_result_with_text(
                    &data,
                    Some(serde_json::to_string(&data)?),
                )?)
            }
            _ => Err(ConnectorError::ToolNotFound),
        }
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParam>,
    ) -> Result<ListResourcesResult, ConnectorError> {
        Ok(ListResourcesResult {
            resources: vec![],
            next_cursor: None,
        })
    }

    async fn read_resource(
        &self,
        _request: ReadResourceRequestParam,
    ) -> Result<Vec<ResourceContents>, ConnectorError> {
        Err(ConnectorError::ResourceNotFound)
    }

    async fn list_prompts(
        &self,
        _request: Option<PaginatedRequestParam>,
    ) -> Result<ListPromptsResult, ConnectorError> {
        Ok(ListPromptsResult {
            prompts: vec![],
            next_cursor: None,
        })
    }

    async fn get_prompt(&self, name: &str) -> Result<Prompt, ConnectorError> {
        Err(ConnectorError::InvalidParams(format!(
            "Prompt '{}' not found",
            name
        )))
    }
}
