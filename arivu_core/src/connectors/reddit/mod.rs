use async_trait::async_trait;
use roux::subreddit::response::AccountsActive;
use serde_json::{json, Value};

use chrono;
use reqwest;
use roux::util::{FeedOption, TimePeriod};
use roux::{Reddit, Subreddit, User};
use std::borrow::Cow;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;

use crate::auth::AuthDetails;
use crate::capabilities::{ConnectorConfigSchema, Field, FieldType};
use crate::error::ConnectorError;
use crate::utils::{collect_paginated, structured_result_with_text, Page};
use crate::Connector;
use rmcp::model::*;

pub struct RedditConnector {
    client: Option<Reddit>,
}

const REDDIT_USER_AGENT: &str = "rzn_datasourcer/0.1.0";
const DEFAULT_COMMENT_LIMIT: u32 = 25;
// Soft limit to prevent runaway fetches when callers pass extremely large values.
const MAX_COMMENT_LIMIT: u32 = 5_000;
const MAX_SEARCH_LIMIT: u32 = 5_000;
const SEARCH_PAGE_SIZE_MAX: usize = 100;
const MAX_SEARCH_REQUESTS: usize = 50;
const MORECHILDREN_BATCH_SIZE: usize = 100;
const MAX_MORECHILDREN_REQUESTS: usize = 100;
const MAX_TOTAL_COMMENTS: usize = 50_000;

#[derive(Debug, Clone)]
struct RedditSearchCursor {
    after: String,
    count: usize,
}

impl RedditConnector {
    pub async fn new(auth: AuthDetails) -> Result<Self, ConnectorError> {
        let mut connector = RedditConnector { client: None };
        connector.set_auth_details(auth).await?;

        Ok(connector)
    }
}

#[async_trait]
impl Connector for RedditConnector {
    fn name(&self) -> &'static str {
        "reddit"
    }

    fn description(&self) -> &'static str {
        "A connector for interacting with Reddit using the roux crate."
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
        // Check if we have credentials
        if let (Some(username), Some(password), Some(client_id), Some(client_secret)) = (
            details.get("username"),
            details.get("password"),
            details.get("client_id"),
            details.get("client_secret"),
        ) {
            // Authenticated client
            let client_builder = Reddit::new(
                client_id,
                client_secret,
                &format!("rzn_datasourcer/0.1.0 (by /u/{})", username),
            )
            .username(username)
            .password(password);

            // We'll store the client builder, not the authenticated client
            self.client = Some(client_builder.clone());

            // Test the authentication
            let me = client_builder
                .login()
                .await
                .map_err(|e| ConnectorError::Other(format!("Failed to authenticate: {}", e)))?;

            // Just to verify it works, we don't need to store the result
            match me.me().await {
                Ok(user) => tracing::debug!(user = %user.id, "Reddit authentication succeeded"),
                Err(e) => tracing::warn!(error = %e, "Reddit authentication verification failed"),
            }
        } else {
            // Anonymous client - no login needed
            let client = Reddit::new(
                "rzn_datasourcer/0.1.0 (anonymous)",
                "CLIENT_ID_NOT_NEEDED_FOR_ANONYMOUS",
                "CLIENT_SECRET_NOT_NEEDED_FOR_ANONYMOUS",
            );

            self.client = Some(client);
        }

        Ok(())
    }

    async fn test_auth(&self) -> Result<(), ConnectorError> {
        // Test by fetching a known user
        let user = User::new("spez");
        let _about = user
            .about(None)
            .await
            .map_err(|e| ConnectorError::Other(format!("Failed to fetch user: {}", e)))?;

        Ok(())
    }

    fn config_schema(&self) -> ConnectorConfigSchema {
        ConnectorConfigSchema {
            fields: vec![
                Field {
                    name: "username".to_string(),
                    field_type: FieldType::Text,
                    description: Some(
                        "Reddit username (optional for anonymous access)".to_string(),
                    ),
                    required: false,
                    label: "Username".to_string(),
                    options: None,
                },
                Field {
                    name: "password".to_string(),
                    field_type: FieldType::Secret,
                    description: Some(
                        "Reddit password (optional for anonymous access)".to_string(),
                    ),
                    required: false,
                    label: "Password".to_string(),
                    options: None,
                },
                Field {
                    name: "client_id".to_string(),
                    field_type: FieldType::Text,
                    description: Some(
                        "Reddit API client ID (optional for anonymous access)".to_string(),
                    ),
                    required: false,
                    label: "Client ID".to_string(),
                    options: None,
                },
                Field {
                    name: "client_secret".to_string(),
                    field_type: FieldType::Secret,
                    description: Some(
                        "Reddit API client secret (optional for anonymous access)".to_string(),
                    ),
                    required: false,
                    label: "Client Secret".to_string(),
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
            instructions: Some(
                "Reddit connector for accessing posts, users, and subreddit data".to_string(),
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
        // Keep the surface small to reduce ambiguity and context bloat for agents.
        // Back-compat: legacy tools are still accepted in call_tool(), but not listed here.
        let tools = vec![
            Tool {
                name: Cow::Borrowed("list"),
                title: None,
                description: Some(Cow::Borrowed(
                    "List posts from a subreddit feed (hot/new/top). Use this for browsing a subreddit, not keyword search. Example: subreddit=\"rust\" sort=\"top\" time=\"week\" limit=10.",
                )),
                input_schema: Arc::new(json!({
                    "type": "object",
                    "properties": {
                        "subreddit": {
                            "type": "string",
                            "description": "Subreddit name, with or without r/ prefix (e.g., \"rust\" or \"r/rust\")."
                        },
                        "sort": {
                            "type": "string",
                            "enum": ["hot", "new", "top"],
                            "description": "Feed type. Use 'top' with a time window; 'hot' for trending; 'new' for latest.",
                            "default": "hot"
                        },
                        "time": {
                            "type": "string",
                            "enum": ["hour", "day", "week", "month", "year", "all"],
                            "description": "Only applies when sort='top'. Default: day.",
                            "default": "day"
                        },
                        "limit": {
                            "type": "integer",
                            "minimum": 1,
                            "maximum": 100,
                            "description": "Max posts to return (default: 10).",
                            "default": 10
                        }
                    },
                    "required": ["subreddit"]
                })
                .as_object()
                .expect("Schema object")
                .clone()),
                output_schema: None,
                annotations: None,
                icons: None,
            },
            Tool {
                name: Cow::Borrowed("search"),
                title: None,
                description: Some(Cow::Borrowed(
                    "Search posts by keywords. Tip: use subreddit=\"rust\" to scope results rather than embedding it in the query string. Example: query=\"async await\" subreddit=\"rust\" limit=10.",
                )),
                input_schema: Arc::new(json!({
                    "type": "object",
                    "properties": {
                        "query": { "type": "string", "description": "Search query text (keywords)." },
                        "sort": { "type": "string", "enum": ["relevance", "hot", "new", "top", "comments"], "default": "relevance", "description": "Search sort order." },
                        "time": { "type": "string", "enum": ["hour", "day", "week", "month", "year", "all"], "default": "all", "description": "Time window filter (maps to Reddit search 't=')." },
                        "limit": { "type": "integer", "minimum": 1, "maximum": 5000, "default": 10 },
                        "subreddit": { "type": "string", "description": "Optional subreddit filter (e.g., \"rust\" or \"r/rust\")." },
                        "author": { "type": "string", "description": "Optional author filter (e.g., \"spez\")." },
                        "include_nsfw": { "type": "boolean", "default": false }
                    },
                    "required": ["query"]
                })
                .as_object()
                .expect("Schema object")
                .clone()),
                output_schema: None,
                annotations: None,
                icons: None,
            },
            Tool {
                name: Cow::Borrowed("get"),
                title: None,
                description: Some(Cow::Borrowed(
                    "Get a post with comments. Provide a full Reddit URL. Tip: set comment_sort=\"best\"|\"top\"|\"new\" and keep comment_limit small for token efficiency. The connector will paginate internally to fetch more than the first page when needed.",
                )),
                input_schema: Arc::new(json!({
                    "type": "object",
                    "properties": {
                        "post_url": { "type": "string", "description": "Full Reddit post URL." },
                        "comment_limit": { "type": "integer", "minimum": 0, "maximum": 5000, "default": 25 },
                        "comment_sort": { "type": "string", "enum": ["best", "top", "new", "controversial", "old", "qa"], "default": "best" }
                    },
                    "required": ["post_url"]
                })
                .as_object()
                .expect("Schema object")
                .clone()),
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
            // === Canonical, low-ambiguity tools ===
            "list" | "list_posts" => {
                let subreddit_name = args.get("subreddit").and_then(|v| v.as_str()).ok_or(
                    ConnectorError::InvalidParams("Missing 'subreddit' parameter".to_string()),
                )?;
                let subreddit_name = subreddit_name.strip_prefix("r/").unwrap_or(subreddit_name);
                let limit = args.get("limit").and_then(|v| v.as_i64()).unwrap_or(10) as u32;
                let sort = args
                    .get("sort")
                    .and_then(|v| v.as_str())
                    .unwrap_or("hot")
                    .to_lowercase();

                match sort.as_str() {
                    "hot" => {
                        let subreddit = Subreddit::new(subreddit_name);
                        let posts = subreddit.hot(limit, None).await.map_err(|e| {
                            ConnectorError::Other(format!("Failed to fetch hot posts: {}", e))
                        })?;

                        let results: Vec<_> = posts
                            .data
                            .children
                            .iter()
                            .map(|post| {
                                json!({
                                    "title": post.data.title,
                                    "url": post.data.url,
                                    "author": post.data.author,
                                    "score": post.data.score,
                                    "num_comments": post.data.num_comments,
                                    "permalink": format!("https://www.reddit.com{}", post.data.permalink),
                                    "created_utc": post.data.created_utc,
                                })
                            })
                            .collect();

                        let text = serde_json::to_string(&results)?;
                        Ok(structured_result_with_text(&results, Some(text))?)
                    }
                    "new" => {
                        let subreddit = Subreddit::new(subreddit_name);
                        let posts = subreddit.latest(limit, None).await.map_err(|e| {
                            ConnectorError::Other(format!("Failed to fetch new posts: {}", e))
                        })?;

                        let results: Vec<_> = posts
                            .data
                            .children
                            .iter()
                            .map(|post| {
                                json!({
                                    "title": post.data.title,
                                    "url": post.data.url,
                                    "author": post.data.author,
                                    "score": post.data.score,
                                    "num_comments": post.data.num_comments,
                                    "permalink": format!("https://www.reddit.com{}", post.data.permalink),
                                    "created_utc": post.data.created_utc,
                                })
                            })
                            .collect();

                        let text = serde_json::to_string(&results)?;
                        Ok(structured_result_with_text(&results, Some(text))?)
                    }
                    "top" => {
                        let subreddit = Subreddit::new(subreddit_name);
                        let time = args
                            .get("time")
                            .and_then(|v| v.as_str())
                            .unwrap_or("day")
                            .to_lowercase();

                        let period = match time.as_str() {
                            "hour" => TimePeriod::Now,
                            "day" => TimePeriod::Today,
                            "week" => TimePeriod::ThisWeek,
                            "month" => TimePeriod::ThisMonth,
                            "year" => TimePeriod::ThisYear,
                            "all" => TimePeriod::AllTime,
                            _ => {
                                return Err(ConnectorError::InvalidParams(format!(
                                    "Invalid 'time' value: '{}'. Expected one of: hour, day, week, month, year, all.",
                                    time
                                )));
                            }
                        };

                        let posts = subreddit
                            .top(limit, Some(FeedOption::new().period(period)))
                            .await
                            .map_err(|e| {
                                ConnectorError::Other(format!("Failed to fetch top posts: {}", e))
                            })?;

                        let results: Vec<_> = posts
                            .data
                            .children
                            .iter()
                            .map(|post| {
                                json!({
                                    "title": post.data.title,
                                    "url": post.data.url,
                                    "author": post.data.author,
                                    "score": post.data.score,
                                    "num_comments": post.data.num_comments,
                                    "permalink": format!("https://www.reddit.com{}", post.data.permalink),
                                    "created_utc": post.data.created_utc,
                                })
                            })
                            .collect();

                        let text = serde_json::to_string(&results)?;
                        Ok(structured_result_with_text(&results, Some(text))?)
                    }
                    _ => Err(ConnectorError::InvalidParams(
                        "sort must be one of: hot, new, top".to_string(),
                    )),
                }
            }
            "search" | "search_posts" => {
                let request = CallToolRequestParam {
                    name: "search_reddit".into(),
                    arguments: Some(args),
                };
                self.call_tool(request).await
            }
            "get" | "get_post" => {
                let request = CallToolRequestParam {
                    name: "get_post_details".into(),
                    arguments: Some(args),
                };
                self.call_tool(request).await
            }

            // === Legacy tool names (kept for compatibility) ===
            "get_user_info" => {
                let username = args.get("username").and_then(|v| v.as_str()).ok_or(
                    ConnectorError::InvalidParams("Missing 'username' parameter".to_string()),
                )?;
                // Strip "u/", "/u/", or leading "/" from username
                let username = username
                    .strip_prefix("/u/")
                    .or_else(|| username.strip_prefix("u/"))
                    .unwrap_or(username);

                let user = User::new(username);
                let about = user
                    .about(None)
                    .await
                    .map_err(|e| ConnectorError::Other(format!("Failed to fetch user: {}", e)))?;

                let data = &about.data;
                let result = json!({
                    "name": data.name,
                    "id": data.id,
                    "link_karma": data.link_karma,
                    "comment_karma": data.comment_karma,
                    "created_utc": data.created_utc,
                    "is_gold": data.is_gold,
                    "is_mod": data.is_mod,
                    "verified": data.verified,
                });

                let text = serde_json::to_string(&result)?;
                Ok(structured_result_with_text(&result, Some(text))?)
            }
            "get_subreddit_top_posts" => {
                let subreddit_name = args.get("subreddit").and_then(|v| v.as_str()).ok_or(
                    ConnectorError::InvalidParams("Missing 'subreddit' parameter".to_string()),
                )?;
                // Strip "r/" prefix if present
                let subreddit_name = subreddit_name.strip_prefix("r/").unwrap_or(subreddit_name);
                let limit = args.get("limit").and_then(|v| v.as_i64()).unwrap_or(10) as u32;

                let subreddit = Subreddit::new(subreddit_name);
                let time = args
                    .get("time")
                    .and_then(|v| v.as_str())
                    .unwrap_or("day")
                    .to_lowercase();

                let period = match time.as_str() {
                    "hour" | "now" => TimePeriod::Now,
                    "day" | "today" => TimePeriod::Today,
                    "week" => TimePeriod::ThisWeek,
                    "month" => TimePeriod::ThisMonth,
                    "year" => TimePeriod::ThisYear,
                    "all" | "alltime" => TimePeriod::AllTime,
                    _ => {
                        return Err(ConnectorError::InvalidParams(format!(
                            "Invalid 'time' value: '{}'. Expected one of: hour, day, week, month, year, all.",
                            time
                        )));
                    }
                };

                let posts = subreddit
                    .top(limit, Some(FeedOption::new().period(period)))
                    .await
                    .map_err(|e| {
                        ConnectorError::Other(format!("Failed to fetch top posts: {}", e))
                    })?;

                let results: Vec<_> = posts
                    .data
                    .children
                    .iter()
                    .map(|post| {
                        json!({
                            "title": post.data.title,
                            "url": post.data.url,
                            "author": post.data.author,
                            "score": post.data.score,
                            "num_comments": post.data.num_comments,
                            "permalink": format!("https://www.reddit.com{}", post.data.permalink),
                            "created_utc": post.data.created_utc,
                        })
                    })
                    .collect();

                let text = serde_json::to_string(&results)?;
                Ok(structured_result_with_text(&results, Some(text))?)
            }
            "get_subreddit_hot_posts" => {
                let subreddit_name = args.get("subreddit").and_then(|v| v.as_str()).ok_or(
                    ConnectorError::InvalidParams("Missing 'subreddit' parameter".to_string()),
                )?;
                // Strip "r/" prefix if present
                let subreddit_name = subreddit_name.strip_prefix("r/").unwrap_or(subreddit_name);
                let limit = args.get("limit").and_then(|v| v.as_i64()).unwrap_or(10) as u32;

                let subreddit = Subreddit::new(subreddit_name);
                let posts = subreddit.hot(limit, None).await.map_err(|e| {
                    ConnectorError::Other(format!("Failed to fetch hot posts: {}", e))
                })?;

                let results: Vec<_> = posts
                    .data
                    .children
                    .iter()
                    .map(|post| {
                        json!({
                            "title": post.data.title,
                            "url": post.data.url,
                            "author": post.data.author,
                            "score": post.data.score,
                            "num_comments": post.data.num_comments,
                            "permalink": format!("https://www.reddit.com{}", post.data.permalink),
                            "created_utc": post.data.created_utc,
                        })
                    })
                    .collect();

                let text = serde_json::to_string(&results)?;
                Ok(structured_result_with_text(&results, Some(text))?)
            }
            "get_subreddit_new_posts" => {
                let subreddit_name = args.get("subreddit").and_then(|v| v.as_str()).ok_or(
                    ConnectorError::InvalidParams("Missing 'subreddit' parameter".to_string()),
                )?;
                // Strip "r/" prefix if present
                let subreddit_name = subreddit_name.strip_prefix("r/").unwrap_or(subreddit_name);
                let limit = args.get("limit").and_then(|v| v.as_i64()).unwrap_or(10) as u32;

                let subreddit = Subreddit::new(subreddit_name);
                let posts = subreddit.latest(limit, None).await.map_err(|e| {
                    ConnectorError::Other(format!("Failed to fetch new posts: {}", e))
                })?;

                let results: Vec<_> = posts
                    .data
                    .children
                    .iter()
                    .map(|post| {
                        json!({
                            "title": post.data.title,
                            "url": post.data.url,
                            "author": post.data.author,
                            "score": post.data.score,
                            "num_comments": post.data.num_comments,
                            "permalink": format!("https://www.reddit.com{}", post.data.permalink),
                            "created_utc": post.data.created_utc,
                        })
                    })
                    .collect();

                let text = serde_json::to_string(&results)?;
                Ok(structured_result_with_text(&results, Some(text))?)
            }
            "get_subreddit_info" => {
                let subreddit_name = args.get("subreddit").and_then(|v| v.as_str()).ok_or(
                    ConnectorError::InvalidParams("Missing 'subreddit' parameter".to_string()),
                )?;
                // Strip "r/" prefix if present
                let subreddit_name = subreddit_name.strip_prefix("r/").unwrap_or(subreddit_name);

                let subreddit = Subreddit::new(subreddit_name);
                let about = subreddit.about().await.map_err(|e| {
                    ConnectorError::Other(format!("Failed to fetch subreddit info: {}", e))
                })?;

                let data = &about;
                let result = json!({
                    "display_name": data.display_name,
                    "title": data.title,
                    "description": data.public_description,
                    "subscribers": data.subscribers,
                    "active_users": format!("{:#?}", data.active_user_count.as_ref().unwrap_or(&AccountsActive::Number(0))),
                    "url": data.url.as_ref().map_or("".to_string(), |url| format!("https://www.reddit.com{}", url)),
                    "created_utc": data.created_utc,
                    "over18": data.over18,
                });

                let text = serde_json::to_string(&result)?;
                Ok(structured_result_with_text(&result, Some(text))?)
            }
            "search_reddit" => {
                let query = args.get("query").and_then(|v| v.as_str()).ok_or(
                    ConnectorError::InvalidParams("Missing 'query' parameter".to_string()),
                )?;
                let sort = args
                    .get("sort")
                    .and_then(|v| v.as_str())
                    .unwrap_or("relevance")
                    .to_lowercase();
                let time = args
                    .get("time")
                    .and_then(|v| v.as_str())
                    .unwrap_or("all")
                    .to_lowercase();

                // Build advanced search query with optional filters
                let mut search_query = query.to_string();

                // Add author filter if provided
                if let Some(author) = args.get("author").and_then(|v| v.as_str()) {
                    if !author.is_empty() {
                        // Strip "u/", "/u/" prefix if present
                        let author = author
                            .strip_prefix("/u/")
                            .or_else(|| author.strip_prefix("u/"))
                            .unwrap_or(author);
                        search_query = format!("{} author:{}", search_query, author);
                    }
                }

                // Add subreddit filter if provided
                if let Some(subreddit) = args.get("subreddit").and_then(|v| v.as_str()) {
                    if !subreddit.is_empty() {
                        // Strip "r/" prefix if present
                        let subreddit_name = subreddit.strip_prefix("r/").unwrap_or(subreddit);
                        search_query = format!("{} subreddit:{}", search_query, subreddit_name);
                    }
                }

                // Add flair filter if provided
                if let Some(flair) = args.get("flair").and_then(|v| v.as_str()) {
                    if !flair.is_empty() {
                        // If flair contains spaces, wrap it in quotes
                        let formatted_flair = if flair.contains(' ') {
                            format!("\"{}\"", flair)
                        } else {
                            flair.to_string()
                        };
                        search_query = format!("{} flair:{}", search_query, formatted_flair);
                    }
                }

                // Add title filter if provided
                if let Some(title) = args.get("title").and_then(|v| v.as_str()) {
                    if !title.is_empty() {
                        // If title contains spaces, wrap it in quotes
                        let formatted_title = if title.contains(' ') {
                            format!("\"{}\"", title)
                        } else {
                            title.to_string()
                        };
                        search_query = format!("{} title:{}", search_query, formatted_title);
                    }
                }

                // Add selftext filter if provided
                if let Some(selftext) = args.get("selftext").and_then(|v| v.as_str()) {
                    if !selftext.is_empty() {
                        // If selftext contains spaces, wrap it in quotes
                        let formatted_selftext = if selftext.contains(' ') {
                            format!("\"{}\"", selftext)
                        } else {
                            selftext.to_string()
                        };
                        search_query = format!("{} selftext:{}", search_query, formatted_selftext);
                    }
                }

                // Add site filter if provided
                if let Some(site) = args.get("site").and_then(|v| v.as_str()) {
                    if !site.is_empty() {
                        search_query = format!("{} site:{}", search_query, site);
                    }
                }

                // Add URL filter if provided
                if let Some(url) = args.get("url").and_then(|v| v.as_str()) {
                    if !url.is_empty() {
                        search_query = format!("{} url:{}", search_query, url);
                    }
                }

                // Add self post filter if provided
                if let Some(self_post) = args.get("self").and_then(|v| v.as_bool()) {
                    search_query = format!("{} self:{}", search_query, self_post);
                }

                // Include NSFW content if specified
                let include_nsfw = args
                    .get("include_nsfw")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                // Use reqwest to directly call the Reddit search API
                let client = reqwest::Client::new();
                let base_url = "https://www.reddit.com/";
                let desired_limit =
                    args.get("limit")
                        .and_then(|v| v.as_i64())
                        .unwrap_or(10)
                        .clamp(1, i64::from(MAX_SEARCH_LIMIT)) as usize;
                let sort_param = match sort.as_str() {
                    "relevance" | "hot" | "new" | "top" | "comments" => sort.as_str(),
                    _ => {
                        return Err(ConnectorError::InvalidParams(
                            "sort must be one of: relevance, hot, new, top, comments".to_string(),
                        ));
                    }
                };
                let time_param = match time.as_str() {
                    "hour" | "day" | "week" | "month" | "year" | "all" => time.as_str(),
                    _ => {
                        return Err(ConnectorError::InvalidParams(
                            "time must be one of: hour, day, week, month, year, all".to_string(),
                        ));
                    }
                };

                let posts = collect_paginated(
                    desired_limit,
                    MAX_SEARCH_REQUESTS,
                    None::<RedditSearchCursor>,
                    |cursor, remaining| {
                        let client = client.clone();
                        let search_query = search_query.clone();
                        async move {
                            let page_limit = remaining.min(SEARCH_PAGE_SIZE_MAX);

                            let mut params: Vec<(String, String)> = vec![
                                ("q".to_string(), search_query.clone()),
                                ("limit".to_string(), page_limit.to_string()),
                                ("include_over_18".to_string(), include_nsfw.to_string()),
                                ("sort".to_string(), sort_param.to_string()),
                                ("t".to_string(), time_param.to_string()),
                                ("raw_json".to_string(), "1".to_string()),
                            ];

                            let mut count = 0usize;
                            if let Some(c) = cursor {
                                count = c.count;
                                params.push(("after".to_string(), c.after));
                                params.push(("count".to_string(), count.to_string()));
                            }

                            let response = client
                                .get(format!("{base_url}search.json"))
                                .header("User-Agent", REDDIT_USER_AGENT)
                                .query(&params)
                                .send()
                                .await
                                .map_err(|e| {
                                    ConnectorError::Other(format!("Failed to send request: {}", e))
                                })?;

                            let search_results: Value = response.json().await.map_err(|e| {
                                ConnectorError::Other(format!("Failed to parse JSON: {}", e))
                            })?;

                            let data = search_results.get("data").ok_or_else(|| {
                                ConnectorError::Other("Invalid response format".to_string())
                            })?;

                            let children = data.get("children").and_then(|c| c.as_array()).ok_or(
                                ConnectorError::Other("Invalid response format".to_string()),
                            )?;

                            let after = data.get("after").and_then(|v| v.as_str()).unwrap_or("");
                            let next_cursor = if after.is_empty() {
                                None
                            } else {
                                Some(RedditSearchCursor {
                                    after: after.to_string(),
                                    count: count.saturating_add(children.len()),
                                })
                            };

                            Ok::<_, ConnectorError>(Page {
                                items: children.clone(),
                                next_cursor,
                            })
                        }
                    },
                    |post: &Value| post["data"]["id"].as_str().map(str::to_string),
                )
                .await?;

                let mut img_results = Vec::new();
                let mut text_results = Vec::new();

                // Process results similar to Python code
                for post in posts.iter().take(desired_limit) {
                    let data = &post["data"];

                    let title = data["title"].as_str().unwrap_or("").to_string();
                    let permalink = data["permalink"].as_str().unwrap_or("").to_string();
                    let full_url = format!("{}{}", base_url.trim_end_matches('/'), permalink);

                    // Check if thumbnail is a valid URL
                    let thumbnail = data["thumbnail"].as_str().unwrap_or("").to_string();
                    if thumbnail.starts_with("http") {
                        let img_src = data["url"].as_str().unwrap_or("").to_string();

                        img_results.push(json!({
                            "url": full_url,
                            "title": title,
                            "img_src": img_src,
                            "thumbnail_src": thumbnail,
                            "template": "images.html"
                        }));
                    } else {
                        // Text result
                        let mut content = data["selftext"].as_str().unwrap_or("").to_string();
                        if content.len() > 500 {
                            content = format!("{}...", &content[0..500]);
                        }

                        // Convert Unix timestamp to datetime
                        let created_utc = data["created_utc"].as_f64().unwrap_or(0.0) as i64;
                        let created = chrono::DateTime::from_timestamp(created_utc, 0)
                            .map(|dt| dt.to_rfc3339())
                            .unwrap_or_else(|| "Unknown date".to_string());

                        text_results.push(json!({
                            "url": full_url,
                            "title": title,
                            "content": content,
                            "publishedDate": created
                        }));
                    }
                }

                // Combine results with images first, then text
                let mut combined_results = Vec::new();
                combined_results.extend(img_results);
                combined_results.extend(text_results);

                let text = serde_json::to_string(&combined_results)?;
                Ok(structured_result_with_text(&combined_results, Some(text))?)
            }
            "get_post_details" => {
                let post_url = args.get("post_url").and_then(|v| v.as_str()).ok_or(
                    ConnectorError::InvalidParams("Missing 'post_url' parameter".to_string()),
                )?;
                let comment_limit =
                    args.get("comment_limit")
                        .and_then(|v| v.as_i64())
                        .unwrap_or(i64::from(DEFAULT_COMMENT_LIMIT))
                        .clamp(0, i64::from(MAX_COMMENT_LIMIT)) as u32;
                let comment_sort = args
                    .get("comment_sort")
                    .and_then(|v| v.as_str())
                    .unwrap_or("best");

                // Extract post ID and subreddit from URL
                let post_info = self.extract_post_info_from_url(post_url).ok_or(
                    ConnectorError::InvalidParams("Invalid post URL format".to_string()),
                )?;

                // Construct the API URL to fetch post details with comments
                let api_url = format!(
                    "https://www.reddit.com/r/{}/comments/{}.json?limit={}&sort={}&raw_json=1",
                    post_info.subreddit, post_info.post_id, comment_limit, comment_sort
                );

                // Make the request to Reddit API
                let client = reqwest::Client::new();
                let response = client
                    .get(&api_url)
                    .header("User-Agent", REDDIT_USER_AGENT)
                    .send()
                    .await
                    .map_err(|e| ConnectorError::Other(format!("Failed to send request: {}", e)))?;

                let post_data: Vec<Value> = response
                    .json()
                    .await
                    .map_err(|e| ConnectorError::Other(format!("Failed to parse JSON: {}", e)))?;

                if post_data.len() < 2 {
                    return Err(ConnectorError::Other("Invalid response format".to_string()));
                }

                // Extract post details from the first element
                let post = &post_data[0]["data"]["children"][0]["data"];

                // Extract comments from the second element
                let link_fullname = format!("t3_{}", post_info.post_id);
                let comments = Self::fetch_comment_tree_with_more(
                    &client,
                    &post_data[1]["data"]["children"],
                    &link_fullname,
                    comment_limit,
                    comment_sort,
                )
                .await?;

                // Build the result
                let result = json!({
                    "post": {
                        "id": post["id"].as_str().unwrap_or(""),
                        "title": post["title"].as_str().unwrap_or(""),
                        "author": post["author"].as_str().unwrap_or(""),
                        "subreddit": post["subreddit"].as_str().unwrap_or(""),
                        "selftext": post["selftext"].as_str().unwrap_or(""),
                        "selftext_html": post["selftext_html"].as_str().unwrap_or(""),
                        "score": post["score"].as_i64().unwrap_or(0),
                        "upvote_ratio": post["upvote_ratio"].as_f64().unwrap_or(0.0),
                        "num_comments": post["num_comments"].as_i64().unwrap_or(0),
                        "created_utc": post["created_utc"].as_f64().unwrap_or(0.0),
                        "permalink": post["permalink"].as_str().unwrap_or(""),
                        "url": post["url"].as_str().unwrap_or(""),
                        "is_video": post["is_video"].as_bool().unwrap_or(false),
                        "is_self": post["is_self"].as_bool().unwrap_or(false),
                        "over_18": post["over_18"].as_bool().unwrap_or(false),
                        "spoiler": post["spoiler"].as_bool().unwrap_or(false),
                        "media": post["media"].clone(),
                        "media_metadata": post["media_metadata"].clone(),
                        "gallery_data": post["gallery_data"].clone(),
                    },
                    "comments": comments
                });

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

    async fn get_prompt(&self, _name: &str) -> Result<Prompt, ConnectorError> {
        Err(ConnectorError::InvalidParams(
            "Prompts not supported".to_string(),
        ))
    }
}

// Helper struct to store post information extracted from URL
struct PostInfo {
    subreddit: String,
    post_id: String,
}

impl RedditConnector {
    // Helper method to extract post ID and subreddit from a Reddit post URL
    fn extract_post_info_from_url(&self, url: &str) -> Option<PostInfo> {
        // Handle different Reddit URL formats
        let url = url.trim();

        // Regular Reddit URL pattern: reddit.com/r/subreddit/comments/post_id/...
        let reddit_patterns = [
            r"(?:https?://)?(?:www\.)?reddit\.com/r/([^/]+)/comments/([^/]+)",
            r"(?:https?://)?(?:old\.)?reddit\.com/r/([^/]+)/comments/([^/]+)",
            r"(?:https?://)?(?:new\.)?reddit\.com/r/([^/]+)/comments/([^/]+)",
            r"(?:https?://)?(?:np\.)?reddit\.com/r/([^/]+)/comments/([^/]+)",
        ];

        for pattern in reddit_patterns {
            if let Ok(regex) = regex::Regex::new(pattern) {
                if let Some(captures) = regex.captures(url) {
                    if captures.len() >= 3 {
                        return Some(PostInfo {
                            subreddit: captures[1].to_string(),
                            post_id: captures[2].to_string(),
                        });
                    }
                }
            }
        }

        None
    }

    fn sort_for_morechildren(comment_sort: &str) -> &str {
        match comment_sort {
            // Reddit's /api/morechildren uses "confidence" instead of "best".
            "best" => "confidence",
            "top" => "top",
            "new" => "new",
            "controversial" => "controversial",
            "old" => "old",
            "qa" => "qa",
            // Keep same default behavior as the main comments endpoint.
            _ => "confidence",
        }
    }

    async fn fetch_comment_tree_with_more(
        client: &reqwest::Client,
        initial_children: &Value,
        link_fullname: &str,
        top_level_limit: u32,
        comment_sort: &str,
    ) -> Result<Vec<Value>, ConnectorError> {
        if top_level_limit == 0 {
            return Ok(Vec::new());
        }

        let mut order: u64 = 0;
        let mut comments_by_id: HashMap<String, CollectedComment> = HashMap::new();
        let mut more_queue: VecDeque<MorePlaceholder> = VecDeque::new();
        let mut seen: HashSet<String> = HashSet::new();

        Self::collect_from_listing(
            initial_children,
            link_fullname,
            &mut order,
            &mut comments_by_id,
            &mut more_queue,
            &mut seen,
        );

        let mut more_requests: usize = 0;

        while Self::top_level_count(&comments_by_id, link_fullname) < top_level_limit as usize
            && !more_queue.is_empty()
            && more_requests < MAX_MORECHILDREN_REQUESTS
            && comments_by_id.len() < MAX_TOTAL_COMMENTS
        {
            // Prefer placeholders that expand top-level comments (parent == link fullname).
            let preferred_idx = more_queue
                .iter()
                .position(|m| m.parent_fullname == link_fullname);
            let more = preferred_idx
                .and_then(|idx| more_queue.remove(idx))
                .unwrap_or_else(|| {
                    more_queue.pop_front().unwrap_or_else(|| MorePlaceholder {
                        parent_fullname: link_fullname.to_string(),
                        children: Vec::new(),
                        depth: 0,
                    })
                });

            if more.children.is_empty() {
                continue;
            }

            for chunk in more.children.chunks(MORECHILDREN_BATCH_SIZE) {
                if Self::top_level_count(&comments_by_id, link_fullname) >= top_level_limit as usize
                    || comments_by_id.len() >= MAX_TOTAL_COMMENTS
                    || more_requests >= MAX_MORECHILDREN_REQUESTS
                {
                    break;
                }

                let unfetched: Vec<String> = chunk
                    .iter()
                    .filter(|id| !seen.contains(*id))
                    .cloned()
                    .collect();
                if unfetched.is_empty() {
                    continue;
                }

                let things = Self::fetch_morechildren_things(
                    client,
                    link_fullname,
                    &unfetched,
                    comment_sort,
                    more.depth,
                )
                .await?;
                more_requests += 1;

                Self::collect_from_things(
                    &things,
                    link_fullname,
                    &mut order,
                    &mut comments_by_id,
                    &mut more_queue,
                    &mut seen,
                );
            }
        }

        Ok(Self::build_comment_tree(
            &comments_by_id,
            link_fullname,
            top_level_limit as usize,
        ))
    }

    fn top_level_count(
        comments_by_id: &HashMap<String, CollectedComment>,
        link_fullname: &str,
    ) -> usize {
        comments_by_id
            .values()
            .filter(|c| c.parent_fullname == link_fullname)
            .count()
    }

    fn collect_from_listing(
        children: &Value,
        link_fullname: &str,
        order: &mut u64,
        comments_by_id: &mut HashMap<String, CollectedComment>,
        more_queue: &mut VecDeque<MorePlaceholder>,
        seen: &mut HashSet<String>,
    ) {
        let empty_vec = Vec::new();
        let items = children.as_array().unwrap_or(&empty_vec);
        Self::collect_from_things(
            items,
            link_fullname,
            order,
            comments_by_id,
            more_queue,
            seen,
        );
    }

    fn collect_from_things(
        things: &[Value],
        link_fullname: &str,
        order: &mut u64,
        comments_by_id: &mut HashMap<String, CollectedComment>,
        more_queue: &mut VecDeque<MorePlaceholder>,
        seen: &mut HashSet<String>,
    ) {
        for thing in things {
            let kind = thing["kind"].as_str().unwrap_or("");
            match kind {
                "t1" => {
                    let data = &thing["data"];
                    let id = data["id"].as_str().unwrap_or("").to_string();
                    if id.is_empty() || seen.contains(&id) {
                        continue;
                    }

                    let parent_fullname = data["parent_id"]
                        .as_str()
                        .unwrap_or(link_fullname)
                        .to_string();
                    let comment = CollectedComment {
                        id: id.clone(),
                        parent_fullname,
                        author: data["author"].as_str().unwrap_or("").to_string(),
                        body: data["body"].as_str().unwrap_or("").to_string(),
                        body_html: data["body_html"].as_str().unwrap_or("").to_string(),
                        score: data["score"].as_i64().unwrap_or(0),
                        created_utc: data["created_utc"].as_f64().unwrap_or(0.0),
                        permalink: data["permalink"].as_str().unwrap_or("").to_string(),
                        is_submitter: data["is_submitter"].as_bool().unwrap_or(false),
                        distinguished: data["distinguished"].as_str().unwrap_or("").to_string(),
                        stickied: data["stickied"].as_bool().unwrap_or(false),
                        order: *order,
                    };
                    *order = order.saturating_add(1);
                    seen.insert(id.clone());
                    comments_by_id.insert(id.clone(), comment);

                    if data["replies"].is_object() {
                        let replies = &data["replies"]["data"]["children"];
                        let empty_vec = Vec::new();
                        let reply_items = replies.as_array().unwrap_or(&empty_vec);
                        Self::collect_from_things(
                            reply_items,
                            link_fullname,
                            order,
                            comments_by_id,
                            more_queue,
                            seen,
                        );
                    }
                }
                "more" => {
                    let data = &thing["data"];
                    let parent_fullname = data["parent_id"]
                        .as_str()
                        .unwrap_or(link_fullname)
                        .to_string();
                    let depth = data["depth"].as_i64().unwrap_or(0);
                    let children = data["children"]
                        .as_array()
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| v.as_str().map(str::to_string))
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default();

                    if !children.is_empty() {
                        more_queue.push_back(MorePlaceholder {
                            parent_fullname,
                            children,
                            depth,
                        });
                    }
                }
                _ => {}
            }
        }
    }

    async fn fetch_morechildren_things(
        client: &reqwest::Client,
        link_fullname: &str,
        children: &[String],
        comment_sort: &str,
        depth: i64,
    ) -> Result<Vec<Value>, ConnectorError> {
        let url = "https://www.reddit.com/api/morechildren.json";
        let params = [
            ("api_type", "json".to_string()),
            ("link_id", link_fullname.to_string()),
            ("children", children.join(",")),
            ("limit_children", "true".to_string()),
            (
                "sort",
                Self::sort_for_morechildren(comment_sort).to_string(),
            ),
            ("raw_json", "1".to_string()),
            ("depth", depth.to_string()),
        ];

        let response = client
            .get(url)
            .header("User-Agent", REDDIT_USER_AGENT)
            .query(&params)
            .send()
            .await
            .map_err(|e| ConnectorError::Other(format!("Failed to send request: {}", e)))?;

        let data: Value = response
            .json()
            .await
            .map_err(|e| ConnectorError::Other(format!("Failed to parse JSON: {}", e)))?;

        let things = data["json"]["data"]["things"]
            .as_array()
            .cloned()
            .unwrap_or_default();

        Ok(things)
    }

    fn build_comment_tree(
        comments_by_id: &HashMap<String, CollectedComment>,
        link_fullname: &str,
        top_level_limit: usize,
    ) -> Vec<Value> {
        let mut children_by_parent: HashMap<String, Vec<String>> = HashMap::new();
        for (id, comment) in comments_by_id {
            children_by_parent
                .entry(comment.parent_fullname.clone())
                .or_default()
                .push(id.clone());
        }

        for children in children_by_parent.values_mut() {
            children.sort_by_key(|id| comments_by_id.get(id).map(|c| c.order).unwrap_or(u64::MAX));
        }

        let top_level_ids = children_by_parent
            .get(link_fullname)
            .cloned()
            .unwrap_or_default();

        top_level_ids
            .into_iter()
            .take(top_level_limit)
            .filter_map(|id| Self::render_comment(&id, comments_by_id, &children_by_parent, 0))
            .collect()
    }

    fn render_comment(
        id: &str,
        comments_by_id: &HashMap<String, CollectedComment>,
        children_by_parent: &HashMap<String, Vec<String>>,
        depth: i64,
    ) -> Option<Value> {
        let comment = comments_by_id.get(id)?;

        let fullname = format!("t1_{}", comment.id);
        let reply_ids = children_by_parent
            .get(&fullname)
            .cloned()
            .unwrap_or_default();
        let replies: Vec<Value> = reply_ids
            .into_iter()
            .filter_map(|rid| {
                Self::render_comment(&rid, comments_by_id, children_by_parent, depth + 1)
            })
            .collect();

        Some(json!({
            "id": comment.id,
            "author": comment.author,
            "body": comment.body,
            "body_html": comment.body_html,
            "score": comment.score,
            "created_utc": comment.created_utc,
            "permalink": comment.permalink,
            "depth": depth,
            "is_submitter": comment.is_submitter,
            "distinguished": comment.distinguished,
            "stickied": comment.stickied,
            "replies": replies
        }))
    }
}

#[derive(Debug, Clone)]
struct MorePlaceholder {
    parent_fullname: String,
    children: Vec<String>,
    depth: i64,
}

#[derive(Debug, Clone)]
struct CollectedComment {
    id: String,
    parent_fullname: String,
    author: String,
    body: String,
    body_html: String,
    score: i64,
    created_utc: f64,
    permalink: String,
    is_submitter: bool,
    distinguished: String,
    stickied: bool,
    order: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_tree_from_morechildren_things() {
        let link_fullname = "t3_post";

        let initial_children = json!([
            { "kind": "t1", "data": { "id": "c1", "parent_id": "t3_post", "author": "a", "body": "b", "body_html": "h", "score": 1, "created_utc": 1.0, "permalink": "/r/x/comments/post/_/c1", "depth": 0, "is_submitter": false, "distinguished": "", "stickied": false, "replies": "" } },
            { "kind": "more", "data": { "parent_id": "t3_post", "children": ["c2", "c3"], "depth": 0 } }
        ]);

        let mut order = 0u64;
        let mut comments_by_id: HashMap<String, CollectedComment> = HashMap::new();
        let mut more_queue: VecDeque<MorePlaceholder> = VecDeque::new();
        let mut seen: HashSet<String> = HashSet::new();
        RedditConnector::collect_from_listing(
            &initial_children,
            link_fullname,
            &mut order,
            &mut comments_by_id,
            &mut more_queue,
            &mut seen,
        );

        assert_eq!(comments_by_id.len(), 1);
        assert_eq!(more_queue.len(), 1);

        let more_things = vec![
            json!({ "kind": "t1", "data": { "id": "c2", "parent_id": "t3_post", "author": "a2", "body": "b2", "body_html": "h2", "score": 2, "created_utc": 2.0, "permalink": "/r/x/comments/post/_/c2", "is_submitter": false, "distinguished": "", "stickied": false } }),
            json!({ "kind": "t1", "data": { "id": "c3", "parent_id": "t3_post", "author": "a3", "body": "b3", "body_html": "h3", "score": 3, "created_utc": 3.0, "permalink": "/r/x/comments/post/_/c3", "is_submitter": false, "distinguished": "", "stickied": false } }),
            json!({ "kind": "t1", "data": { "id": "r1", "parent_id": "t1_c2", "author": "ar", "body": "br", "body_html": "hr", "score": 1, "created_utc": 4.0, "permalink": "/r/x/comments/post/_/r1", "is_submitter": false, "distinguished": "", "stickied": false } }),
        ];

        RedditConnector::collect_from_things(
            &more_things,
            link_fullname,
            &mut order,
            &mut comments_by_id,
            &mut more_queue,
            &mut seen,
        );

        let tree = RedditConnector::build_comment_tree(&comments_by_id, link_fullname, 10);
        assert_eq!(tree.len(), 3);
        assert_eq!(tree[0]["id"], "c1");
        assert_eq!(tree[1]["id"], "c2");
        assert_eq!(tree[2]["id"], "c3");

        assert_eq!(tree[1]["depth"], 0);
        assert_eq!(tree[1]["replies"][0]["id"], "r1");
        assert_eq!(tree[1]["replies"][0]["depth"], 1);
    }
}
