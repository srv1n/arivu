use crate::capabilities::{ConnectorConfigSchema, Field, FieldType};
use crate::error::ConnectorError;
use crate::utils::structured_result_with_text;
use crate::{auth::AuthDetails, Connector};
use async_trait::async_trait;
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};
use rmcp::model::*;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::borrow::Cow;
use std::sync::Arc;

#[derive(Debug, Serialize, Deserialize)]
pub struct SciHubResult {
    pub doi: String,
    pub pdf_url: Option<String>,
    pub title: Option<String>,
    pub authors: Option<String>,
    pub journal: Option<String>,
    pub year: Option<String>,
    pub success: bool,
    pub message: String,
}

pub struct SciHubConnector {
    client: reqwest::Client,
    headers: HeaderMap,
    base_url: String,
}

impl SciHubConnector {
    pub async fn new(auth: AuthDetails) -> Result<Self, ConnectorError> {
        let mut connector = SciHubConnector {
            client: reqwest::Client::new(),
            headers: HeaderMap::new(),
            base_url: "https://sci-hub.se".to_string(),
        };

        // Set default user agent
        connector.headers.insert(
            USER_AGENT,
            HeaderValue::from_static(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/117.0.0.0 Safari/537.36",
            ),
        );

        connector.set_auth_details(auth).await?;
        Ok(connector)
    }

    async fn search_scihub(&self, doi: &str) -> Result<SciHubResult, ConnectorError> {
        // Construct the URL
        let url = format!("{}/{}", self.base_url, doi);

        // Make the HTTP request
        let response = self
            .client
            .get(&url)
            .headers(self.headers.clone())
            .send()
            .await
            .map_err(|e| ConnectorError::Other(e.to_string()))?;

        // Check if the request was successful
        if !response.status().is_success() {
            return Ok(SciHubResult {
                doi: doi.to_string(),
                pdf_url: None,
                title: None,
                authors: None,
                journal: None,
                year: None,
                success: false,
                message: format!(
                    "Failed to retrieve paper: HTTP status {}",
                    response.status()
                ),
            });
        }

        // Get the HTML content
        let content = response
            .text()
            .await
            .map_err(|e| ConnectorError::Other(e.to_string()))?;

        // Parse the HTML document
        let html = Html::parse_document(&content);

        // Define CSS selectors for the elements we want to extract
        let embed_selector = Selector::parse("embed[type='application/pdf']")
            .map_err(|e| ConnectorError::Other(e.to_string()))?;
        let citation_selector =
            Selector::parse("div#citation").map_err(|e| ConnectorError::Other(e.to_string()))?;

        // Extract PDF URL
        let pdf_url = html
            .select(&embed_selector)
            .next()
            .and_then(|el| el.value().attr("src"))
            .map(|src| {
                if src.starts_with("//") {
                    format!("https:{}", src)
                } else if src.starts_with("/") {
                    format!("{}{}", self.base_url, src)
                } else {
                    src.to_string()
                }
            });

        // Extract citation information
        let citation_text = html
            .select(&citation_selector)
            .next()
            .map(|el| el.text().collect::<Vec<_>>().join(" ").trim().to_string())
            .unwrap_or_default();

        // Parse citation text to extract title, authors, journal, and year
        let (title, authors, journal, year) = self.parse_citation(&citation_text);

        // Determine success and message
        let success = pdf_url.is_some();
        let message = if success {
            "Successfully found PDF".to_string()
        } else {
            "No PDF found for this DOI".to_string()
        };

        Ok(SciHubResult {
            doi: doi.to_string(),
            pdf_url,
            title,
            authors,
            journal,
            year,
            success,
            message,
        })
    }

    fn parse_citation(
        &self,
        citation: &str,
    ) -> (
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
    ) {
        // This is a simple parser for citation text
        // In a real implementation, you might want to use a more sophisticated approach

        if citation.is_empty() {
            return (None, None, None, None);
        }

        // Try to extract information from citation text
        // Example format: "Author, A. (Year). Title. Journal, Volume(Issue), Pages."

        // Extract year
        let year = citation
            .split('(')
            .nth(1)
            .and_then(|s| s.split(')').next())
            .map(|s| s.trim().to_string());

        // Extract title - assuming it's between the year and the journal
        let title = citation
            .split(')')
            .nth(1)
            .and_then(|s| s.split('.').next())
            .map(|s| s.trim().to_string());

        // Extract authors - assuming they're before the year
        let authors = citation.split('(').next().map(|s| s.trim().to_string());

        // Extract journal - assuming it's after the title
        let journal = citation
            .split('.')
            .nth(1)
            .and_then(|s| s.split(',').next())
            .map(|s| s.trim().to_string());

        (title, authors, journal, year)
    }
}

#[async_trait]
impl Connector for SciHubConnector {
    fn name(&self) -> &'static str {
        "scihub"
    }

    fn description(&self) -> &'static str {
        "A connector for retrieving scientific papers from Sci-Hub using DOIs"
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

    async fn set_auth_details(&mut self, details: AuthDetails) -> Result<(), ConnectorError> {
        // Check if a custom base URL is provided
        if let Some(base_url) = details.get("base_url") {
            self.base_url = base_url.to_string();
        }

        Ok(())
    }

    async fn test_auth(&self) -> Result<(), ConnectorError> {
        // Test a simple search to verify connectivity
        let _result = self
            .search_scihub("10.1046/j.1365-2125.2003.02007.x")
            .await?;
        Ok(())
    }

    fn config_schema(&self) -> ConnectorConfigSchema {
        ConnectorConfigSchema {
            fields: vec![Field {
                name: "base_url".to_string(),
                label: "Sci-Hub Base URL".to_string(),
                field_type: FieldType::Text,
                required: false,
                description: Some(
                    "The base URL for Sci-Hub (default: https://sci-hub.se)".to_string(),
                ),
                options: None,
            }],
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
            instructions: Some(
                "Sci-Hub connector. Supply the article DOI whenever you can—it is the most precise lookup key. If you do not have the DOI, use a metadata search (e.g., Crossref with the title in quotes) to retrieve it, then retry. When you must search by title alone, include distinctive author or year terms and confirm the returned title, authors, journal, and year before using the file.".to_string(),
            ),
        })
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParam>,
    ) -> Result<ListResourcesResult, ConnectorError> {
        let resources = vec![Resource {
            raw: RawResource {
                uri: "scihub://paper/{doi}".to_string(),
                name: "Scientific Paper".to_string(),
                title: None,
                description: Some("A scientific paper from Sci-Hub".to_string()),
                mime_type: Some("application/pdf".to_string()),
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

        if uri_str.starts_with("scihub://paper/") {
            let parts: Vec<&str> = uri_str.split('/').collect();
            if parts.len() < 4 {
                return Err(ConnectorError::InvalidInput(format!(
                    "Invalid resource URI: {}",
                    uri_str
                )));
            }
            let doi = parts[3];

            let result = self.search_scihub(doi).await?;

            if !result.success {
                return Err(ConnectorError::ResourceNotFound);
            }

            let content_text = serde_json::to_string(&result)?;
            Ok(vec![ResourceContents::text(content_text, uri_str)])
        } else {
            Err(ConnectorError::ResourceNotFound)
        }
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParam>,
    ) -> Result<ListToolsResult, ConnectorError> {
        Ok(ListToolsResult {
            tools: vec![Tool {
                name: Cow::Borrowed("get_paper"),
                title: None,
                description: Some(Cow::Borrowed("Paper by DOI from Sci-Hub.")),
                input_schema: Arc::new(
                    json!({
                        "type": "object",
                        "properties": {
                            "doi": {
                                "type": "string",
                                "description": "The DOI (Digital Object Identifier) of the paper"
                            }
                        },
                        "required": ["doi"]
                    })
                    .as_object()
                    .expect("Schema object")
                    .clone(),
                ),
                output_schema: None,
                annotations: None,
                icons: None,
            }],
            next_cursor: None,
        })
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParam,
    ) -> Result<CallToolResult, ConnectorError> {
        let args = request.arguments.unwrap_or_default();

        match request.name.as_ref() {
            "get_paper" => {
                let doi = args.get("doi").and_then(|v| v.as_str()).ok_or(
                    ConnectorError::InvalidParams("Missing 'doi' parameter".to_string()),
                )?;

                let result = self.search_scihub(doi).await?;
                let text = serde_json::to_string(&result)?;
                Ok(structured_result_with_text(&result, Some(text))?)
            }
            _ => Err(ConnectorError::ToolNotFound),
        }
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
            "Prompt with name {} not found",
            name
        )))
    }
}
