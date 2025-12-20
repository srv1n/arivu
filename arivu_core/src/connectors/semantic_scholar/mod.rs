use crate::capabilities::{ConnectorConfigSchema, Field, FieldType};
use crate::error::ConnectorError;
use crate::utils::structured_result_with_text;
use crate::{auth::AuthDetails, Connector};
use async_trait::async_trait;
use reqwest::StatusCode;
use rmcp::model::*;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::Arc;

// Define the structs for deserializing the API responses
#[derive(Debug, Deserialize, Serialize)]
struct PaperSearchResponse {
    data: Vec<Paper>,
    next: Option<String>,
    offset: Option<i32>,
    total: Option<i32>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct Paper {
    pub paper_id: String,
    pub external_ids: Option<ExternalIds>,
    pub url: Option<String>,
    pub title: Option<String>,
    #[serde(rename = "abstract")]
    pub abstract_field: Option<String>,
    pub venue: Option<String>,
    pub year: Option<i64>,
    pub citation_count: Option<i64>,
    pub influential_citation_count: Option<i64>,
    pub open_access_pdf: Option<OpenAccessPdf>,
    #[serde(default)]
    pub fields_of_study: Option<Vec<String>>,
    #[serde(default)]
    pub publication_types: Option<Vec<String>>,
    pub publication_date: Option<String>,
    pub authors: Option<Vec<Author>>,
}
#[derive(Debug, Deserialize, Serialize)]
struct Author {
    #[serde(rename = "authorId")]
    author_id: Option<String>,
    name: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct OpenAccessPdf {
    url: Option<String>,
    status: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalIds {
    #[serde(rename = "MAG")]
    pub mag: Option<String>,
    #[serde(rename = "DOI")]
    pub doi: Option<String>,
    #[serde(rename = "CorpusId")]
    pub corpus_id: i64,
    #[serde(rename = "DBLP")]
    pub dblp: Option<String>,
    #[serde(rename = "ArXiv")]
    pub ar_xiv: Option<String>,
    #[serde(rename = "PubMed")]
    pub pub_med: Option<String>,
    #[serde(rename = "PubMedCentral")]
    pub pub_med_central: Option<String>,
    #[serde(rename = "ACL")]
    pub acl: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct RecommendationsResponse {
    #[serde(rename = "recommendedPapers")]
    recommended_papers: Vec<Paper>,
}

// Define the structs for search arguments
#[derive(Debug, Deserialize)]
struct SearchPapersArgs {
    query: String,
    #[serde(default = "default_page_size")]
    page_size: i32,
    #[serde(default = "default_page")]
    page: i32,
    #[serde(default = "default_sort")]
    sort: String,
    #[serde(default = "default_year_filter")]
    year: Option<String>,
    #[serde(default = "default_fields_of_study")]
    fields_of_study: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct GetPaperDetailsArgs {
    paper_id: String,
}

#[derive(Debug, Deserialize)]
struct GetRelatedPapersArgs {
    paper_id: String,
    #[serde(default = "default_page_size")]
    limit: i32,
}

fn default_page_size() -> i32 {
    10
}

fn default_page() -> i32 {
    1
}

fn default_sort() -> String {
    "relevance".to_string()
}

fn default_year_filter() -> Option<String> {
    None
}

fn default_fields_of_study() -> Option<Vec<String>> {
    None
}

// Define the Semantic Scholar connector
pub struct SemanticScholarConnector {
    client: reqwest::Client,
    api_key: Option<String>,
}

impl SemanticScholarConnector {
    pub async fn new(auth: AuthDetails) -> Result<Self, ConnectorError> {
        let client = reqwest::Client::builder()
            .user_agent("rzn_datasourcer/0.1.0")
            .build()
            .map_err(|e| ConnectorError::Other(e.to_string()))?;

        let api_key = auth.get("api_key").map(|v| v.to_string());

        Ok(SemanticScholarConnector { client, api_key })
    }

    async fn search_papers(
        &self,
        args: &SearchPapersArgs,
    ) -> Result<PaperSearchResponse, ConnectorError> {
        // Use the Academic Graph API for paper search
        let mut url = format!(
            "https://api.semanticscholar.org/graph/v1/paper/search/bulk?query={}&limit={}&offset={}",
            urlencoding::encode(&args.query),
            args.page_size,
            (args.page - 1) * args.page_size
        );

        // Add fields parameter to get comprehensive paper details
        url.push_str("&fields=paperId,title,abstract,url,venue,year,publicationDate,publicationTypes,authors,citationCount,influentialCitationCount,openAccessPdf,fieldsOfStudy,externalIds");

        // Add sort parameter if not default
        if args.sort != "relevance" {
            url.push_str(&format!("&sort={}", args.sort));
        }

        // Add year filter if provided
        if let Some(year) = &args.year {
            url.push_str(&format!("&year={}", year));
        }

        // Add fields of study filter if provided
        if let Some(fields) = &args.fields_of_study {
            for field in fields {
                url.push_str(&format!("&fieldsOfStudy={}", urlencoding::encode(field)));
            }
        }

        let mut request = self.client.get(&url);

        // Add API key if available
        if let Some(api_key) = &self.api_key {
            request = request.header("x-api-key", api_key);
        }

        let response = request.send().await.map_err(ConnectorError::HttpRequest)?;

        if response.status() == StatusCode::NOT_FOUND {
            return Err(ConnectorError::ResourceNotFound);
        }

        if response.status() == StatusCode::NOT_FOUND {
            return Err(ConnectorError::ResourceNotFound);
        }

        if !response.status().is_success() {
            return Err(ConnectorError::Other(format!(
                "Semantic Scholar API returned error status: {}",
                response.status()
            )));
        }

        let response_text = response
            .text()
            .await
            .map_err(|e| ConnectorError::Other(format!("Failed to get response {}", e)))?;

        // println!("Response {}", response_text);

        let search_response: PaperSearchResponse = serde_json::from_str(&response_text)
            .map_err(|e| ConnectorError::Other(format!("Failed to parse JSON response: {}", e)))?;
        // let search_response = PaperSearchResponse {
        //     data: vec![],
        //     next: None,
        //     offset: None,
        //     total: None,
        // };

        Ok(search_response)
    }

    async fn get_paper_details(&self, paper_id: &str) -> Result<Paper, ConnectorError> {
        let url = format!(
            //
            // "https://api.semanticscholar.org/graph/v1/paper/{}?fields=paperId,title,abstract,url,venue,year,publicationDate,publicationTypes,authors,citationCount,influentialCitationCount,openAccessPdf,fieldsOfStudy,externalIds",
            "https://api.semanticscholar.org/graph/v1/paper/{}?fields=paperId,title,abstract,url,venue,year,openAccessPdf,fieldsOfStudy,externalIds",
            paper_id
        );

        let mut request = self.client.get(&url);

        // Add API key if available
        if let Some(api_key) = &self.api_key {
            request = request.header("x-api-key", api_key);
        }

        let response = request.send().await.map_err(ConnectorError::HttpRequest)?;

        if !response.status().is_success() {
            return Err(ConnectorError::Other(format!(
                "Semantic Scholar API returned error status: {}",
                response.status()
            )));
        }

        let paper: Paper = response
            .json()
            .await
            .map_err(|e| ConnectorError::Other(format!("Failed to parse JSON response: {}", e)))?;

        Ok(paper)
    }

    async fn get_related_papers(
        &self,
        paper_id: &str,
        limit: i32,
    ) -> Result<RecommendationsResponse, ConnectorError> {
        let url = format!(
            "https://api.semanticscholar.org/recommendations/v1/papers/forpaper/{}?fields=paperId,title,abstract,url,venue,year,publicationDate,publicationTypes,authors,citationCount,influentialCitationCount,openAccessPdf,fieldsOfStudy,externalIds&limit={}",
            paper_id, limit
        );

        let mut request = self.client.get(&url);

        // Add API key if available
        if let Some(api_key) = &self.api_key {
            request = request.header("x-api-key", api_key);
        }

        let response = request.send().await.map_err(ConnectorError::HttpRequest)?;

        if !response.status().is_success() {
            return Err(ConnectorError::Other(format!(
                "Semantic Scholar API returned error status: {}",
                response.status()
            )));
        }

        let recommendations: RecommendationsResponse = response
            .json()
            .await
            .map_err(|e| ConnectorError::Other(format!("Failed to parse JSON response: {}", e)))?;

        Ok(recommendations)
    }

    fn format_paper(&self, paper: &Paper) -> HashMap<String, Value> {
        let mut result = HashMap::new();

        result.insert("id".to_string(), json!(paper.paper_id));
        result.insert("title".to_string(), json!(paper.title));

        // Handle abstract which might be in different fields
        let abstract_text = paper
            .abstract_field
            .clone()
            .or_else(|| paper.abstract_field.clone())
            .unwrap_or_default();
        result.insert("abstract".to_string(), json!(abstract_text));
        result.insert("content".to_string(), json!(abstract_text));

        result.insert("url".to_string(), json!(paper.url));

        if let Some(venue) = &paper.venue {
            result.insert("venue".to_string(), json!(venue));
            result.insert("journal".to_string(), json!(venue));
        }

        if let Some(year) = paper.year {
            result.insert("year".to_string(), json!(year));
        }

        if let Some(date) = &paper.publication_date {
            result.insert("publication_date".to_string(), json!(date));
        }

        if let Some(types) = &paper.publication_types {
            result.insert("publication_types".to_string(), json!(types));
        }

        if let Some(authors) = &paper.authors {
            let author_names: Vec<String> = authors
                .iter()
                .map(|author| author.name.clone().unwrap_or_default())
                .collect();
            result.insert("authors".to_string(), json!(author_names));
        }

        if let Some(count) = paper.citation_count {
            result.insert("citation_count".to_string(), json!(count));
        }

        if let Some(count) = paper.influential_citation_count {
            result.insert("influential_citation_count".to_string(), json!(count));
        }

        if let Some(pdf) = &paper.open_access_pdf {
            result.insert("pdf_url".to_string(), json!(pdf.url));
        }

        if let Some(fields) = &paper.fields_of_study {
            result.insert("fields_of_study".to_string(), json!(fields));
            result.insert("tags".to_string(), json!(fields));
        }

        if let Some(ids) = &paper.external_ids {
            if let Some(doi) = &ids.doi {
                result.insert("doi".to_string(), json!(doi));
            }

            if let Some(arxiv) = &ids.ar_xiv {
                result.insert("arxiv_id".to_string(), json!(arxiv));
            }
        }

        result
    }
}

#[async_trait]
impl Connector for SemanticScholarConnector {
    fn name(&self) -> &'static str {
        "semantic_scholar"
    }

    fn description(&self) -> &'static str {
        "A connector for searching academic papers on Semantic Scholar."
    }

    async fn capabilities(&self) -> ServerCapabilities {
        ServerCapabilities {
            tools: None,
            ..Default::default()
        }
    }

    async fn get_auth_details(&self) -> Result<AuthDetails, ConnectorError> {
        let mut auth = AuthDetails::new();
        if let Some(api_key) = &self.api_key {
            auth.insert("api_key".to_string(), api_key.clone());
        }
        Ok(auth)
    }

    async fn set_auth_details(&mut self, details: AuthDetails) -> Result<(), ConnectorError> {
        self.api_key = details.get("api_key").map(|v| v.to_string());
        Ok(())
    }

    async fn test_auth(&self) -> Result<(), ConnectorError> {
        // Simple test to check if the API is accessible
        let args = SearchPapersArgs {
            query: "artificial intelligence".to_string(),
            page_size: 1,
            page: 1,
            sort: "relevance".to_string(),
            year: None,
            fields_of_study: None,
        };

        let test_response = self.search_papers(&args).await?;
        if test_response.data.is_empty() {
            return Err(ConnectorError::Other(
                "Failed to get test results from Semantic Scholar API".to_string(),
            ));
        }
        Ok(())
    }

    fn config_schema(&self) -> ConnectorConfigSchema {
        ConnectorConfigSchema {
            fields: vec![
                Field {
                    name: "api_key".to_string(),
                    description: Some("API key for Semantic Scholar (optional but recommended for higher rate limits)".to_string()),
                    field_type: FieldType::Secret,
                    required: false,
                    options: None,
                    label: "API Key".to_string(),
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
                uri: "semanticscholar://paper/{paper_id}".to_string(),
                name: "Academic Paper".to_string(),
                title: None,
                description: Some("Represents an academic paper on Semantic Scholar.".to_string()),
                mime_type: Some("application/vnd.semanticscholar.paper+json".to_string()),
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

        if uri_str.starts_with("semanticscholar://paper/") {
            let parts: Vec<&str> = uri_str.split('/').collect();
            if parts.len() < 4 {
                return Err(ConnectorError::InvalidInput(format!(
                    "Invalid resource URI: {}",
                    uri_str
                )));
            }
            let paper_id = parts[3];

            let paper = self.get_paper_details(paper_id).await?;
            let _paper_data = self.format_paper(&paper);

            let content_text = serde_json::to_string(&paper)?;
            Ok(vec![ResourceContents::text(content_text, uri_str)])
        } else {
            Err(ConnectorError::ResourceNotFound)
        }
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParam>,
    ) -> Result<ListToolsResult, ConnectorError> {
        let tools = vec![
            Tool {
                name: Cow::Borrowed("search_papers"),
                title: None,
                description: Some(Cow::Borrowed("Search papers by query.")),
                input_schema: Arc::new(json!({
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "The search query."
                        },
                        "page_size": {
                            "type": "integer",
                            "description": "Number of results per page (default: 10)."
                        },
                        "page": {
                            "type": "integer",
                            "description": "Page number (default: 1)."
                        },
                        "sort": {
                            "type": "string",
                            "description": "Sort order (default: 'relevance').",
                            "enum": ["relevance", "citationCount", "publicationDate"]
                        },
                        "year": {
                            "type": "string",
                            "description": "Filter by year range (e.g., '2020-2023' or '2020-')."
                        },
                        "fields_of_study": {
                            "type": "array",
                            "items": {
                                "type": "string"
                            },
                            "description": "Filter by fields of study."
                        }
                    },
                    "required": ["query"]
                }).as_object().expect("Schema object").clone()),
                output_schema: None,
                annotations: None,
                icons: None,
            },
            Tool {
                name: Cow::Borrowed("get_paper_details"),
                title: None,
                description: Some(Cow::Borrowed("Paper details by paper_id.")),
                input_schema: Arc::new(json!({
                    "type": "object",
                    "properties": {
                        "paper_id": {
                            "type": "string",
                            "description": "The ID of the paper to retrieve."
                        }
                    },
                    "required": ["paper_id"]
                }).as_object().expect("Schema object").clone()),
                output_schema: None,
                annotations: None,
                icons: None,
            },
            Tool {
                name: Cow::Borrowed("get_related_papers"),
                title: None,
                description: Some(Cow::Borrowed("Related papers by paper_id.")),
                input_schema: Arc::new(json!({
                    "type": "object",
                    "properties": {
                        "paper_id": {
                            "type": "string",
                            "description": "The ID of the paper to find related papers for."
                        },
                        "limit": {
                            "type": "integer",
                            "description": "Maximum number of related papers to return (default: 10)."
                        }
                    },
                    "required": ["paper_id"]
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
            "search_papers" => {
                let args: SearchPapersArgs = serde_json::from_value(json!(args)).map_err(|e| {
                    ConnectorError::InvalidParams(format!("Invalid arguments: {}", e))
                })?;

                let search_response = self.search_papers(&args).await?;
                tracing::debug!(?search_response, "Semantic Scholar search response");

                let papers: Vec<HashMap<String, Value>> = search_response
                    .data
                    .iter()
                    .map(|paper| self.format_paper(paper))
                    .collect();

                let text = serde_json::to_string(&papers)?;
                Ok(structured_result_with_text(&papers, Some(text))?)
            }
            "get_paper_details" => {
                let args: GetPaperDetailsArgs =
                    serde_json::from_value(json!(args)).map_err(|e| {
                        ConnectorError::InvalidParams(format!("Invalid arguments: {}", e))
                    })?;

                match self.get_paper_details(&args.paper_id).await {
                    Ok(paper) => {
                        let paper_data = self.format_paper(&paper);

                        let text = serde_json::to_string(&paper_data)?;
                        Ok(structured_result_with_text(&paper_data, Some(text))?)
                    }
                    Err(ConnectorError::ResourceNotFound) => {
                        let payload = json!({
                            "requested_id": args.paper_id,
                            "papers": [],
                        });
                        let text = serde_json::to_string(&payload)?;
                        Ok(structured_result_with_text(&payload, Some(text))?)
                    }
                    Err(err) => Err(err),
                }
            }
            "get_related_papers" => {
                let args: GetRelatedPapersArgs =
                    serde_json::from_value(json!(args)).map_err(|e| {
                        ConnectorError::InvalidParams(format!("Invalid arguments: {}", e))
                    })?;

                match self.get_related_papers(&args.paper_id, args.limit).await {
                    Ok(recommendations) => {
                        let papers: Vec<HashMap<String, Value>> = recommendations
                            .recommended_papers
                            .iter()
                            .map(|paper| self.format_paper(paper))
                            .collect();

                        let text = serde_json::to_string(&papers)?;
                        Ok(structured_result_with_text(&papers, Some(text))?)
                    }
                    Err(ConnectorError::ResourceNotFound) => {
                        let payload = json!({
                            "requested_id": args.paper_id,
                            "papers": [],
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
            name: "summarize_paper".to_string(),
            title: None,
            description: Some("Summarizes an academic paper.".to_string()),
            arguments: Some(vec![PromptArgument {
                name: "paper_id".to_string(),
                title: None,
                description: Some("The ID of the paper to summarize.".to_string()),
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
            "summarize_paper" => Ok(Prompt {
                name: "summarize_paper".to_string(),
                title: None,
                description: Some("Summarizes an academic paper.".to_string()),
                arguments: Some(vec![PromptArgument {
                    name: "paper_id".to_string(),
                    title: None,
                    description: Some("The ID of the paper to summarize.".to_string()),
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
