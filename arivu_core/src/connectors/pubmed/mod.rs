use crate::capabilities::ConnectorConfigSchema;
use crate::cpu_pool;
use crate::error::ConnectorError;
use crate::utils::structured_result_with_text;
use crate::{auth::AuthDetails, Connector};
use async_trait::async_trait;
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, ACCEPT_LANGUAGE};
use rmcp::model::*;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::borrow::Cow;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, error, info};

mod parse;
use parse::{parse_pubmed_search_document, SearchParseInput};

#[derive(Debug, Serialize, Deserialize)]
pub struct PubMedArticle {
    pub title: String,
    pub authors: String,
    pub citation: String,
    pub pmid: String,
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PubMedSearchResult {
    pub query: String,
    pub articles: Vec<PubMedArticle>,
    pub total_results: usize,
    pub page: usize,
    pub total_pages: Option<usize>,
    pub message: Option<String>,
}

impl Default for PubMedSearchResult {
    fn default() -> Self {
        Self::new()
    }
}

impl PubMedSearchResult {
    pub fn new() -> Self {
        PubMedSearchResult {
            query: String::new(),
            articles: Vec::new(),
            total_results: 0,
            page: 1,
            total_pages: None,
            message: None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PubMedSimilarArticle {
    pub title: String,
    pub authors: String,
    pub journal: String,
    pub pmid: String,
    pub publication_type: Option<String>,
}

impl Default for PubMedSimilarArticle {
    fn default() -> Self {
        Self::new()
    }
}

impl PubMedSimilarArticle {
    pub fn new() -> Self {
        PubMedSimilarArticle {
            title: String::new(),
            authors: String::new(),
            journal: String::new(),
            pmid: String::new(),
            publication_type: None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PubMedAbstract {
    pub pmid: String,
    pub title: String,
    pub authors: String,
    pub abstract_text: String,
    pub publication_date: String,
    pub journal: String,
    pub doi: Option<String>,
    pub affiliations: Vec<String>,
    pub keywords: Vec<String>,
    pub publication_type: Option<String>,
    pub similar_articles: Vec<PubMedSimilarArticle>,
    pub citation_count: Option<usize>,
}

impl Default for PubMedAbstract {
    fn default() -> Self {
        Self::new()
    }
}

impl PubMedAbstract {
    pub fn new() -> Self {
        PubMedAbstract {
            pmid: String::new(),
            title: String::new(),
            authors: String::new(),
            abstract_text: String::new(),
            publication_date: String::new(),
            journal: String::new(),
            doi: None,
            affiliations: Vec::new(),
            keywords: Vec::new(),
            publication_type: None,
            similar_articles: Vec::new(),
            citation_count: None,
        }
    }
}

#[derive(Clone)]
pub struct PubMedConnector {
    client: reqwest::Client,
    headers: HeaderMap,
}

impl PubMedConnector {
    pub async fn new() -> Result<Self, ConnectorError> {
        // Build a tuned HTTP client to avoid slow handshakes or protocol quirks
        let client = reqwest::Client::builder()
            // http/2 can occasionally stall on misconfigured servers; http1 is safer for scraping
            .http1_only()
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(20))
            .pool_max_idle_per_host(2)
            .tcp_keepalive(Some(Duration::from_secs(30)))
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/133.0.0.0 Safari/537.36")
            .build()
            .map_err(|e| ConnectorError::Other(format!("failed to build http client: {}", e)))?;

        let mut headers = HeaderMap::new();
        headers.insert(
            ACCEPT,
            HeaderValue::from_static(
                "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
            ),
        );
        headers.insert(ACCEPT_LANGUAGE, HeaderValue::from_static("en-US,en;q=0.9"));

        let connector = PubMedConnector { client, headers };

        Ok(connector)
    }

    async fn search_pubmed(
        &self,
        query: &str,
        page: usize,
        limit: usize,
        date_range: Option<(u32, u32)>,
    ) -> Result<PubMedSearchResult, ConnectorError> {
        // URL encode the query
        let encoded_query = query.replace(" ", "+");

        // Build the URL with date range if provided
        let url = if let Some((start_year, end_year)) = date_range {
            format!(
                "https://pubmed.ncbi.nlm.nih.gov/?term={}+{}%3A{}%5Bdp%5D&page={}",
                encoded_query, start_year, end_year, page
            )
        } else {
            format!(
                "https://pubmed.ncbi.nlm.nih.gov/?term={}&page={}",
                encoded_query, page
            )
        };

        // Make the HTTP request
        let t0 = std::time::Instant::now();
        let response = self
            .client
            .get(&url)
            .headers(self.headers.clone())
            .send()
            .await
            .map_err(|e| ConnectorError::Other(e.to_string()))?;
        let t1 = std::time::Instant::now();

        // Get the HTML content
        let content = response
            .text()
            .await
            .map_err(|e| ConnectorError::Other(e.to_string()))?;
        let content_len = content.len();
        let t2 = std::time::Instant::now();

        debug!(
            target: "connector.pubmed",
            url = %url,
            connect_send_ms = %((t1 - t0).as_millis()),
            read_body_ms = %((t2 - t1).as_millis()),
            total_ms = %((t2 - t0).as_millis()),
            "fetched pubmed search page"
        );
        info!(
            target: "app_lib::pubmed",
            %url,
            connect_send_ms = %((t1 - t0).as_millis()),
            read_body_ms = %((t2 - t1).as_millis()),
            total_ms = %((t2 - t0).as_millis()),
            "fetched pubmed search page"
        );
        println!(
            "[PUBMED] fetched {} connect={}ms body={}ms total={}ms",
            url,
            (t1 - t0).as_millis(),
            (t2 - t1).as_millis(),
            (t2 - t0).as_millis()
        );

        let parse_start = std::time::Instant::now();
        println!(
            "[PUBMED] cpu_pool dispatch queue={} workers={}",
            cpu_pool::queue_depth(),
            cpu_pool::worker_count()
        );
        info!(
            target: "app_lib::pubmed",
            queue_depth = cpu_pool::queue_depth(),
            workers = cpu_pool::worker_count(),
            "dispatching pubmed search parse to datasourcer cpu pool"
        );
        let query_owned = query.to_string();
        let parse_result = cpu_pool::spawn_cpu(move || {
            parse_pubmed_search_document(SearchParseInput {
                content,
                limit,
                query: query_owned,
                page,
                content_len,
            })
        })
        .await?;
        println!(
            "[PUBMED] cpu_pool complete queue={} elapsed={}ms",
            cpu_pool::queue_depth(),
            parse_start.elapsed().as_millis()
        );
        info!(
            target: "app_lib::pubmed",
            queue_depth = cpu_pool::queue_depth(),
            parse_ms = parse_start.elapsed().as_millis(),
            "pubmed search parse completed"
        );

        Ok(parse_result)
    }

    async fn get_article_abstract(&self, pmid: &str) -> Result<PubMedAbstract, ConnectorError> {
        let url = format!("https://pubmed.ncbi.nlm.nih.gov/{}/", pmid);

        // Make the HTTP request
        let t0 = std::time::Instant::now();
        let response = self
            .client
            .get(&url)
            .headers(self.headers.clone())
            .send()
            .await
            .map_err(|e| ConnectorError::Other(e.to_string()))?;
        let t1 = std::time::Instant::now();

        // Get the HTML content
        let content = response
            .text()
            .await
            .map_err(|e| ConnectorError::Other(e.to_string()))?;
        let t2 = std::time::Instant::now();

        debug!(
            target: "connector.pubmed",
            url = %url,
            connect_send_ms = %((t1 - t0).as_millis()),
            read_body_ms = %((t2 - t1).as_millis()),
            total_ms = %((t2 - t0).as_millis()),
            "fetched pubmed article page"
        );
        info!(
            target: "app_lib::pubmed",
            %url,
            connect_send_ms = %((t1 - t0).as_millis()),
            read_body_ms = %((t2 - t1).as_millis()),
            total_ms = %((t2 - t0).as_millis()),
            "fetched pubmed article page"
        );
        println!(
            "[PUBMED] fetched article {} connect={}ms body={}ms total={}ms",
            url,
            (t1 - t0).as_millis(),
            (t2 - t1).as_millis(),
            (t2 - t0).as_millis()
        );

        // Parse the HTML document
        let html = Html::parse_document(&content);

        // Define CSS selectors for the elements we want to extract
        let title_selector = Selector::parse("h1.heading-title")
            .map_err(|e| ConnectorError::Other(e.to_string()))?;
        let authors_selector = Selector::parse("div.authors-list")
            .map_err(|e| ConnectorError::Other(e.to_string()))?;
        let abstract_selector =
            Selector::parse("div#abstract").map_err(|e| ConnectorError::Other(e.to_string()))?;
        let abstract_content_selector = Selector::parse("div.abstract-content")
            .map_err(|e| ConnectorError::Other(e.to_string()))?;
        let journal_selector = Selector::parse("button.journal-actions-trigger")
            .map_err(|e| ConnectorError::Other(e.to_string()))?;
        let pub_date_selector =
            Selector::parse("span.cit").map_err(|e| ConnectorError::Other(e.to_string()))?;
        let doi_selector = Selector::parse("span.identifier.doi")
            .map_err(|e| ConnectorError::Other(e.to_string()))?;
        let affiliations_selector =
            Selector::parse("ul.item-list").map_err(|e| ConnectorError::Other(e.to_string()))?;
        let keywords_selector =
            Selector::parse("div.keywords").map_err(|e| ConnectorError::Other(e.to_string()))?;
        let publication_type_selector = Selector::parse("div.publication-type")
            .map_err(|e| ConnectorError::Other(e.to_string()))?;
        let similar_articles_selector = Selector::parse("div.similar-articles ul.articles-list li")
            .map_err(|e| ConnectorError::Other(e.to_string()))?;
        let citation_count_selector = Selector::parse("div.citedby-articles h2.title")
            .map_err(|e| ConnectorError::Other(e.to_string()))?;

        // Extract title
        let title = html
            .select(&title_selector)
            .next()
            .map(|el| el.text().collect::<Vec<_>>().join(" ").trim().to_string())
            .unwrap_or_else(|| format!("Article {}", pmid));

        // Extract authors
        let authors = html
            .select(&authors_selector)
            .next()
            .map(|el| el.text().collect::<Vec<_>>().join(" ").trim().to_string())
            .unwrap_or_default();

        // Extract abstract - try multiple selectors to handle different HTML structures
        let paragraph_selector = match Selector::parse("p") {
            Ok(selector) => selector,
            Err(e) => {
                return Err(ConnectorError::Other(format!(
                    "Failed to parse paragraph selector: {}",
                    e
                )))
            }
        };

        let abstract_text = {
            // First try the abstract-content selector
            let abstract_from_content = html.select(&abstract_content_selector).next().map(|el| {
                // Get all paragraphs in the abstract
                let paragraphs: Vec<_> = el
                    .select(&paragraph_selector)
                    .map(|p| p.text().collect::<Vec<_>>().join(" ").trim().to_string())
                    .collect();

                paragraphs.join("\n\n")
            });

            // If that fails, try the abstract selector
            let abstract_from_abstract = if abstract_from_content.is_none()
                || abstract_from_content
                    .as_ref()
                    .map_or(true, |s| s.is_empty())
            {
                html.select(&abstract_selector).next().map(|el| {
                    // Get all paragraphs in the abstract
                    let paragraphs: Vec<_> = el
                        .select(&paragraph_selector)
                        .map(|p| p.text().collect::<Vec<_>>().join(" ").trim().to_string())
                        .collect();

                    paragraphs.join("\n\n")
                })
            } else {
                None
            };

            // Try a more general approach if both previous attempts failed
            if (abstract_from_content.is_none()
                || abstract_from_content
                    .as_ref()
                    .map_or(true, |s| s.is_empty()))
                && (abstract_from_abstract.is_none()
                    || abstract_from_abstract
                        .as_ref()
                        .map_or(true, |s| s.is_empty()))
            {
                // Look for any div with "abstract" in the id or class
                let abstract_selector_general =
                    match Selector::parse("div[id*='abstract'], div[class*='abstract']") {
                        Ok(selector) => selector,
                        Err(e) => {
                            return Err(ConnectorError::Other(format!(
                                "Failed to parse general abstract selector: {}",
                                e
                            )))
                        }
                    };

                html.select(&abstract_selector_general).next().map(|el| {
                    // Get all paragraphs or text nodes
                    let text = el.text().collect::<Vec<_>>().join(" ").trim().to_string();
                    if !text.is_empty() {
                        text
                    } else {
                        // If no direct text, try to get paragraphs
                        let paragraphs: Vec<_> = el
                            .select(&paragraph_selector)
                            .map(|p| p.text().collect::<Vec<_>>().join(" ").trim().to_string())
                            .collect();

                        paragraphs.join("\n\n")
                    }
                })
            } else if abstract_from_content.is_some()
                && !abstract_from_content.as_ref().unwrap().is_empty()
            {
                abstract_from_content
            } else {
                abstract_from_abstract
            }
        }
        .unwrap_or_else(|| "Abstract not available".to_string());

        // Extract journal
        let journal = html
            .select(&journal_selector)
            .next()
            .map(|el| el.text().collect::<Vec<_>>().join(" ").trim().to_string())
            .unwrap_or_default();

        // Extract publication date
        let publication_date = html
            .select(&pub_date_selector)
            .next()
            .map(|el| el.text().collect::<Vec<_>>().join(" ").trim().to_string())
            .unwrap_or_default();

        // Extract DOI
        let doi = html.select(&doi_selector).next().and_then(|el| {
            el.text()
                .collect::<Vec<_>>()
                .join(" ")
                .trim()
                .strip_prefix("doi: ")
                .map(|s| s.to_string())
        });

        // Extract affiliations
        let li_selector = match Selector::parse("li") {
            Ok(selector) => selector,
            Err(e) => {
                return Err(ConnectorError::Other(format!(
                    "Failed to parse li selector: {}",
                    e
                )))
            }
        };

        let affiliations = html
            .select(&affiliations_selector)
            .next()
            .map(|ul| {
                ul.select(&li_selector)
                    .map(|li| li.text().collect::<Vec<_>>().join(" ").trim().to_string())
                    .collect()
            })
            .unwrap_or_default();

        // Extract keywords
        let keywords = html
            .select(&keywords_selector)
            .next()
            .map(|div| {
                let text = div.text().collect::<Vec<_>>().join(" ");
                let keywords_text = text.trim().replace("Keywords:", "").trim().to_string();
                keywords_text
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect()
            })
            .unwrap_or_default();

        // Extract publication type
        let publication_type = html
            .select(&publication_type_selector)
            .next()
            .map(|el| el.text().collect::<Vec<_>>().join(" ").trim().to_string());

        // Extract similar articles
        let title_selector = match Selector::parse("a.docsum-title") {
            Ok(selector) => selector,
            Err(e) => {
                return Err(ConnectorError::Other(format!(
                    "Failed to parse title selector: {}",
                    e
                )))
            }
        };
        let authors_selector = match Selector::parse("span.docsum-authors") {
            Ok(selector) => selector,
            Err(e) => {
                return Err(ConnectorError::Other(format!(
                    "Failed to parse authors selector: {}",
                    e
                )))
            }
        };
        let journal_selector = match Selector::parse("span.docsum-journal-citation") {
            Ok(selector) => selector,
            Err(e) => {
                return Err(ConnectorError::Other(format!(
                    "Failed to parse journal selector: {}",
                    e
                )))
            }
        };
        let pmid_selector = match Selector::parse("span.docsum-pmid") {
            Ok(selector) => selector,
            Err(e) => {
                return Err(ConnectorError::Other(format!(
                    "Failed to parse pmid selector: {}",
                    e
                )))
            }
        };
        let pub_type_selector = match Selector::parse("span.publication-type") {
            Ok(selector) => selector,
            Err(e) => {
                return Err(ConnectorError::Other(format!(
                    "Failed to parse publication type selector: {}",
                    e
                )))
            }
        };

        let similar_articles = html
            .select(&similar_articles_selector)
            .map(|li| {
                let title_el = li.select(&title_selector).next();
                let title = title_el
                    .map(|el| el.text().collect::<Vec<_>>().join(" ").trim().to_string())
                    .unwrap_or_default();

                let authors = li
                    .select(&authors_selector)
                    .next()
                    .map(|el| el.text().collect::<Vec<_>>().join(" ").trim().to_string())
                    .unwrap_or_default();

                let journal = li
                    .select(&journal_selector)
                    .next()
                    .map(|el| el.text().collect::<Vec<_>>().join(" ").trim().to_string())
                    .unwrap_or_default();

                let pmid = li
                    .select(&pmid_selector)
                    .next()
                    .map(|el| el.text().collect::<Vec<_>>().join(" ").trim().to_string())
                    .unwrap_or_default();

                let publication_type = li
                    .select(&pub_type_selector)
                    .next()
                    .map(|el| el.text().collect::<Vec<_>>().join(" ").trim().to_string());

                PubMedSimilarArticle {
                    title,
                    authors,
                    journal,
                    pmid,
                    publication_type,
                }
            })
            .take(5) // Limit to 5 similar articles
            .collect();

        // Extract citation count (if available)
        let citation_count = html.select(&citation_count_selector).next().and_then(|el| {
            let text = el.text().collect::<Vec<_>>().join(" ");
            if text.contains("Cited by") {
                text.split_whitespace()
                    .find(|s| s.parse::<usize>().is_ok())
                    .and_then(|s| s.parse::<usize>().ok())
            } else {
                None
            }
        });

        Ok(PubMedAbstract {
            pmid: pmid.to_string(),
            title,
            authors,
            abstract_text,
            publication_date,
            journal,
            doi,
            affiliations,
            keywords,
            publication_type,
            similar_articles,
            citation_count,
        })
    }
}

#[async_trait]
impl Connector for PubMedConnector {
    fn name(&self) -> &'static str {
        "pubmed"
    }

    fn description(&self) -> &'static str {
        "A connector for searching and retrieving articles from PubMed, the open-access database of scholarly research articles in the biomedical and life sciences."
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
        // PubMed doesn't require authentication for basic searches
        Ok(())
    }

    async fn test_auth(&self) -> Result<(), ConnectorError> {
        // Test a simple search to verify connectivity
        let _result = self.search_pubmed("test", 1, 1, None).await?;
        Ok(())
    }

    fn config_schema(&self) -> ConnectorConfigSchema {
        // PubMed doesn't require any configuration for basic usage
        ConnectorConfigSchema { fields: vec![] }
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
                "PubMed connector. Effective query tips:\n\
- Begin with a few essential concepts and add more only if the results are too broad.\n\
- Group synonyms inside parentheses with OR, then connect distinct ideas with uppercase AND to control the logic.\n\
- Use quotation marks only when an exact phrase is critical; quoting or truncating turns off Automatic Term Mapping, so compare results with and without those limits.\n\
- Scan an early relevant record to capture MeSH headings and combine those controlled terms with your free-text keywords.\n\
- Apply filters (date, article type, language) after reviewing the initial set—filters persist until you clear them.\n\
- For proximity, use the Title/Abstract proximity syntax (e.g., \"term1 term2\"[tiab:~2]) to keep related words near each other.\n\
- When you see few or no results, remove field tags or exclusions, broaden terminology, or drop the narrowest concept before re-running.".to_string(),
            ),
        })
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParam>,
    ) -> Result<ListResourcesResult, ConnectorError> {
        let resources = vec![Resource {
            raw: RawResource {
                uri: "pubmed://article/{pmid}".to_string(),
                name: "PubMed Article".to_string(),
                title: None,
                description: Some("A scientific article from PubMed".to_string()),
                mime_type: Some("application/vnd.pubmed.article+json".to_string()),
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

        if uri_str.starts_with("pubmed://article/") {
            let parts: Vec<&str> = uri_str.split('/').collect();
            if parts.len() < 4 {
                return Err(ConnectorError::InvalidInput(format!(
                    "Invalid resource URI: {}",
                    uri_str
                )));
            }
            let pmid = parts[3];

            let article = self.get_article_abstract(pmid).await?;

            let content_text = serde_json::to_string(&article)?;
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
            tools: vec![
                Tool {
                    name: Cow::Borrowed("search"),
                    title: None,
                    description: Some(Cow::Borrowed("Search for articles in PubMed")),
                    input_schema: Arc::new(json!({
                        "type": "object",
                        "properties": {
                            "query": {
                                "type": "string",
                                "description": "Enter a handful of core concepts, join synonyms with OR inside parentheses, and connect distinct ideas with AND (uppercase). Use quotes or truncation only when necessary, since either disables PubMed's automatic MeSH mapping."
                            },
                            "page": {
                                "type": "integer",
                                "description": "Page number (default: 1)"
                            },
                            "limit": {
                                "type": "integer",
                                "description": "Maximum number of results to return (default: 10). Lower this to keep responses concise."
                            },
                            "start_year": {
                                "type": "integer",
                                "description": "Start year for publication date range filter"
                            },
                            "end_year": {
                                "type": "integer",
                                "description": "End year for publication date range filter"
                            }
                        },
                        "required": ["query"]
                    }).as_object().expect("Schema object").clone()),
                    output_schema: None,
                    annotations: None,
                    icons: None,
                },
                Tool {
                    name: Cow::Borrowed("get_abstract"),
                    title: None,
                    description: Some(Cow::Borrowed("Get the abstract and details of a PubMed article by PMID")),
                    input_schema: Arc::new(json!({
                        "type": "object",
                        "properties": {
                            "pmid": {
                                "type": "string",
                                "description": "The PubMed ID (PMID) of the article"
                            }
                        },
                        "required": ["pmid"]
                    }).as_object().expect("Schema object").clone()),
                    output_schema: None,
                    annotations: None,
                    icons: None,
                },
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
            "search" => {
                let query = args.get("query").and_then(|v| v.as_str()).ok_or(
                    ConnectorError::InvalidParams("Missing 'query' parameter".to_string()),
                )?;

                // Make all parameters optional
                let page = args.get("page").and_then(|v| v.as_u64()).unwrap_or(1) as usize;
                let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(10) as usize;

                // Handle optional date range
                let date_range =
                    if args.get("start_year").is_some() && args.get("end_year").is_some() {
                        let start_year =
                            args.get("start_year").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                        let end_year =
                            args.get("end_year").and_then(|v| v.as_u64()).unwrap_or(0) as u32;

                        if start_year > 0 && end_year > 0 {
                            Some((start_year, end_year))
                        } else {
                            None
                        }
                    } else {
                        None
                    };

                let result = self
                    .search_pubmed(query, page, limit, date_range)
                    .await
                    .unwrap_or_else(|e| {
                        error!("Error: {}", e);
                        PubMedSearchResult::new()
                    });

                let text = serde_json::to_string(&result)?;
                Ok(structured_result_with_text(&result, Some(text))?)
            }
            "get_abstract" => {
                let pmid = args.get("pmid").and_then(|v| v.as_str()).ok_or(
                    ConnectorError::InvalidParams("Missing 'pmid' parameter".to_string()),
                )?;

                let abstract_data: PubMedAbstract =
                    self.get_article_abstract(pmid).await.unwrap_or_else(|e| {
                        error!("Error: {}", e);
                        PubMedAbstract::new()
                    });

                let text = serde_json::to_string(&abstract_data)?;
                Ok(structured_result_with_text(&abstract_data, Some(text))?)
            }
            _ => Err(ConnectorError::ToolNotFound),
        }
    }

    async fn list_prompts(
        &self,
        _request: Option<PaginatedRequestParam>,
    ) -> Result<ListPromptsResult, ConnectorError> {
        Ok(ListPromptsResult {
            prompts: vec![
                Prompt {
                    name: "summarize_abstract".to_string(),
                    title: None,
                    description: Some("Summarize a PubMed article abstract".to_string()),
                    arguments: Some(vec![PromptArgument {
                        name: "pmid".to_string(),
                        title: None,
                        description: Some("The PubMed ID (PMID) of the article".to_string()),
                        required: Some(true),
                    }]),
                    icons: None,
                },
                Prompt {
                    name: "analyze_research".to_string(),
                    title: None,
                    description: Some("Analyze multiple research papers on a topic".to_string()),
                    arguments: Some(vec![
                        PromptArgument {
                            name: "query".to_string(),
                            title: None,
                            description: Some("The research topic to analyze".to_string()),
                            required: Some(true),
                        },
                        PromptArgument {
                            name: "limit".to_string(),
                            title: None,
                            description: Some(
                                "Number of papers to analyze (default: 5)".to_string(),
                            ),
                            required: Some(false),
                        },
                    ]),
                    icons: None,
                },
            ],
            next_cursor: None,
        })
    }

    async fn get_prompt(&self, name: &str) -> Result<Prompt, ConnectorError> {
        match name {
            "summarize_abstract" => Ok(Prompt {
                name: "summarize_abstract".to_string(),
                title: None,
                description: Some("Summarize the key findings and conclusions from this PubMed abstract in a concise manner.".to_string()),
                arguments: Some(vec![
                    PromptArgument {
                        name: "pmid".to_string(),
                        title: None,
                        description: Some("The PubMed ID (PMID) of the article".to_string()),
                        required: Some(true),
                    },
                ]),
                icons: None,
            }),
            "analyze_research" => Ok(Prompt {
                name: "analyze_research".to_string(),
                title: None,
                description: Some("Analyze the following research papers on the topic. Identify common themes, contradictions, and gaps in the research. Summarize the current state of knowledge and suggest directions for future research.".to_string()),
                arguments: Some(vec![
                    PromptArgument {
                        name: "query".to_string(),
                        title: None,
                        description: Some("The research topic to analyze".to_string()),
                        required: Some(true),
                    },
                    PromptArgument {
                        name: "limit".to_string(),
                        title: None,
                        description: Some("Number of papers to analyze (default: 5)".to_string()),
                        required: Some(false),
                    },
                ]),
                icons: None,
            }),
            _ => Err(ConnectorError::InvalidParams(format!("Prompt with name {} not found", name))),
        }
    }
}
