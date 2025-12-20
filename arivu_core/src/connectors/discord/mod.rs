use crate::capabilities::{ConnectorConfigSchema, Field, FieldType};
use crate::error::ConnectorError;
use crate::utils::structured_result_with_text;
use crate::{auth::AuthDetails, Connector};
use async_trait::async_trait;
use rmcp::model::*;
use serde::Deserialize;
use serde_json::{json, Value};
use serenity::http::Http;
use serenity::model::id::{ChannelId, GuildId};
use std::borrow::Cow;
use std::sync::Arc;

#[derive(Debug, Deserialize)]
struct ReadMessagesArgs {
    channel_id: u64,
    limit: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct SendMessageArgs {
    channel_id: u64,
    content: String,
}

#[derive(Debug, Deserialize)]
struct ListChannelsArgs {
    guild_id: u64,
}

#[derive(Debug, Deserialize)]
struct GetServerInfoArgs {
    guild_id: u64,
}

#[derive(Debug, Deserialize)]
struct SearchMessagesArgs {
    channel_id: u64,
    query: String,
    limit: Option<u64>,
}

pub struct DiscordConnector {
    http: Option<Arc<Http>>,
    token: Option<String>,
}

impl DiscordConnector {
    pub async fn new(auth: AuthDetails) -> Result<Self, ConnectorError> {
        let mut connector = Self {
            http: None,
            token: None,
        };
        if !auth.is_empty() {
            connector.set_auth_details(auth).await?;
        }
        Ok(connector)
    }

    fn get_http(&self) -> Result<&Arc<Http>, ConnectorError> {
        self.http.as_ref().ok_or(ConnectorError::Authentication(
            "Discord token not provided".to_string(),
        ))
    }
}

#[async_trait]
impl Connector for DiscordConnector {
    fn name(&self) -> &'static str {
        "discord"
    }

    fn description(&self) -> &'static str {
        "Interact with Discord servers, channels, and messages"
    }

    async fn capabilities(&self) -> ServerCapabilities {
        ServerCapabilities {
            tools: None,
            ..Default::default()
        }
    }

    async fn get_auth_details(&self) -> Result<AuthDetails, ConnectorError> {
        let mut auth = AuthDetails::new();
        if let Some(token) = &self.token {
            auth.insert("token".to_string(), token.clone());
        }
        Ok(auth)
    }

    async fn set_auth_details(&mut self, details: AuthDetails) -> Result<(), ConnectorError> {
        if let Some(token) = details.get("token").or(details.get("bot_token")) {
            self.token = Some(token.clone());
            self.http = Some(Arc::new(Http::new(token)));
            Ok(())
        } else {
            // Maybe it's in env?
            if let Ok(token) = std::env::var("DISCORD_TOKEN") {
                self.token = Some(token.clone());
                self.http = Some(Arc::new(Http::new(&token)));
                Ok(())
            } else {
                Err(ConnectorError::Authentication(
                    "Missing 'token' in auth details".to_string(),
                ))
            }
        }
    }

    async fn test_auth(&self) -> Result<(), ConnectorError> {
        let http = self.get_http()?;
        match http.get_current_user().await {
            Ok(_) => Ok(()),
            Err(e) => Err(ConnectorError::Authentication(format!(
                "Auth failed: {}",
                e
            ))),
        }
    }

    fn config_schema(&self) -> ConnectorConfigSchema {
        ConnectorConfigSchema {
            fields: vec![Field {
                name: "token".to_string(),
                label: "Bot Token".to_string(),
                field_type: FieldType::Secret,
                required: true,
                description: Some("Discord Bot Token".to_string()),
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
            instructions: Some("Access Discord. Requires a Bot Token and MESSAGE_CONTENT intent enabled in Discord Developer Portal for reading message content.".to_string()),
        })
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParam>,
    ) -> Result<ListToolsResult, ConnectorError> {
        let tools = vec![
            Tool {
                name: Cow::Borrowed("list_servers"),
                title: None,
                description: Some(Cow::Borrowed("List servers (guilds) for the bot.")),
                input_schema: Arc::new(json!({
                    "type": "object",
                    "properties": {},
                }).as_object().expect("Schema object").clone()),
                output_schema: None,
                annotations: None,
                icons: None,
            },
            Tool {
                name: Cow::Borrowed("get_server_info"),
                title: None,
                description: Some(Cow::Borrowed("Server details by guild_id.")),
                input_schema: Arc::new(json!({
                    "type": "object",
                    "properties": {
                        "guild_id": { "type": "integer", "description": "ID of the server/guild" }
                    },
                    "required": ["guild_id"]
                }).as_object().expect("Schema object").clone()),
                output_schema: None,
                annotations: None,
                icons: None,
            },
            Tool {
                name: Cow::Borrowed("list_channels"),
                title: None,
                description: Some(Cow::Borrowed("List channels in a server.")),
                input_schema: Arc::new(json!({
                    "type": "object",
                    "properties": {
                        "guild_id": { "type": "integer", "description": "ID of the server/guild" }
                    },
                    "required": ["guild_id"]
                }).as_object().expect("Schema object").clone()),
                output_schema: None,
                annotations: None,
                icons: None,
            },
            Tool {
                name: Cow::Borrowed("read_messages"),
                title: None,
                description: Some(Cow::Borrowed("Read recent messages in a channel.")),
                input_schema: Arc::new(json!({
                    "type": "object",
                    "properties": {
                        "channel_id": { "type": "integer", "description": "ID of the channel" },
                        "limit": { "type": "integer", "description": "Number of messages (max 100)" }
                    },
                    "required": ["channel_id"]
                }).as_object().expect("Schema object").clone()),
                output_schema: None,
                annotations: None,
                icons: None,
            },
            Tool {
                name: Cow::Borrowed("send_message"),
                title: None,
                description: Some(Cow::Borrowed("Send a message to a channel.")),
                input_schema: Arc::new(json!({
                    "type": "object",
                    "properties": {
                        "channel_id": { "type": "integer", "description": "ID of the channel" },
                        "content": { "type": "string", "description": "Message content" }
                    },
                    "required": ["channel_id", "content"]
                }).as_object().expect("Schema object").clone()),
                output_schema: None,
                annotations: None,
                icons: None,
            },
            Tool {
                name: Cow::Borrowed("search_messages"),
                title: None,
                description: Some(Cow::Borrowed("Search messages in a channel.")),
                input_schema: Arc::new(json!({
                    "type": "object",
                    "properties": {
                        "channel_id": { "type": "integer", "description": "ID of the channel" },
                        "query": { "type": "string", "description": "Text to search for within message content" },
                        "limit": { "type": "integer", "description": "Number of matching messages (max 100)" }
                    },
                    "required": ["channel_id", "query"]
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
        let http = self.get_http()?;

        match request.name.as_ref() {
            "list_servers" => {
                let guilds = http
                    .get_guilds(None, None)
                    .await
                    .map_err(|e| ConnectorError::Other(e.to_string()))?;
                let data: Vec<Value> = guilds
                    .iter()
                    .map(|g| {
                        json!({
                            "id": g.id.get(),
                            "name": g.name,
                        })
                    })
                    .collect();

                Ok(structured_result_with_text(
                    &data,
                    Some(serde_json::to_string(&data)?),
                )?)
            }
            "get_server_info" => {
                let args: GetServerInfoArgs = serde_json::from_value(
                    serde_json::to_value(request.arguments.unwrap_or_default())
                        .map_err(ConnectorError::SerdeJson)?,
                )
                .map_err(|e| ConnectorError::InvalidParams(e.to_string()))?;

                let guild = http
                    .get_guild(GuildId::new(args.guild_id))
                    .await
                    .map_err(|e| ConnectorError::Other(e.to_string()))?;

                let data = json!({
                    "id": guild.id.get(),
                    "name": guild.name,
                    "description": guild.description,
                    "member_count": guild.approximate_member_count,
                    "owner_id": guild.owner_id.get(),
                });
                Ok(structured_result_with_text(
                    &data,
                    Some(serde_json::to_string(&data)?),
                )?)
            }
            "list_channels" => {
                let args: ListChannelsArgs = serde_json::from_value(
                    serde_json::to_value(request.arguments.unwrap_or_default())
                        .map_err(ConnectorError::SerdeJson)?,
                )
                .map_err(|e| ConnectorError::InvalidParams(e.to_string()))?;

                let channels = http
                    .get_channels(GuildId::new(args.guild_id))
                    .await
                    .map_err(|e| ConnectorError::Other(e.to_string()))?;

                let data: Vec<Value> = channels
                    .iter()
                    .map(|c| {
                        json!({
                            "id": c.id.get(),
                            "name": c.name,
                            "type": format!("{:?}", c.kind),
                        })
                    })
                    .collect();
                Ok(structured_result_with_text(
                    &data,
                    Some(serde_json::to_string(&data)?),
                )?)
            }
            "read_messages" => {
                let args: ReadMessagesArgs = serde_json::from_value(
                    serde_json::to_value(request.arguments.unwrap_or_default())
                        .map_err(ConnectorError::SerdeJson)?,
                )
                .map_err(|e| ConnectorError::InvalidParams(e.to_string()))?;

                let messages = http
                    .get_messages(
                        ChannelId::new(args.channel_id),
                        None,
                        Some(args.limit.unwrap_or(50).min(100) as u8),
                    )
                    .await
                    .map_err(|e| ConnectorError::Other(e.to_string()))?;

                let data: Vec<Value> = messages
                    .iter()
                    .map(|m| {
                        json!({
                            "id": m.id.get(),
                            "author": m.author.name,
                            "content": m.content,
                            "timestamp": m.timestamp.to_rfc3339(),
                        })
                    })
                    .collect();
                Ok(structured_result_with_text(
                    &data,
                    Some(serde_json::to_string(&data)?),
                )?)
            }
            "send_message" => {
                let args: SendMessageArgs = serde_json::from_value(
                    serde_json::to_value(request.arguments.unwrap_or_default())
                        .map_err(ConnectorError::SerdeJson)?,
                )
                .map_err(|e| ConnectorError::InvalidParams(e.to_string()))?;

                let map = json!({ "content": args.content });
                // Serenity 0.12: send_message(channel_id, files, map)
                let msg = http
                    .send_message(
                        ChannelId::new(args.channel_id),
                        Vec::<serenity::builder::CreateAttachment>::new(),
                        &map,
                    )
                    .await
                    .map_err(|e| ConnectorError::Other(e.to_string()))?;

                let data = json!({
                    "id": msg.id.get(),
                    "content": msg.content,
                    "status": "sent"
                });
                Ok(structured_result_with_text(
                    &data,
                    Some(serde_json::to_string(&data)?),
                )?)
            }
            "search_messages" => {
                let args: SearchMessagesArgs = serde_json::from_value(
                    serde_json::to_value(request.arguments.unwrap_or_default())
                        .map_err(ConnectorError::SerdeJson)?,
                )
                .map_err(|e| ConnectorError::InvalidParams(e.to_string()))?;

                let limit = args.limit.unwrap_or(50).min(100) as u8;
                let messages = http
                    .get_messages(ChannelId::new(args.channel_id), None, Some(limit))
                    .await
                    .map_err(|e| ConnectorError::Other(e.to_string()))?;

                let query_lower = args.query.to_lowercase();
                let filtered_messages: Vec<Value> = messages
                    .iter()
                    .filter(|m| m.content.to_lowercase().contains(&query_lower))
                    .map(|m| {
                        json!({
                            "id": m.id.get(),
                            "author": m.author.name,
                            "content": m.content,
                            "timestamp": m.timestamp.to_rfc3339(),
                        })
                    })
                    .collect();
                Ok(structured_result_with_text(
                    &json!({"query": args.query, "results": filtered_messages}),
                    Some(serde_json::to_string(
                        &json!({"query": args.query, "results": filtered_messages}),
                    )?),
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
