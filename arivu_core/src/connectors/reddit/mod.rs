use async_trait::async_trait;
use roux::subreddit::response::AccountsActive;
use serde_json::{json, Value};

use chrono;
use reqwest;
use roux::{Reddit, Subreddit, User};
use std::borrow::Cow;
use std::sync::Arc;
use urlencoding;

use crate::auth::AuthDetails;
use crate::capabilities::{ConnectorConfigSchema, Field, FieldType};
use crate::error::ConnectorError;
use crate::utils::structured_result_with_text;
use crate::Connector;
use rmcp::model::*;

pub struct RedditConnector {
    client: Option<Reddit>,
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
        let tools = vec![
            Tool {
                name: Cow::Borrowed("get_user_info"),
                title: None,
                description: Some(Cow::Borrowed("Get a Reddit user profile by username")),
                input_schema: Arc::new(json!({
                    "type": "object",
                    "properties": {
                        "username": {
                            "type": "string",
                            "description": "The username of the Reddit user (e.g., 'spez')"
                        }
                    },
                    "required": ["username"]
                }).as_object().expect("Schema object").clone()),
                output_schema: None,
                annotations: None,
                icons: None,
            },
            Tool {
                name: Cow::Borrowed("get_subreddit_top_posts"),
                title: None,
                description: Some(Cow::Borrowed(
                    "Top posts in a specific subreddit (not keyword search)",
                )),
                input_schema: Arc::new(json!({
                    "type": "object",
                    "properties": {
                        "subreddit": {
                            "type": "string",
                            "description": "The subreddit name (e.g., 'rust' or 'r/rust')"
                        },
                        "limit": {
                            "type": "integer",
                            "description": "The maximum number of posts to return (default: 10)"
                        }
                    },
                    "required": ["subreddit"]
                }).as_object().expect("Schema object").clone()),
                output_schema: None,
                annotations: None,
                icons: None,
            },
            Tool {
                name: Cow::Borrowed("get_subreddit_hot_posts"),
                title: None,
                description: Some(Cow::Borrowed(
                    "Trending/hot posts in a specific subreddit",
                )),
                input_schema: Arc::new(json!({
                    "type": "object",
                    "properties": {
                        "subreddit": {
                            "type": "string",
                            "description": "The subreddit name (e.g., 'rust' or 'r/rust')"
                        },
                        "limit": {
                            "type": "integer",
                            "description": "The maximum number of posts to return (default: 10)"
                        }
                    },
                    "required": ["subreddit"]
                }).as_object().expect("Schema object").clone()),
                output_schema: None,
                annotations: None,
                icons: None,
            },
            Tool {
                name: Cow::Borrowed("get_subreddit_new_posts"),
                title: None,
                description: Some(Cow::Borrowed(
                    "Newest posts in a specific subreddit",
                )),
                input_schema: Arc::new(json!({
                    "type": "object",
                    "properties": {
                        "subreddit": {
                            "type": "string",
                            "description": "The subreddit name (e.g., 'rust' or 'r/rust')"
                        },
                        "limit": {
                            "type": "integer",
                            "description": "The maximum number of posts to return (default: 10)"
                        }
                    },
                    "required": ["subreddit"]
                }).as_object().expect("Schema object").clone()),
                output_schema: None,
                annotations: None,
                icons: None,
            },
            Tool {
                name: Cow::Borrowed("get_subreddit_info"),
                title: None,
                description: Some(Cow::Borrowed("Get subreddit metadata by name")),
                input_schema: Arc::new(json!({
                    "type": "object",
                    "properties": {
                        "subreddit": {
                            "type": "string",
                            "description": "The subreddit name (e.g., 'rust' or 'r/rust')"
                        }
                    },
                    "required": ["subreddit"]
                }).as_object().expect("Schema object").clone()),
                output_schema: None,
                annotations: None,
                icons: None,
            },
            Tool {
                name: Cow::Borrowed("search_reddit"),
                title: None,
                description: Some(Cow::Borrowed(
                    "Keyword search for posts (use when you have query terms)",
                )),
                input_schema: Arc::new(json!({
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "Search query for Reddit posts"
                        },
                        "limit": {
                            "type": "integer",
                            "description": "Maximum number of posts to return",
                            "default": 10
                        },
                        "author": {
                            "type": "string",
                            "description": "Filter by post author username (e.g., 'spez')"
                        },
                        "subreddit": {
                            "type": "string",
                            "description": "Optional subreddit filter (e.g., 'rust' or 'r/rust')"
                        },
                        "flair": {
                            "type": "string",
                            "description": "Filter by post flair text"
                        },
                        "title": {
                            "type": "string",
                            "description": "Search within post titles only"
                        },
                        "selftext": {
                            "type": "string",
                            "description": "Search within post body text only"
                        },
                        "site": {
                            "type": "string",
                            "description": "Filter by domain of submitted URL (e.g., 'github.com')"
                        },
                        "url": {
                            "type": "string",
                            "description": "Filter by URL content"
                        },
                        "self": {
                            "type": "boolean",
                            "description": "Filter to text posts only when true, link posts only when false"
                        },
                        "include_nsfw": {
                            "type": "boolean",
                            "description": "Include NSFW results in search",
                            "default": false
                        }
                    },
                    "required": ["query"]
                }).as_object().expect("Schema object").clone()),
                output_schema: None,
                annotations: None,
                icons: None,
            },
            Tool {
                name: Cow::Borrowed("get_post_details"),
                title: None,
                description: Some(Cow::Borrowed(
                    "Post details and comments; requires a post URL",
                )),
                input_schema: Arc::new(json!({
                    "type": "object",
                    "properties": {
                        "post_url": {
                            "type": "string",
                            "description": "Full Reddit post URL (not post ID)"
                        },
                        "comment_limit": {
                            "type": "integer",
                            "description": "The maximum number of top-level comments to return (default: 25)"
                        },
                        "comment_sort": {
                            "type": "string",
                            "description": "The sort method for comments (default: 'best', options: 'best', 'top', 'new', 'controversial', 'old', 'qa')"
                        }
                    },
                    "required": ["post_url"]
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
                let posts = subreddit.top(limit, None).await.map_err(|e| {
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
                let limit = args.get("limit").and_then(|v| v.as_i64()).unwrap_or(10) as u32;

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
                let page_size = 25; // Same as in Python code
                let base_url = "https://www.reddit.com/";
                let search_url = format!(
                    "{}search.json?q={}&limit={}&include_over_18={}",
                    base_url,
                    urlencoding::encode(&search_query),
                    page_size,
                    include_nsfw
                );

                let response = client
                    .get(&search_url)
                    .header("User-Agent", "rzn_datasourcer/0.1.0")
                    .send()
                    .await
                    .map_err(|e| ConnectorError::Other(format!("Failed to send request: {}", e)))?;

                let search_results: Value = response
                    .json()
                    .await
                    .map_err(|e| ConnectorError::Other(format!("Failed to parse JSON: {}", e)))?;

                // Check if there are results
                if search_results.get("data").is_none() {
                    let empty: Vec<Value> = Vec::new();
                    return structured_result_with_text(&empty, Some("[]".to_string()));
                }

                let posts = search_results["data"]["children"]
                    .as_array()
                    .ok_or(ConnectorError::Other("Invalid response format".to_string()))?;

                let mut img_results = Vec::new();
                let mut text_results = Vec::new();

                // Process results similar to Python code
                for post in posts.iter().take(limit as usize) {
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
                let comment_limit = args
                    .get("comment_limit")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(25) as u32;
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
                    "https://www.reddit.com/r/{}/comments/{}.json?limit={}&sort={}",
                    post_info.subreddit, post_info.post_id, comment_limit, comment_sort
                );

                // Make the request to Reddit API
                let client = reqwest::Client::new();
                let response = client
                    .get(&api_url)
                    .header("User-Agent", "rzn_datasourcer/0.1.0")
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
                let comments = Self::parse_comments(&post_data[1]["data"]["children"]);

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

    // Helper method to recursively parse comments
    fn parse_comments(comments_data: &Value) -> Vec<Value> {
        let empty_vec = Vec::new();
        let comments = comments_data.as_array().unwrap_or(&empty_vec);

        comments
            .iter()
            .filter_map(|comment| {
                let kind = comment["kind"].as_str().unwrap_or("");

                // Skip "more" type comments which are just placeholders
                if kind == "more" {
                    return None;
                }

                let data = &comment["data"];

                // Parse replies recursively if they exist
                let replies = if data["replies"].is_object() {
                    Self::parse_comments(&data["replies"]["data"]["children"])
                } else {
                    Vec::new()
                };

                Some(json!({
                    "id": data["id"].as_str().unwrap_or(""),
                    "author": data["author"].as_str().unwrap_or(""),
                    "body": data["body"].as_str().unwrap_or(""),
                    "body_html": data["body_html"].as_str().unwrap_or(""),
                    "score": data["score"].as_i64().unwrap_or(0),
                    "created_utc": data["created_utc"].as_f64().unwrap_or(0.0),
                    "permalink": data["permalink"].as_str().unwrap_or(""),
                    "depth": data["depth"].as_i64().unwrap_or(0),
                    "is_submitter": data["is_submitter"].as_bool().unwrap_or(false),
                    "distinguished": data["distinguished"].as_str().unwrap_or(""),
                    "stickied": data["stickied"].as_bool().unwrap_or(false),
                    "replies": replies
                }))
            })
            .collect()
    }
}
