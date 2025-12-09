use async_trait::async_trait;
use reqwest;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use crate::auth::AuthDetails;
use crate::capabilities::ConnectorConfigSchema;
use crate::error::ConnectorError;
use crate::utils::structured_result_with_text;
use crate::Connector;
use rmcp::model::*;
use urlencoding;

// Import the types module
mod types;
pub use types::{AlgoliaHit, HackerNewsItem, ItemType, SimpleItem};

const DEFAULT_STORY_FIELDS: &[&str] = &["title", "text"];
const DEFAULT_COMMENT_FIELDS: &[&str] = &["text"];

const STORY_FIELD_ORDER: &[&str] = &[
    "id",
    "title",
    "text",
    "url",
    "author",
    "created_at",
    "created_at_i",
    "type",
    "points",
    "parent_id",
    "story_id",
    "options",
];

const COMMENT_FIELD_ORDER: &[&str] = &[
    "id",
    "text",
    "author",
    "created_at",
    "created_at_i",
    "parent_id",
    "story_id",
    "points",
];

fn parse_field_sets(args: &serde_json::Map<String, Value>) -> (HashSet<String>, HashSet<String>) {
    (
        parse_field_list(args.get("storyFields"), DEFAULT_STORY_FIELDS),
        parse_field_list(args.get("commentFields"), DEFAULT_COMMENT_FIELDS),
    )
}

fn parse_field_list(value: Option<&Value>, defaults: &[&str]) -> HashSet<String> {
    let mut fields: HashSet<String> = defaults.iter().map(|s| s.to_string()).collect();

    if let Some(raw) = value {
        match raw {
            Value::Array(items) => {
                for item in items {
                    if let Some(text) = item.as_str() {
                        update_field_set(&mut fields, text);
                    }
                }
            }
            Value::String(text) => {
                for part in text.split(',') {
                    let trimmed = part.trim();
                    if !trimmed.is_empty() {
                        update_field_set(&mut fields, trimmed);
                    }
                }
            }
            _ => {}
        }
    }

    if fields.is_empty() {
        for default in defaults {
            fields.insert((*default).to_string());
        }
    }

    fields
}

fn update_field_set(fields: &mut HashSet<String>, raw: &str) {
    if raw.is_empty() {
        return;
    }

    let normalized = raw.trim();
    if normalized.starts_with('-') || normalized.starts_with('!') {
        let key = normalized
            .trim_start_matches('-')
            .trim_start_matches('!')
            .trim();
        if !key.is_empty() {
            fields.remove(key);
        }
    } else {
        fields.insert(normalized.to_string());
    }
}

fn story_item_to_payload(
    item: &HackerNewsItem,
    story_fields: &HashSet<String>,
    comment_fields: &HashSet<String>,
) -> Value {
    let mut map = serde_json::Map::new();

    for field in STORY_FIELD_ORDER {
        if !story_fields.contains(*field) {
            continue;
        }

        match *field {
            "id" => {
                if let Some(id) = item.id {
                    map.insert("id".to_string(), json!(id));
                }
            }
            "title" => {
                let title = item.title.clone().unwrap_or_default();
                map.insert("title".to_string(), Value::String(title));
            }
            "text" => {
                let text = item.text.clone().unwrap_or_default();
                map.insert("text".to_string(), Value::String(text));
            }
            "url" => {
                if let Some(url) = &item.url {
                    map.insert("url".to_string(), json!(url));
                }
            }
            "author" => {
                if let Some(author) = &item.author {
                    map.insert("author".to_string(), json!(author));
                }
            }
            "created_at" => {
                if let Some(created_at) = &item.created_at {
                    map.insert("created_at".to_string(), json!(created_at));
                }
            }
            "created_at_i" => {
                if let Some(created_at_i) = item.created_at_i {
                    map.insert("created_at_i".to_string(), json!(created_at_i));
                }
            }
            "type" => {
                if let Some(item_type) = &item.r#type {
                    map.insert("type".to_string(), json!(item_type));
                }
            }
            "points" => {
                if let Some(points) = item.points {
                    map.insert("points".to_string(), json!(points));
                }
            }
            "parent_id" => {
                if let Some(parent_id) = item.parent_id {
                    map.insert("parent_id".to_string(), json!(parent_id));
                }
            }
            "story_id" => {
                if let Some(story_id) = item.story_id {
                    map.insert("story_id".to_string(), json!(story_id));
                }
            }
            "options" => {
                if let Some(options) = &item.options {
                    map.insert("options".to_string(), json!(options));
                }
            }
            _ => {}
        }
    }

    let comments = item
        .children
        .as_ref()
        .map(|children| {
            children
                .iter()
                .filter(|child| matches!(child.r#type, Some(ItemType::Comment)))
                .map(|child| comment_item_to_payload(child, comment_fields, true))
                .collect::<Vec<Value>>()
        })
        .unwrap_or_default();

    map.insert("comments".to_string(), Value::Array(comments));

    Value::Object(map)
}

fn comment_item_to_payload(
    item: &HackerNewsItem,
    comment_fields: &HashSet<String>,
    include_children: bool,
) -> Value {
    let mut map = serde_json::Map::new();

    for field in COMMENT_FIELD_ORDER {
        if !comment_fields.contains(*field) {
            continue;
        }

        match *field {
            "id" => {
                if let Some(id) = item.id {
                    map.insert("id".to_string(), json!(id));
                }
            }
            "text" => {
                let text = item.text.clone().unwrap_or_default();
                map.insert("text".to_string(), Value::String(text));
            }
            "author" => {
                if let Some(author) = &item.author {
                    map.insert("author".to_string(), json!(author));
                }
            }
            "created_at" => {
                if let Some(created_at) = &item.created_at {
                    map.insert("created_at".to_string(), json!(created_at));
                }
            }
            "created_at_i" => {
                if let Some(created_at_i) = item.created_at_i {
                    map.insert("created_at_i".to_string(), json!(created_at_i));
                }
            }
            "parent_id" => {
                if let Some(parent_id) = item.parent_id {
                    map.insert("parent_id".to_string(), json!(parent_id));
                }
            }
            "story_id" => {
                if let Some(story_id) = item.story_id {
                    map.insert("story_id".to_string(), json!(story_id));
                }
            }
            "points" => {
                if let Some(points) = item.points {
                    map.insert("points".to_string(), json!(points));
                }
            }
            _ => {}
        }
    }

    let replies = if include_children {
        item.children
            .as_ref()
            .map(|children| {
                children
                    .iter()
                    .filter(|child| matches!(child.r#type, Some(ItemType::Comment)))
                    .map(|child| comment_item_to_payload(child, comment_fields, true))
                    .collect::<Vec<Value>>()
            })
            .unwrap_or_default()
    } else {
        Vec::new()
    };

    map.insert("comments".to_string(), Value::Array(replies));

    Value::Object(map)
}

fn story_as_comment_payload(item: &HackerNewsItem, comment_fields: &HashSet<String>) -> Value {
    let mut map = serde_json::Map::new();

    for field in COMMENT_FIELD_ORDER {
        if !comment_fields.contains(*field) {
            continue;
        }

        match *field {
            "id" => {
                if let Some(id) = item.id {
                    map.insert("id".to_string(), json!(id));
                }
            }
            "text" => {
                let combined = match (&item.title, &item.text) {
                    (Some(title), Some(text)) if !text.is_empty() => {
                        format!("{}\n\n{}", title, text)
                    }
                    (Some(title), _) => title.clone(),
                    (None, Some(text)) => text.clone(),
                    (None, None) => String::new(),
                };
                map.insert("text".to_string(), Value::String(combined));
            }
            "author" => {
                if let Some(author) = &item.author {
                    map.insert("author".to_string(), json!(author));
                }
            }
            "created_at" => {
                if let Some(created_at) = &item.created_at {
                    map.insert("created_at".to_string(), json!(created_at));
                }
            }
            "created_at_i" => {
                if let Some(created_at_i) = item.created_at_i {
                    map.insert("created_at_i".to_string(), json!(created_at_i));
                }
            }
            "story_id" => {
                if let Some(id) = item.id {
                    map.insert("story_id".to_string(), json!(id));
                }
            }
            "points" => {
                if let Some(points) = item.points {
                    map.insert("points".to_string(), json!(points));
                }
            }
            _ => {}
        }
    }

    map.insert("comments".to_string(), Value::Array(Vec::new()));

    Value::Object(map)
}

fn flatten_comment_values(
    item: &HackerNewsItem,
    comment_fields: &HashSet<String>,
    out: &mut Vec<Value>,
) {
    if let Some(children) = &item.children {
        for child in children {
            if !matches!(child.r#type, Some(ItemType::Comment)) {
                continue;
            }

            out.push(comment_item_to_payload(child, comment_fields, false));
            flatten_comment_values(child, comment_fields, out);
        }
    }
}

// Algolia search response structure
#[derive(Debug, Serialize, Deserialize)]
pub struct AlgoliaSearchResponse {
    pub ab_test_id: Option<i64>,
    #[serde(rename = "abTestVariantID")]
    pub ab_test_variant_id: Option<i64>,
    #[serde(rename = "aroundLatLng")]
    pub around_lat_lng: Option<String>,
    #[serde(rename = "automaticRadius")]
    pub automatic_radius: Option<String>,
    pub exhaustive: Option<ExhaustiveInfo>,
    #[serde(rename = "appliedRules")]
    pub applied_rules: Option<Vec<HashMap<String, Value>>>,
    #[serde(rename = "exhaustiveFacetsCount")]
    pub exhaustive_facets_count: Option<bool>,
    #[serde(rename = "exhaustiveNbHits")]
    pub exhaustive_nb_hits: Option<bool>,
    #[serde(rename = "exhaustiveTypo")]
    pub exhaustive_typo: Option<bool>,
    pub facets: Option<HashMap<String, HashMap<String, i64>>>,
    #[serde(rename = "facets_stats")]
    pub facets_stats: Option<HashMap<String, FacetStats>>,
    pub index: Option<String>,
    #[serde(rename = "indexUsed")]
    pub index_used: Option<String>,
    pub message: Option<String>,
    #[serde(rename = "nbSortedHits")]
    pub nb_sorted_hits: Option<i64>,
    #[serde(rename = "parsedQuery")]
    pub parsed_query: Option<String>,
    #[serde(rename = "processingTimeMS")]
    pub processing_time_ms: Option<i64>,
    #[serde(rename = "processingTimingsMS")]
    pub processing_timings_ms: Option<HashMap<String, Value>>,
    #[serde(rename = "queryAfterRemoval")]
    pub query_after_removal: Option<String>,
    pub redirect: Option<RedirectInfo>,
    #[serde(rename = "renderingContent")]
    pub rendering_content: Option<RenderingContent>,
    #[serde(rename = "serverTimeMS")]
    pub server_time_ms: Option<i64>,
    #[serde(rename = "serverUsed")]
    pub server_used: Option<String>,
    #[serde(rename = "userData")]
    pub user_data: Option<HashMap<String, Value>>,
    #[serde(rename = "queryID")]
    pub query_id: Option<String>,
    #[serde(rename = "_automaticInsights")]
    pub automatic_insights: Option<bool>,
    pub page: Option<i64>,
    #[serde(rename = "nbHits")]
    pub nb_hits: Option<i64>,
    #[serde(rename = "nbPages")]
    pub nb_pages: Option<i64>,
    #[serde(rename = "hitsPerPage")]
    pub hits_per_page: Option<i64>,
    pub hits: Option<Vec<AlgoliaHit>>,
    pub query: Option<String>,
    pub params: Option<String>,
}

impl Default for AlgoliaSearchResponse {
    fn default() -> Self {
        Self::new()
    }
}

impl AlgoliaSearchResponse {
    pub fn new() -> Self {
        AlgoliaSearchResponse {
            ab_test_id: None,
            ab_test_variant_id: None,
            around_lat_lng: None,
            automatic_radius: None,
            exhaustive: None,
            applied_rules: None,
            exhaustive_facets_count: None,
            exhaustive_nb_hits: None,
            exhaustive_typo: None,
            facets: None,
            facets_stats: None,
            index: None,
            index_used: None,
            message: None,
            nb_sorted_hits: None,
            parsed_query: None,
            processing_time_ms: None,
            processing_timings_ms: None,
            query_after_removal: None,
            redirect: None,
            rendering_content: None,
            server_time_ms: None,
            server_used: None,
            user_data: None,
            query_id: None,
            automatic_insights: None,
            page: None,
            nb_hits: None,
            nb_pages: None,
            hits_per_page: None,
            hits: None,
            query: None,
            params: None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExhaustiveInfo {
    #[serde(rename = "facetsCount")]
    pub facets_count: Option<bool>,
    #[serde(rename = "facetValues")]
    pub facet_values: Option<bool>,
    #[serde(rename = "nbHits")]
    pub nb_hits: Option<bool>,
    #[serde(rename = "rulesMatch")]
    pub rules_match: Option<bool>,
    pub typo: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FacetStats {
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub avg: Option<f64>,
    pub sum: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RedirectInfo {
    pub index: Option<Vec<RedirectIndexItem>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RedirectIndexItem {
    pub source: Option<String>,
    pub dest: Option<String>,
    pub reason: Option<String>,
    pub succeed: Option<bool>,
    pub data: Option<RedirectData>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RedirectData {
    #[serde(rename = "ruleObjectID")]
    pub rule_object_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RenderingContent {
    #[serde(rename = "facetOrdering")]
    pub facet_ordering: Option<FacetOrdering>,
    pub redirect: Option<RenderingRedirect>,
    pub widgets: Option<Widgets>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FacetOrdering {
    pub facets: Option<FacetOrder>,
    pub values: Option<HashMap<String, FacetValueOrder>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FacetOrder {
    pub order: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FacetValueOrder {
    pub order: Option<Vec<String>>,
    #[serde(rename = "sortRemainingBy")]
    pub sort_remaining_by: Option<String>,
    pub hide: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RenderingRedirect {
    pub url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Widgets {
    pub banners: Option<Vec<Banner>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Banner {
    pub image: Option<BannerImage>,
    pub link: Option<BannerLink>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BannerImage {
    pub urls: Option<Vec<UrlItem>>,
    pub title: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UrlItem {
    pub url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BannerLink {
    pub url: Option<String>,
}

// Remove AlgoliaHit and related structs
#[derive(Clone)]
pub struct HackerNewsConnector {
    client: reqwest::Client,
}

impl Default for HackerNewsConnector {
    fn default() -> Self {
        Self::new()
    }
}

impl HackerNewsConnector {
    pub fn new() -> Self {
        HackerNewsConnector {
            client: reqwest::Client::new(),
        }
    }

    // Helper: fetch JSON from URL
    async fn fetch_json(&self, url: &str) -> Result<Value, ConnectorError> {
        let res = self
            .client
            .get(url)
            .header("User-Agent", "rzn_datasourcer/0.1.0")
            .send()
            .await
            .map_err(|e| ConnectorError::Other(format!("Request error: {}", e)))?;
        let json = res
            .json::<Value>()
            .await
            .map_err(|e| ConnectorError::Other(format!("JSON parse error: {}", e)))?;
        Ok(json)
    }

    // Helper: fetch typed response from URL
    async fn fetch_typed<T: for<'de> Deserialize<'de>>(
        &self,
        url: &str,
    ) -> Result<T, ConnectorError> {
        let res = self
            .client
            .get(url)
            .header("User-Agent", "rzn_datasourcer/0.1.0")
            .send()
            .await
            .map_err(|e| ConnectorError::Other(format!("Request error: {}", e)))?;
        let typed_response = res
            .json::<T>()
            .await
            .map_err(|e| ConnectorError::Other(format!("JSON parse error: {}", e)))?;
        Ok(typed_response)
    }

    // Helper: fetch Algolia search response from URL
    async fn fetch_algolia_search(
        &self,
        url: &str,
    ) -> Result<AlgoliaSearchResponse, ConnectorError> {
        let res = self
            .client
            .get(url)
            .header("User-Agent", "rzn_datasourcer/0.1.0")
            .send()
            .await
            .map_err(|e| ConnectorError::Other(format!("Request error: {}", e)))?;
        let json = res
            .json::<AlgoliaSearchResponse>()
            .await
            .map_err(|e| ConnectorError::Other(format!("JSON parse error: {}", e)))?;
        Ok(json)
    }

    // Helper: fetch a Hacker News item by ID using Algolia API
    async fn get_item(&self, item_id: i64) -> Result<HackerNewsItem, ConnectorError> {
        let url = format!("https://hn.algolia.com/api/v1/items/{}", item_id);
        self.fetch_typed::<HackerNewsItem>(&url).await
    }

    // Helper: fetch top stories
    async fn get_top_stories_list(&self) -> Result<Vec<HackerNewsItem>, ConnectorError> {
        let url = "http://hn.algolia.com/api/v1/search?tags=front_page";
        let response = self.fetch_algolia_search(url).await?;
        Ok(response
            .hits
            .unwrap_or_default()
            .into_iter()
            .filter_map(|hit| {
                if let Some(id) = hit.object_id.and_then(|id| id.parse().ok()) {
                    Some(HackerNewsItem {
                        id: Some(id),
                        author: hit.author,
                        created_at: hit.created_at,
                        created_at_i: hit.created_at_i,
                        r#type: Some(ItemType::Story),
                        text: hit.story_text.or(hit.comment_text),
                        title: hit.title,
                        url: hit.url,
                        points: hit.points,
                        parent_id: hit.parent_id,
                        story_id: hit.story_id,
                        options: None,
                        children: None,
                    })
                } else {
                    None
                }
            })
            .collect())
    }

    // Helper: fetch new stories
    async fn get_new_stories_list(&self) -> Result<Vec<HackerNewsItem>, ConnectorError> {
        let url = "http://hn.algolia.com/api/v1/search_by_date?tags=story";
        let response = self.fetch_algolia_search(url).await?;
        Ok(self.hits_to_items(response.hits.unwrap_or_default()))
    }

    // Helper: fetch best stories (front page sorted by points)
    async fn get_best_stories_list(&self) -> Result<Vec<HackerNewsItem>, ConnectorError> {
        let url = "http://hn.algolia.com/api/v1/search?tags=front_page&hitsPerPage=50";
        let response = self.fetch_algolia_search(url).await?;
        let mut items = self.hits_to_items(response.hits.unwrap_or_default());
        // Sort by points descending for "best"
        items.sort_by(|a, b| b.points.unwrap_or(0).cmp(&a.points.unwrap_or(0)));
        Ok(items)
    }

    // Helper: fetch ask stories
    async fn get_ask_stories_list(&self) -> Result<Vec<HackerNewsItem>, ConnectorError> {
        let url = "http://hn.algolia.com/api/v1/search_by_date?tags=ask_hn";
        let response = self.fetch_algolia_search(url).await?;
        Ok(self.hits_to_items(response.hits.unwrap_or_default()))
    }

    // Helper: fetch show stories
    async fn get_show_stories_list(&self) -> Result<Vec<HackerNewsItem>, ConnectorError> {
        let url = "http://hn.algolia.com/api/v1/search_by_date?tags=show_hn";
        let response = self.fetch_algolia_search(url).await?;
        Ok(self.hits_to_items(response.hits.unwrap_or_default()))
    }

    // Helper: fetch job stories
    async fn get_job_stories_list(&self) -> Result<Vec<HackerNewsItem>, ConnectorError> {
        let url = "http://hn.algolia.com/api/v1/search_by_date?tags=job";
        let response = self.fetch_algolia_search(url).await?;
        Ok(self.hits_to_items(response.hits.unwrap_or_default()))
    }

    // Helper: convert Algolia hits to HackerNewsItems
    fn hits_to_items(&self, hits: Vec<AlgoliaHit>) -> Vec<HackerNewsItem> {
        hits.into_iter()
            .filter_map(|hit| {
                if let Some(id) = hit.object_id.and_then(|id| id.parse().ok()) {
                    Some(HackerNewsItem {
                        id: Some(id),
                        author: hit.author,
                        created_at: hit.created_at,
                        created_at_i: hit.created_at_i,
                        r#type: Some(ItemType::Story),
                        text: hit.story_text.or(hit.comment_text),
                        title: hit.title,
                        url: hit.url,
                        points: hit.points,
                        parent_id: hit.parent_id,
                        story_id: hit.story_id,
                        options: None,
                        children: None,
                    })
                } else {
                    None
                }
            })
            .collect()
    }
}

// Helper function to flatten comments recursively
#[async_trait]
impl Connector for HackerNewsConnector {
    fn name(&self) -> &'static str {
        "hackernews"
    }

    fn description(&self) -> &'static str {
        "A connector for interacting with Hacker News via Firebase and Algolia search API."
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
        // No auth required for public Hacker News API
        Ok(())
    }

    async fn test_auth(&self) -> Result<(), ConnectorError> {
        // Test connectivity by fetching maxitem
        let _ = self
            .fetch_json("https://hacker-news.firebaseio.com/v0/maxitem.json")
            .await?;
        Ok(())
    }

    fn config_schema(&self) -> ConnectorConfigSchema {
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
                "Hacker News connector for accessing stories, comments, and user data. Story endpoints return minimal fields by default (title/text with comment text). Use 'storyFields' or 'commentFields' to include more metadata or prefix '-' to drop defaults.".to_string(),
            ),
        })
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

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParam>,
    ) -> Result<ListToolsResult, ConnectorError> {
        let tools = vec![
            Tool {
                name: Cow::Borrowed("search_stories"),
                title: None,
                description: Some(Cow::Borrowed("Search for Hacker News stories using Algolia")),
                input_schema: Arc::new(json!({
                    "type": "object",
                    "properties": {
                        "query": { "type": "string", "description": "The search query" },
                        "page": { "type": "integer", "description": "Page number", "default": 0 },
                        "hitsPerPage": { "type": "integer", "description": "Results per page", "default": 20 },
                        "tags": {
                            "type": "string",
                            "description": "Filter on specific tags (e.g., 'story', 'comment', 'poll', 'pollopt', 'show_hn', 'ask_hn', 'front_page', 'author_:USERNAME', 'story_:ID')"
                        },
                        "numericFilters": {
                            "type": "string",
                            "description": "Filter on numerical conditions (e.g., 'points>10', 'num_comments>5', 'created_at_i>1600000000')"
                        }
                    },
                    "required": ["query"]
                }).as_object().expect("Schema object").clone()),
                output_schema: None,
                annotations: None,
                icons: None,
            },
            Tool {
                name: Cow::Borrowed("search_by_date"),
                title: None,
                description: Some(Cow::Borrowed("Search for recent Hacker News stories using Algolia")),
                input_schema: Arc::new(json!({
                    "type": "object",
                    "properties": {
                        "query": { "type": "string", "description": "The search query" },
                        "page": { "type": "integer", "description": "Page number", "default": 0 },
                        "hitsPerPage": { "type": "integer", "description": "Results per page", "default": 20 },
                        "tags": {
                            "type": "string",
                            "description": "Filter on specific tags (e.g., 'story', 'comment', 'poll', 'pollopt', 'show_hn', 'ask_hn', 'front_page', 'author_:USERNAME', 'story_:ID')"
                        },
                        "numericFilters": {
                            "type": "string",
                            "description": "Filter on numerical conditions (e.g., 'points>10', 'num_comments>5', 'created_at_i>1600000000')"
                        }
                    },
                    "required": ["query"]
                }).as_object().expect("Schema object").clone()),
                output_schema: None,
                annotations: None,
                icons: None,
            },
            Tool {
                name: Cow::Borrowed("get_top_stories"),
                title: None,
                description: Some(Cow::Borrowed("Get top Hacker News stories")),
                input_schema: Arc::new(json!({
                    "type": "object",
                    "properties": {
                        "limit": { "type": "integer", "description": "Maximum number of stories", "default": 10 },
                        "storyFields": {
                            "type": ["array", "string"],
                            "items": { "type": "string" },
                            "description": "Optional list of additional story fields to include. Prefix with '-' to remove defaults. Defaults: ['title','text']"
                        },
                        "commentFields": {
                            "type": ["array", "string"],
                            "items": { "type": "string" },
                            "description": "Optional list of additional comment fields to include. Prefix with '-' to remove defaults. Defaults: ['text']"
                        }
                    },
                    "required": []
                }).as_object().expect("Schema object").clone()),
                output_schema: None,
                annotations: None,
                icons: None,
            },
            Tool {
                name: Cow::Borrowed("get_new_stories"),
                title: None,
                description: Some(Cow::Borrowed("Get new Hacker News stories")),
                input_schema: Arc::new(json!({
                    "type": "object",
                    "properties": {
                        "limit": { "type": "integer", "description": "Maximum number of stories", "default": 10 },
                        "storyFields": {
                            "type": ["array", "string"],
                            "items": { "type": "string" },
                            "description": "Optional list of additional story fields to include. Prefix with '-' to remove defaults. Defaults: ['title','text']"
                        },
                        "commentFields": {
                            "type": ["array", "string"],
                            "items": { "type": "string" },
                            "description": "Optional list of additional comment fields to include. Prefix with '-' to remove defaults. Defaults: ['text']"
                        }
                    },
                    "required": []
                }).as_object().expect("Schema object").clone()),
                output_schema: None,
                annotations: None,
                icons: None,
            },
            Tool {
                name: Cow::Borrowed("get_best_stories"),
                title: None,
                description: Some(Cow::Borrowed("Get best Hacker News stories (sorted by points)")),
                input_schema: Arc::new(json!({
                    "type": "object",
                    "properties": {
                        "limit": { "type": "integer", "description": "Maximum number of stories", "default": 10 },
                        "storyFields": {
                            "type": ["array", "string"],
                            "items": { "type": "string" },
                            "description": "Optional list of additional story fields to include. Prefix with '-' to remove defaults. Defaults: ['title','text']"
                        },
                        "commentFields": {
                            "type": ["array", "string"],
                            "items": { "type": "string" },
                            "description": "Optional list of additional comment fields to include. Prefix with '-' to remove defaults. Defaults: ['text']"
                        }
                    },
                    "required": []
                }).as_object().expect("Schema object").clone()),
                output_schema: None,
                annotations: None,
                icons: None,
            },
            Tool {
                name: Cow::Borrowed("get_ask_stories"),
                title: None,
                description: Some(Cow::Borrowed("Get Ask HN stories")),
                input_schema: Arc::new(json!({
                    "type": "object",
                    "properties": {
                        "limit": { "type": "integer", "description": "Maximum number of stories", "default": 10 },
                        "storyFields": {
                            "type": ["array", "string"],
                            "items": { "type": "string" },
                            "description": "Optional list of additional story fields to include. Prefix with '-' to remove defaults. Defaults: ['title','text']"
                        },
                        "commentFields": {
                            "type": ["array", "string"],
                            "items": { "type": "string" },
                            "description": "Optional list of additional comment fields to include. Prefix with '-' to remove defaults. Defaults: ['text']"
                        }
                    },
                    "required": []
                }).as_object().expect("Schema object").clone()),
                output_schema: None,
                annotations: None,
                icons: None,
            },
            Tool {
                name: Cow::Borrowed("get_show_stories"),
                title: None,
                description: Some(Cow::Borrowed("Get Show HN stories")),
                input_schema: Arc::new(json!({
                    "type": "object",
                    "properties": {
                        "limit": { "type": "integer", "description": "Maximum number of stories", "default": 10 },
                        "storyFields": {
                            "type": ["array", "string"],
                            "items": { "type": "string" },
                            "description": "Optional list of additional story fields to include. Prefix with '-' to remove defaults. Defaults: ['title','text']"
                        },
                        "commentFields": {
                            "type": ["array", "string"],
                            "items": { "type": "string" },
                            "description": "Optional list of additional comment fields to include. Prefix with '-' to remove defaults. Defaults: ['text']"
                        }
                    },
                    "required": []
                }).as_object().expect("Schema object").clone()),
                output_schema: None,
                annotations: None,
                icons: None,
            },
            Tool {
                name: Cow::Borrowed("get_job_stories"),
                title: None,
                description: Some(Cow::Borrowed("Get job stories from Hacker News")),
                input_schema: Arc::new(json!({
                    "type": "object",
                    "properties": {
                        "limit": { "type": "integer", "description": "Maximum number of stories", "default": 10 },
                        "storyFields": {
                            "type": ["array", "string"],
                            "items": { "type": "string" },
                            "description": "Optional list of additional story fields to include. Prefix with '-' to remove defaults. Defaults: ['title','text']"
                        },
                        "commentFields": {
                            "type": ["array", "string"],
                            "items": { "type": "string" },
                            "description": "Optional list of additional comment fields to include. Prefix with '-' to remove defaults. Defaults: ['text']"
                        }
                    },
                    "required": []
                }).as_object().expect("Schema object").clone()),
                output_schema: None,
                annotations: None,
                icons: None,
            },
            Tool {
                name: Cow::Borrowed("get_post"),
                title: None,
                description: Some(Cow::Borrowed("Get Hacker News post details with nested comments by id")),
                input_schema: Arc::new(json!({
                    "type": "object",
                    "properties": {
                        "id": { "type": "integer", "description": "The Hacker News item id" },
                        "flatten": {
                            "type": "boolean",
                            "description": "Flatten the results into a single array",
                            "default": false
                        },
                        "storyFields": {
                            "type": ["array", "string"],
                            "items": { "type": "string" },
                            "description": "Optional list of additional story fields to include. Prefix with '-' to remove defaults. Defaults: ['title','text']"
                        },
                        "commentFields": {
                            "type": ["array", "string"],
                            "items": { "type": "string" },
                            "description": "Optional list of additional comment fields to include. Prefix with '-' to remove defaults. Defaults: ['text']"
                        }
                    },
                    "required": ["id"]
                }).as_object().expect("Schema object").clone()),
                output_schema: None,
                annotations: None,
                icons: None,
            },
            //  Tool {
            //      name: Cow::Borrowed("get_user"),
            //      description: Some(Cow::Borrowed("Get Hacker News user details by username")),
            //      annotations: None,
            //      input_schema: Arc::new(json!({
            //          "type": "object",
            //          "properties": {
            //              "id": { "type": "string", "description": "The Hacker News username (case-sensitive)" }
            //          },
            //          "required": ["id"]
            //      }).as_object().expect("Schema object").clone()),
            //      output_schema: None,
            //  },
            //  Tool {
            //      name: Cow::Borrowed("get_max_item_id"),
            //      description: Some(Cow::Borrowed("Get the current largest item id on Hacker News")),
            //      annotations: None,
            //      input_schema: Arc::new(json!({
            //          "type": "object",
            //          "properties": {},
            //          "required": []
            //      }).as_object().expect("Schema object").clone()),
            //      output_schema: None,
            //  },
            //  Tool {
            //      name: Cow::Borrowed("get_updates"),
            //      description: Some(Cow::Borrowed("Get the latest item and profile changes on Hacker News")),
            //      annotations: None,
            //      input_schema: Arc::new(json!({
            //          "type": "object",
            //          "properties": {},
            //          "required": []
            //      }).as_object().expect("Schema object").clone()),
            //      output_schema: None,
            //  }
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
            "search_stories" => {
                let query = args.get("query").and_then(|v| v.as_str()).ok_or(
                    ConnectorError::InvalidParams("Missing 'query' parameter".to_string()),
                )?;
                let page = args.get("page").and_then(|v| v.as_i64()).unwrap_or(0);
                let hits_per_page = args
                    .get("hitsPerPage")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(20);

                // Build the base URL
                let mut url = format!(
                    "http://hn.algolia.com/api/v1/search?query={}&page={}&hitsPerPage={}",
                    urlencoding::encode(query),
                    page,
                    hits_per_page
                );

                // Add tags if provided
                if let Some(tags) = args.get("tags").and_then(|v| v.as_str()) {
                    url.push_str(&format!("&tags={}", urlencoding::encode(tags)));
                } else {
                    // Default to story tag if no tags specified
                    //   url.push_str("&tags=story");
                }
                //   if let Some(dateRange) = args.get("dateRange").and_then(|v| v.as_str()) {
                //       url.push_str(&format!("&dateRange={}", urlencoding::encode(dateRange)));
                //   } else {
                //       // Default to last 30 days if no date range specified
                //       url.push_str("&dateRange=all");
                //   }
                // Add numeric filters if provided
                if let Some(numeric_filters) = args.get("numericFilters").and_then(|v| v.as_str()) {
                    url.push_str(&format!(
                        "&numericFilters={}",
                        urlencoding::encode(numeric_filters)
                    ));
                }

                tracing::debug!(url = %url, "Executing Hacker News search");
                let result: AlgoliaSearchResponse = self.fetch_algolia_search(&url).await?;
                let text = serde_json::to_string(&result)?;
                Ok(structured_result_with_text(&result, Some(text))?)
            }
            "search_by_date" => {
                let query = args.get("query").and_then(|v| v.as_str()).ok_or(
                    ConnectorError::InvalidParams("Missing 'query' parameter".to_string()),
                )?;
                let page = args.get("page").and_then(|v| v.as_i64()).unwrap_or(0);
                let hits_per_page = args
                    .get("hitsPerPage")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(20);

                // Build the base URL
                let mut url = format!(
                    "http://hn.algolia.com/api/v1/search_by_date?query={}&page={}&hitsPerPage={}",
                    urlencoding::encode(query),
                    page,
                    hits_per_page
                );

                // Add tags if provided
                if let Some(tags) = args.get("tags").and_then(|v| v.as_str()) {
                    url.push_str(&format!("&tags={}", urlencoding::encode(tags)));
                } else {
                    // Default to story tag if no tags specified
                    url.push_str("&tags=story");
                }

                // Add numeric filters if provided
                if let Some(numeric_filters) = args.get("numericFilters").and_then(|v| v.as_str()) {
                    url.push_str(&format!(
                        "&numericFilters={}",
                        urlencoding::encode(numeric_filters)
                    ));
                }

                let result: AlgoliaSearchResponse = self.fetch_algolia_search(&url).await?;
                let text = serde_json::to_string(&result)?;
                Ok(structured_result_with_text(&result, Some(text))?)
            }
            "get_top_stories" => {
                let limit = args.get("limit").and_then(|v| v.as_i64()).unwrap_or(10) as usize;
                let (story_fields, comment_fields) = parse_field_sets(&args);

                // Get the list of top story IDs
                let story_ids = self.get_top_stories_list().await?;

                // Fetch details for each story up to the limit
                let mut stories = Vec::new();
                for item in story_ids.iter().take(limit) {
                    match item.id {
                        Some(id) => {
                            let story = self.get_item(id).await?;
                            stories.push(story_item_to_payload(
                                &story,
                                &story_fields,
                                &comment_fields,
                            ));
                        }
                        None => {
                            tracing::debug!("Skipping Hacker News item without ID");
                        }
                    }
                }

                let text = serde_json::to_string(&stories)?;
                Ok(structured_result_with_text(&stories, Some(text))?)
            }
            "get_new_stories" => {
                let limit = args.get("limit").and_then(|v| v.as_i64()).unwrap_or(10) as usize;
                let (story_fields, comment_fields) = parse_field_sets(&args);

                // Get the list of new story IDs
                let story_ids = self.get_new_stories_list().await?;

                // Fetch details for each story up to the limit
                let mut stories = Vec::new();
                for item in story_ids.iter().take(limit) {
                    match item.id {
                        Some(id) => {
                            let story = self.get_item(id).await?;
                            stories.push(story_item_to_payload(
                                &story,
                                &story_fields,
                                &comment_fields,
                            ));
                        }
                        None => {
                            tracing::debug!("Skipping Hacker News item without ID");
                        }
                    }
                }

                let text = serde_json::to_string(&stories)?;
                Ok(structured_result_with_text(&stories, Some(text))?)
            }
            "get_best_stories" => {
                let limit = args.get("limit").and_then(|v| v.as_i64()).unwrap_or(10) as usize;
                let (story_fields, comment_fields) = parse_field_sets(&args);

                let story_ids = self.get_best_stories_list().await?;

                let mut stories = Vec::new();
                for item in story_ids.iter().take(limit) {
                    if let Some(id) = item.id {
                        let story = self.get_item(id).await?;
                        stories.push(story_item_to_payload(
                            &story,
                            &story_fields,
                            &comment_fields,
                        ));
                    }
                }

                let text = serde_json::to_string(&stories)?;
                Ok(structured_result_with_text(&stories, Some(text))?)
            }
            "get_ask_stories" => {
                let limit = args.get("limit").and_then(|v| v.as_i64()).unwrap_or(10) as usize;
                let (story_fields, comment_fields) = parse_field_sets(&args);

                let story_ids = self.get_ask_stories_list().await?;

                let mut stories = Vec::new();
                for item in story_ids.iter().take(limit) {
                    if let Some(id) = item.id {
                        let story = self.get_item(id).await?;
                        stories.push(story_item_to_payload(
                            &story,
                            &story_fields,
                            &comment_fields,
                        ));
                    }
                }

                let text = serde_json::to_string(&stories)?;
                Ok(structured_result_with_text(&stories, Some(text))?)
            }
            "get_show_stories" => {
                let limit = args.get("limit").and_then(|v| v.as_i64()).unwrap_or(10) as usize;
                let (story_fields, comment_fields) = parse_field_sets(&args);

                let story_ids = self.get_show_stories_list().await?;

                let mut stories = Vec::new();
                for item in story_ids.iter().take(limit) {
                    if let Some(id) = item.id {
                        let story = self.get_item(id).await?;
                        stories.push(story_item_to_payload(
                            &story,
                            &story_fields,
                            &comment_fields,
                        ));
                    }
                }

                let text = serde_json::to_string(&stories)?;
                Ok(structured_result_with_text(&stories, Some(text))?)
            }
            "get_job_stories" => {
                let limit = args.get("limit").and_then(|v| v.as_i64()).unwrap_or(10) as usize;
                let (story_fields, comment_fields) = parse_field_sets(&args);

                let story_ids = self.get_job_stories_list().await?;

                let mut stories = Vec::new();
                for item in story_ids.iter().take(limit) {
                    if let Some(id) = item.id {
                        let story = self.get_item(id).await?;
                        stories.push(story_item_to_payload(
                            &story,
                            &story_fields,
                            &comment_fields,
                        ));
                    }
                }

                let text = serde_json::to_string(&stories)?;
                Ok(structured_result_with_text(&stories, Some(text))?)
            }
            "get_post" => {
                // println!("args: {:#?}\n\n\n", args);
                let id = args.get("id").and_then(|v| v.as_i64()).ok_or(
                    ConnectorError::InvalidParams("Missing 'id' parameter".to_string()),
                )?;
                let flatten = args
                    .get("flatten")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let (story_fields, comment_fields) = parse_field_sets(&args);

                // Use the Algolia items endpoint directly
                let url = format!("https://hn.algolia.com/api/v1/items/{}", id);
                // println!("URL: {}", url);
                let result = self.fetch_typed::<HackerNewsItem>(&url).await?;

                if flatten {
                    let mut flattened_payload = Vec::new();
                    flattened_payload.push(story_as_comment_payload(&result, &comment_fields));
                    flatten_comment_values(&result, &comment_fields, &mut flattened_payload);

                    let text = serde_json::to_string(&flattened_payload)?;
                    Ok(structured_result_with_text(&flattened_payload, Some(text))?)
                } else {
                    let payload = story_item_to_payload(&result, &story_fields, &comment_fields);
                    let text = serde_json::to_string(&payload)?;
                    Ok(structured_result_with_text(&payload, Some(text))?)
                }
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

    async fn get_prompt(&self, _name: &str) -> Result<Prompt, ConnectorError> {
        Err(ConnectorError::InvalidParams(
            "Prompts not supported".to_string(),
        ))
    }
}
