use crate::capabilities::ConnectorConfigSchema;
use crate::error::ConnectorError;
use crate::utils::structured_result_with_text;
use crate::{auth::AuthDetails, Connector};
use async_trait::async_trait;
use chrono::{Duration, Utc};
use reqwest::Client;
use rmcp::model::*;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct BiorxivPaper {
    pub doi: String,
    pub title: String,
    pub authors: String,
    pub author_corresponding: String,
    pub author_corresponding_institution: String,
    pub date: String,
    pub version: String,
    #[serde(rename = "type")]
    pub type_field: String,
    pub license: String,
    pub category: String,
    pub jatsxml: String,
    pub abstract_url: String, // Constructed manually usually, but API returns paths?
    pub published: String,
    pub server: String,
}

// API Response wrapper
#[derive(Debug, Deserialize)]
struct BiorxivResponse {
    messages: Vec<BiorxivMessage>,
    collection: Vec<BiorxivPaperRaw>,
}

#[derive(Debug, Deserialize)]
struct BiorxivMessage {
    status: String,
    #[allow(dead_code)]
    interval: Option<String>,
    #[allow(dead_code)]
    cursor: Option<Value>, // Can be string or int
    #[allow(dead_code)]
    count: Option<Value>, // API returns string or int
    #[allow(dead_code)]
    count_new_papers: Option<Value>, // API returns string
    #[allow(dead_code)]
    total: Option<Value>, // API returns string or int
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct BiorxivPaperRaw {
    pub doi: String,
    pub title: String,
    pub authors: String,
    pub author_corresponding: String,
    pub author_corresponding_institution: String,
    pub date: String,
    pub version: String,
    #[serde(rename = "type")]
    pub paper_type: String,
    pub license: String,
    pub category: String,
    pub jatsxml: String,
    #[serde(rename = "abstract")]
    pub abstract_text: Option<String>,
    pub funder: Option<Value>, // Can be string "NA" or object
    pub published: String,
    pub server: String,
}

#[derive(Debug, Deserialize)]
struct GetRecentArgs {
    server: String, // "biorxiv" or "medrxiv"
    count: Option<i32>,
}

#[derive(Debug, Deserialize)]
struct GetByDateArgs {
    server: String,
    start_date: String, // YYYY-MM-DD
    end_date: String,   // YYYY-MM-DD
}

#[derive(Debug, Deserialize)]
struct GetByDoiArgs {
    server: String,
    doi: String,
}

pub struct BiorxivConnector {
    client: Client,
}

impl BiorxivConnector {
    pub async fn new(_auth: AuthDetails) -> Result<Self, ConnectorError> {
        Ok(Self {
            client: Client::builder()
                .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36")
                .http1_only()
                .build()
                .map_err(ConnectorError::HttpRequest)?,
        })
    }

    async fn fetch_from_api(&self, path: &str) -> Result<Vec<BiorxivPaperRaw>, ConnectorError> {
        let url = format!("https://api.biorxiv.org/details/{}", path);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(ConnectorError::HttpRequest)?;

        if !response.status().is_success() {
            return Err(ConnectorError::Other(format!(
                "bioRxiv API returned error status: {}",
                response.status()
            )));
        }

        let parsed: BiorxivResponse = response
            .json()
            .await
            .map_err(|e| ConnectorError::Other(format!("Failed to parse JSON: {}", e)))?;

        if let Some(msg) = parsed.messages.first() {
            if msg.status != "ok" {
                // Sometimes it returns 'ok' even if empty, but if status is bad, log it.
                // But typically it just returns empty collection.
            }
        }

        Ok(parsed.collection)
    }

    fn format_paper(&self, paper: &BiorxivPaperRaw) -> HashMap<String, Value> {
        let mut result = HashMap::new();
        result.insert("doi".to_string(), json!(paper.doi));
        result.insert("title".to_string(), json!(paper.title));
        result.insert("authors".to_string(), json!(paper.authors));
        result.insert("date".to_string(), json!(paper.date));
        result.insert("version".to_string(), json!(paper.version));
        result.insert("type".to_string(), json!(paper.paper_type));
        result.insert("category".to_string(), json!(paper.category));
        result.insert("server".to_string(), json!(paper.server));

        // Add abstract if present
        if let Some(ref abstract_text) = paper.abstract_text {
            result.insert("abstract".to_string(), json!(abstract_text));
        }

        // Construct useful URLs
        let abstract_url = format!(
            "https://www.{}.org/content/{}",
            paper.server.to_lowercase(),
            paper.doi
        );
        let pdf_url = format!(
            "https://www.{}.org/content/{}.full.pdf",
            paper.server.to_lowercase(),
            paper.doi
        );

        result.insert("url".to_string(), json!(abstract_url));
        result.insert("pdf_url".to_string(), json!(pdf_url));
        result.insert("published_in".to_string(), json!(paper.published));

        result
    }
}

#[async_trait]
impl Connector for BiorxivConnector {
    fn name(&self) -> &'static str {
        "biorxiv"
    }

    fn description(&self) -> &'static str {
        "Access preprints from bioRxiv and medRxiv"
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
        // Simple test: fetch 1 recent paper from biorxiv
        match self.fetch_from_api("biorxiv/recent/1").await {
            Ok(_) => Ok(()),
            Err(e) => Err(e),
        }
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
                website_url: Some("https://api.biorxiv.org".to_string()),
            },
            instructions: Some(
                "Access bioRxiv and medRxiv preprints via official API.".to_string(),
            ),
        })
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParam>,
    ) -> Result<ListToolsResult, ConnectorError> {
        let tools = vec![
            Tool {
                name: Cow::Borrowed("get_recent_preprints"),
                title: None,
                description: Some(Cow::Borrowed(
                    "Get most recent preprints from bioRxiv or medRxiv",
                )),
                input_schema: Arc::new(
                    json!({
                        "type": "object",
                        "properties": {
                            "server": {
                                "type": "string",
                                "enum": ["biorxiv", "medrxiv"],
                                "description": "The server to fetch from"
                            },
                            "count": {
                                "type": "integer",
                                "description": "Number of papers to fetch (default: 10, max: 100)"
                            }
                        },
                        "required": ["server"]
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
                name: Cow::Borrowed("get_preprints_by_date"),
                title: None,
                description: Some(Cow::Borrowed("Get preprints within a date range")),
                input_schema: Arc::new(
                    json!({
                        "type": "object",
                        "properties": {
                            "server": {
                                "type": "string",
                                "enum": ["biorxiv", "medrxiv"],
                                "description": "The server to fetch from"
                            },
                            "start_date": {
                                "type": "string",
                                "description": "Start date in YYYY-MM-DD format"
                            },
                            "end_date": {
                                "type": "string",
                                "description": "End date in YYYY-MM-DD format"
                            }
                        },
                        "required": ["server", "start_date", "end_date"]
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
                name: Cow::Borrowed("get_preprint_by_doi"),
                title: None,
                description: Some(Cow::Borrowed("Get details of a specific preprint by DOI")),
                input_schema: Arc::new(
                    json!({
                        "type": "object",
                        "properties": {
                            "server": {
                                "type": "string",
                                "enum": ["biorxiv", "medrxiv"],
                                "description": "The server to fetch from"
                            },
                            "doi": {
                                "type": "string",
                                "description": "DOI of the paper"
                            }
                        },
                        "required": ["server", "doi"]
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
            "get_recent_preprints" => {
                let args: GetRecentArgs = serde_json::from_value(
                    serde_json::to_value(request.arguments.unwrap_or_default())
                        .map_err(ConnectorError::SerdeJson)?,
                )
                .map_err(|e| ConnectorError::InvalidParams(e.to_string()))?;

                let count = args.count.unwrap_or(10).min(100) as usize;
                // API requires date range format: server/YYYY-MM-DD/YYYY-MM-DD
                // Use last 7 days to get recent papers
                let end_date = Utc::now().format("%Y-%m-%d").to_string();
                let start_date = (Utc::now() - Duration::days(7))
                    .format("%Y-%m-%d")
                    .to_string();
                let path = format!("{}/{}/{}", args.server, start_date, end_date);
                let mut papers = self.fetch_from_api(&path).await?;
                papers.truncate(count);

                let results: Vec<HashMap<String, Value>> =
                    papers.iter().map(|p| self.format_paper(p)).collect();

                let data = json!({
                    "server": args.server,
                    "count": results.len(),
                    "results": results
                });

                Ok(structured_result_with_text(
                    &data,
                    Some(serde_json::to_string(&data)?),
                )?)
            }
            "get_preprints_by_date" => {
                let args: GetByDateArgs = serde_json::from_value(
                    serde_json::to_value(request.arguments.unwrap_or_default())
                        .map_err(ConnectorError::SerdeJson)?,
                )
                .map_err(|e| ConnectorError::InvalidParams(e.to_string()))?;

                // API format: server/YYYY-MM-DD/YYYY-MM-DD
                let path = format!("{}/{}/{}", args.server, args.start_date, args.end_date);
                let papers = self.fetch_from_api(&path).await?;

                let results: Vec<HashMap<String, Value>> =
                    papers.iter().map(|p| self.format_paper(p)).collect();

                let data = json!({
                    "server": args.server,
                    "range": format!("{} to {}", args.start_date, args.end_date),
                    "count": results.len(),
                    "results": results
                });

                Ok(structured_result_with_text(
                    &data,
                    Some(serde_json::to_string(&data)?),
                )?)
            }
            "get_preprint_by_doi" => {
                let args: GetByDoiArgs = serde_json::from_value(
                    serde_json::to_value(request.arguments.unwrap_or_default())
                        .map_err(ConnectorError::SerdeJson)?,
                )
                .map_err(|e| ConnectorError::InvalidParams(e.to_string()))?;

                // API format: server/DOI
                let path = format!("{}/{}", args.server, args.doi);
                let papers = self.fetch_from_api(&path).await?;

                if papers.is_empty() {
                    return Err(ConnectorError::ResourceNotFound);
                }

                let result = self.format_paper(&papers[0]);
                Ok(structured_result_with_text(
                    &result,
                    Some(serde_json::to_string(&result)?),
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
