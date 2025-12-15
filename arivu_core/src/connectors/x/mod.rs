// src/connectors/x/mod.rs

use std::borrow::Cow;
use std::sync::Arc;

use crate::capabilities::{ConnectorConfigSchema, Field, FieldType};
use crate::error::ConnectorError;
use crate::utils::{get_cookies, match_browser, structured_result_with_text};
use crate::{auth::AuthDetails, Connector};
use agent_twitter_client::timeline::v1::{QueryProfilesResponse, QueryTweetsResponse};
use agent_twitter_client::timeline::v2::QueryTweetsResponse as V2QueryTweetsResponse;
use async_trait::async_trait;
use serde_json::{json, Value};

// Directly use types from agent-twitter-client
use agent_twitter_client::models::{Profile, Tweet};
use agent_twitter_client::scraper::Scraper;
use agent_twitter_client::search::SearchMode;

// use agent_twitter_client::error::Error as AgentError;

use rmcp::model::*;

pub struct XConnector {
    scraper: Scraper, // Directly use AgentScraper
}

impl XConnector {
    pub async fn new(auth: AuthDetails) -> Result<Self, ConnectorError> {
        let mut connector = XConnector {
            scraper: Scraper::new()
                .await
                .map_err(|e| ConnectorError::Other(e.to_string()))?,
        };

        // Validate auth details before proceeding
        // connector.validate_auth_details(&auth)?;

        // Set the auth details which will handle either cookie-based or credential-based auth
        connector.set_auth_details(auth).await?;

        Ok(connector)
    }
}

#[async_trait]
impl Connector for XConnector {
    fn name(&self) -> &'static str {
        "x"
    }

    fn description(&self) -> &'static str {
        "A connector for interacting with X (formerly Twitter)."
    }

    async fn capabilities(&self) -> ServerCapabilities {
        // Define the capabilities according to what your connector supports.
        ServerCapabilities {
            tools: None,
            ..Default::default() // Use default for other capabilities
        }
    }

    async fn get_auth_details(&self) -> Result<AuthDetails, ConnectorError> {
        Ok(AuthDetails::new())
    }

    async fn set_auth_details(&mut self, details: AuthDetails) -> Result<(), ConnectorError> {
        // If no auth details provided, skip authentication (allows listing tools without auth)
        if details.is_empty() {
            return Ok(());
        }

        // Check for browser-based cookie extraction
        if let Some(browser) = details.get("browser") {
            let browser = match_browser(browser.to_string())
                .await
                .map_err(|e| ConnectorError::Other(e.to_string()))?;
            let cookies = get_cookies(browser, "x.com".to_string())
                .await
                .map_err(|e| ConnectorError::Other(e.to_string()))?;
            self.scraper
                .set_from_cookie_string(&cookies)
                .await
                .map_err(|e| ConnectorError::Other(e.to_string()))?;
            return Ok(());
        }

        // If no cookies, try credentials-based auth
        let username = details.get("username").ok_or_else(|| {
            ConnectorError::InvalidInput("Username is required for credential auth".to_string())
        })?;
        let password = details.get("password").ok_or_else(|| {
            ConnectorError::InvalidInput("Password is required for credential auth".to_string())
        })?;

        // Optional email and 2FA
        let email = details.get("email").map(|s| s.to_string());
        let two_fa = details.get("2fa_secret").map(|s| s.to_string());

        self.scraper
            .login(
                username.to_string(),
                password.to_string(),
                email.map(|s| s.to_string()),
                two_fa.map(|s| s.to_string()),
            )
            .await
            .map_err(|e| ConnectorError::Other(e.to_string()))?;

        Ok(())
    }

    async fn test_auth(&self) -> Result<(), ConnectorError> {
        let profile = self
            .scraper
            .get_profile("elonmusk")
            .await
            .map_err(|e| ConnectorError::Other(e.to_string()))?;
        tracing::debug!(?profile, "Fetched sample profile during X auth test");
        Ok(())
    }
    fn config_schema(&self) -> ConnectorConfigSchema {
        ConnectorConfigSchema {
            fields: vec![
                Field {
                    //Browser
                    name: "browser".to_string(),
                    label: "Browser for Cookie Extraction".to_string(),
                    field_type: FieldType::Select {
                        options: vec![
                            "chrome".to_string(),
                            "firefox".to_string(),
                            "edge".to_string(),
                            "safari".to_string(),
                        ],
                    },
                    required: false, // Only required if using cookie auth, handled by logic
                    description: Some(
                        "Select the browser from which to extract cookies.".to_string(),
                    ),
                    options: None,
                },
                Field {
                    //Username
                    name: "username".to_string(),
                    label: "X Username".to_string(),
                    field_type: FieldType::Text,
                    required: false, // NOT individually required
                    description: Some("Your X username.".to_string()),
                    options: None,
                },
                Field {
                    //Password
                    name: "password".to_string(),
                    label: "X Password".to_string(),
                    field_type: FieldType::Secret,
                    required: false, // NOT individually required
                    description: Some("Your X password.".to_string()),
                    options: None,
                },
                Field {
                    // Bearer token
                    name: "email".to_string(),
                    label: "X Email".to_string(),
                    field_type: FieldType::Text,
                    required: true,
                    description: Some("Your X Email".to_string()),
                    options: None,
                },
                Field {
                    // Bearer token
                    name: "2fa_secret".to_string(),
                    label: "X 2FA Secret".to_string(),
                    field_type: FieldType::Secret,
                    required: true,
                    description: Some("Your X 2FA Secret".to_string()),
                    options: None,
                },
            ],
        }
    }

    async fn initialize(
        &self,
        _request: InitializeRequestParam,
    ) -> Result<InitializeResult, ConnectorError> {
        // Implement initialization logic (if needed).
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
                "X (Twitter) connector for accessing user profiles, tweets, and social media data"
                    .to_string(),
            ),
        })
    }

    async fn list_resources(
        &self,
        request: Option<PaginatedRequestParam>,
    ) -> Result<ListResourcesResult, ConnectorError> {
        let _cursor = request.and_then(|r| r.cursor);
        let resources = vec![
            Resource {
                raw: RawResource {
                    uri: "twitter://user/{username}".to_string(),
                    name: "User Profile".to_string(),
                    title: None,
                    description: Some("Represents an X user profile.".to_string()),
                    mime_type: Some("application/vnd.twitter.user+json".to_string()),
                    size: None,
                    icons: None,
                },
                annotations: None,
            },
            Resource {
                raw: RawResource {
                    uri: "twitter://tweet/{tweet_id}".to_string(),
                    name: "Tweet".to_string(),
                    title: None,
                    description: Some("Represents a Tweet.".to_string()),
                    mime_type: Some("application/vnd.twitter.tweet+json".to_string()),
                    size: None,
                    icons: None,
                },
                annotations: None,
            },
        ];

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

        if uri_str.starts_with("twitter://user/") {
            let parts: Vec<&str> = uri_str.split('/').collect();
            if parts.len() < 4 {
                return Err(ConnectorError::InvalidInput(format!(
                    "Invalid resource URI: {}",
                    uri_str
                )));
            }
            let username = parts[3];

            let profile = self
                .scraper
                .get_profile(username)
                .await
                .map_err(|e| ConnectorError::Other(e.to_string()))?;
            let content_text = serde_json::to_string(&profile)?;
            Ok(vec![ResourceContents::text(content_text, uri_str)])
        } else if uri_str.starts_with("twitter://tweet/") {
            let parts: Vec<&str> = uri_str.split('/').collect();

            if parts.len() < 4 {
                return Err(ConnectorError::InvalidInput(format!(
                    "Invalid resource URI: {}",
                    uri_str
                )));
            }
            let tweet_id = parts[3];
            let tweet = self
                .scraper
                .get_tweet(tweet_id)
                .await
                .map_err(|e| ConnectorError::Other(e.to_string()))?;
            let content_text = serde_json::to_string(&tweet)?;
            Ok(vec![ResourceContents::text(content_text, uri_str)])
        } else {
            Err(ConnectorError::ResourceNotFound)
        }
    }

    async fn list_tools(
        &self,
        request: Option<PaginatedRequestParam>,
    ) -> Result<ListToolsResult, ConnectorError> {
        let _cursor = request.and_then(|r| r.cursor);
        let tools = vec![
            Tool {
                name: Cow::Borrowed("get_profile"),
                title: None,
                description: Some(Cow::Borrowed("Retrieves a user's profile information.")),
                input_schema: Arc::new(
                    json!({
                        "type": "object",
                        "properties": {
                            "username": {
                                "type": "string",
                                "description": "The X username."
                            }
                        },
                        "required": ["username"]
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
                name: Cow::Borrowed("search_tweets"),
                title: None,
                description: Some(Cow::Borrowed("Searches for tweets matching a query.")),
                input_schema: Arc::new(
                    json!({
                        "type": "object",
                        "properties": {
                            "query": {
                                "type": "string",
                                "description": "The search query."
                            },
                            "limit": {
                                "type": "integer",
                                "description": "Maximum number of tweets to return."
                            }
                        },
                        "required": ["query"]
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
                name: Cow::Borrowed("get_followers"),
                title: None,
                description: Some(Cow::Borrowed("Retrieves a list of followers for a user.")),
                input_schema: Arc::new(
                    json!({
                        "type": "object",
                        "properties":{
                            "username": {
                                "type": "string",
                                "description": "The user's name"
                            },
                            "limit":{
                                "type": "integer",
                                "description": "Maximum number of followers to return"
                            },
                            "cursor":{
                                "type": "string",
                                "description": "Optional cursor for pagination"
                            }
                        },
                        "required": ["username", "limit"]
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
                name: Cow::Borrowed("get_tweet"),
                title: None,
                description: Some(Cow::Borrowed(
                    "Retrieves details of a specific tweet given its ID",
                )),
                input_schema: Arc::new(
                    json!({
                        "type": "object",
                        "properties":{
                            "tweet_id":{
                                "type": "string",
                                "description": "The ID of the tweet"
                            }
                        },
                        "required": ["tweet_id"]
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
                name: Cow::Borrowed("get_home_timeline"),
                title: None,
                description: Some(Cow::Borrowed(
                    "Retrieves tweets from the user's home timeline.",
                )),
                input_schema: Arc::new(
                    json!({
                        "type": "object",
                        "properties":{
                            "count":{
                                "type": "integer",
                                "description": "Number of tweets to retrieve"
                            },
                            "exclude_replies":{
                                "type": "boolean",
                                "description": "Whether to exclude replies"
                            }
                        },
                        "required": ["count"]
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
                name: Cow::Borrowed("fetch_tweets_and_replies"),
                title: None,
                description: Some(Cow::Borrowed(
                    "Fetches tweets and replies for a specific user.",
                )),
                input_schema: Arc::new(
                    json!({
                        "type": "object",
                        "properties":{
                            "username":{
                                "type": "string",
                                "description": "The username for which to fetch tweets and replies"
                            },
                            "limit":{
                                "type": "integer",
                                "description": "Maximum number of tweets and replies to return"
                            },
                            "cursor":{
                                "type": "string",
                                "description": "Optional cursor for pagination"
                            }
                        },
                        "required": ["username", "limit"]
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
                name: Cow::Borrowed("search_profiles"),
                title: None,
                description: Some(Cow::Borrowed(
                    "Searches for user profiles matching a query.",
                )),
                input_schema: Arc::new(
                    json!({
                        "type": "object",
                        "properties":{
                            "query":{
                                "type": "string",
                                "description": "The search query for profiles"
                            },
                            "limit":{
                                "type": "integer",
                                "description": "Maximum number of profiles to return"
                            },
                            "cursor":{
                                "type": "string",
                                "description": "Optional cursor for pagination"
                            }
                        },
                        "required": ["query", "limit"]
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
                name: Cow::Borrowed("get_direct_message_conversations"),
                title: None,
                description: Some(Cow::Borrowed(
                    "Gets direct message conversations for the authenticated user.",
                )),
                input_schema: Arc::new(
                    json!({
                        "type": "object",
                        "properties":{
                            "user_id":{
                                "type": "string",
                                "description": "The user ID"
                            },
                            "cursor":{
                                "type": "string",
                                "description": "Optional cursor for pagination"
                            }
                        },
                        "required": ["user_id"]
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
                name: Cow::Borrowed("send_direct_message"),
                title: None,
                description: Some(Cow::Borrowed(
                    "Sends a direct message to a specified conversation.",
                )),
                input_schema: Arc::new(
                    json!({
                        "type": "object",
                        "properties":{
                            "conversation_id":{
                                "type": "string",
                                "description": "The ID of the conversation"
                            },
                            "text":{
                                "type": "string",
                                "description": "The text of the message"
                            }
                        },
                        "required": ["conversation_id", "text"]
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
        let name: &str = &request.name;
        let args = request.arguments.unwrap_or_default();
        match name {
            "get_profile" => {
                let username = args["username"]
                    .as_str()
                    .ok_or(ConnectorError::InvalidParams(
                        "Missing 'username' argument".to_string(),
                    ))?;
                // Strip "@" prefix if present
                let username = username.strip_prefix('@').unwrap_or(username);

                let profile: Profile = self
                    .scraper
                    .get_profile(username)
                    .await
                    .map_err(|e| ConnectorError::Other(e.to_string()))?;

                let text = serde_json::to_string(&profile)?;
                Ok(structured_result_with_text(&profile, Some(text))?)
            }
            "search_tweets" => {
                let query = args["query"].as_str().ok_or(ConnectorError::InvalidParams(
                    "Missing 'query' argument".to_string(),
                ))?;
                let limit = args["limit"].as_i64().unwrap_or(20) as i32; // Default

                let tweets: QueryTweetsResponse = self
                    .scraper
                    .search_tweets(query, limit, SearchMode::Latest, None)
                    .await
                    .map_err(|e| ConnectorError::Other(e.to_string()))?;

                let text = serde_json::to_string(&tweets)?;
                Ok(structured_result_with_text(&tweets, Some(text))?)
            }
            "get_followers" => {
                let username = args["username"]
                    .as_str()
                    .ok_or(ConnectorError::InvalidParams(
                        "Missing 'username' argument".to_string(),
                    ))?;
                // Strip "@" prefix if present
                let username = username.strip_prefix('@').unwrap_or(username);
                let limit = args["limit"].as_i64().unwrap_or(20) as i32;
                let cursor = args["cursor"].as_str().map(String::from);

                let (followers, next_cursor) = self
                    .scraper
                    .get_followers(username, limit, cursor)
                    .await
                    .map_err(|e| ConnectorError::Other(e.to_string()))?;
                let payload = json!({
                    "followers": followers,
                    "next_cursor": next_cursor,
                });
                let text = serde_json::to_string(&payload)?;
                Ok(structured_result_with_text(&payload, Some(text))?)
            }
            "get_tweet" => {
                let tweet_id = args["tweet_id"]
                    .as_str()
                    .ok_or(ConnectorError::InvalidParams(
                        "Missing 'tweet_id' parameter".to_string(),
                    ))?;
                let tweet: Tweet = self
                    .scraper
                    .get_tweet(tweet_id)
                    .await
                    .map_err(|e| ConnectorError::Other(e.to_string()))?;
                let text = serde_json::to_string(&tweet)?;
                Ok(structured_result_with_text(&tweet, Some(text))?)
            }
            "get_home_timeline" => {
                let count = args["count"].as_i64().unwrap_or(20) as i32;
                let exclude_replies: Vec<String> = match args["exclude_replies"].as_bool() {
                    Some(true) => vec!["rts".to_string(), "replies".to_string()],
                    _ => vec![],
                };
                let tweets: Vec<Value> = self
                    .scraper
                    .get_home_timeline(count, exclude_replies)
                    .await
                    .map_err(|e| ConnectorError::Other(e.to_string()))?;
                let text = serde_json::to_string(&tweets)?;
                Ok(structured_result_with_text(&tweets, Some(text))?)
            }
            "fetch_tweets_and_replies" => {
                let username = args["username"]
                    .as_str()
                    .ok_or(ConnectorError::InvalidParams(
                        "Missing 'username' argument".to_string(),
                    ))?;
                // Strip "@" prefix if present
                let username = username.strip_prefix('@').unwrap_or(username);
                let limit = args["limit"].as_i64().unwrap_or(20) as i32;
                let cursor = args["cursor"].as_str();
                let tweets: V2QueryTweetsResponse = self
                    .scraper
                    .fetch_tweets_and_replies(username, limit, cursor)
                    .await
                    .map_err(|e| ConnectorError::Other(e.to_string()))?;
                let text = serde_json::to_string(&tweets)?;
                Ok(structured_result_with_text(&tweets, Some(text))?)
            }
            "search_profiles" => {
                let query = args["query"].as_str().ok_or(ConnectorError::InvalidParams(
                    "Missing 'query' argument".to_string(),
                ))?;
                let limit = args["limit"].as_i64().unwrap_or(20) as i32;
                let cursor = args["cursor"].as_str().map(String::from);

                let profiles: QueryProfilesResponse = self
                    .scraper
                    .search_profiles(query, limit, cursor)
                    .await
                    .map_err(|e| ConnectorError::Other(e.to_string()))?;
                let text = serde_json::to_string(&profiles)?;
                Ok(structured_result_with_text(&profiles, Some(text))?)
            }
            "get_direct_message_conversations" => {
                let user_id = args["user_id"]
                    .as_str()
                    .ok_or(ConnectorError::InvalidParams(
                        "Missing 'user_id' argument".to_string(),
                    ))?;
                let cursor = args["cursor"].as_str();
                let conversations = self
                    .scraper
                    .get_direct_message_conversations(user_id, cursor)
                    .await
                    .map_err(|e| ConnectorError::Other(e.to_string()))?;
                let text = serde_json::to_string(&conversations)?;
                Ok(structured_result_with_text(&conversations, Some(text))?)
            }
            "send_direct_message" => {
                let conversation_id =
                    args["conversation_id"]
                        .as_str()
                        .ok_or(ConnectorError::InvalidParams(
                            "Missing 'conversation_id' parameter".to_string(),
                        ))?;

                let text = args["text"].as_str().ok_or(ConnectorError::InvalidParams(
                    "Missing 'text' parameter".to_string(),
                ))?;
                self.scraper
                    .send_direct_message(conversation_id, text)
                    .await
                    .map_err(|e| ConnectorError::Other(e.to_string()))?;
                let payload = json!({
                    "status": "sent",
                    "message": "Direct message sent successfully.",
                });
                let serialized = serde_json::to_string(&payload)?;
                Ok(structured_result_with_text(&payload, Some(serialized))?)
            }
            _ => Err(ConnectorError::ToolNotFound),
        }
    }
    async fn list_prompts(
        &self,
        request: Option<PaginatedRequestParam>,
    ) -> Result<ListPromptsResult, ConnectorError> {
        let _cursor = request.and_then(|r| r.cursor);
        let prompts = vec![Prompt {
            name: "summarize_user_tweets".to_string(),
            title: None,
            description: Some("Summarizes the recent tweets of a given user.".to_string()),
            arguments: Some(vec![PromptArgument {
                name: "username".to_string(),
                title: None,
                description: Some("Twitter username for which to summarize tweets".to_string()),
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
            "summarize_user_tweets" => {
                //Does not make sense to retrieve tweets here, we should probably inject them.
                let prompt = Prompt{
                    name: "summarize_user_tweets".to_string(),
                    title: None,
                    description: Some("Given the provided tweets, generate a concise summary highlighting the main topics, sentiments, and key information conveyed by the user.".to_string()),
                    arguments: Some(vec![
                        PromptArgument{
                            name: "username".to_string(),
                            title: None,
                            description: Some("Twitter username for which to summarize tweets".to_string()),
                            required: Some(true)
                        }
                    ]),
                    icons: None,
                };
                Ok(prompt)
            }
            _ => Err(ConnectorError::InvalidParams(format!(
                "Prompt with name {} not found",
                name
            ))),
        }
    }
}
