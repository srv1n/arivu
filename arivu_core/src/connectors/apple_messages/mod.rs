// Apple Messages Connector - Native Messages.app integration via AppleScript
// macOS only - interact with iMessage and SMS conversations
//
// Note: Messages.app has limited AppleScript support for privacy reasons.
// - Sending messages: Fully supported
// - Reading conversations: Limited (basic chat listing, requires Full Disk Access for history)
// - The Messages SQLite database at ~/Library/Messages/chat.db contains full history
//   but requires special permissions to access.

#[cfg(target_os = "macos")]
use crate::connectors::apple_common::{
    apple_connector_capabilities, escape_applescript_string, run_applescript_output,
};
use crate::error::ConnectorError;
use crate::utils::structured_result_with_text;
use async_trait::async_trait;
use rmcp::model::*;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::borrow::Cow;
use std::sync::Arc;

/// Apple Messages connector - interact with Messages.app via AppleScript
#[derive(Default)]
pub struct AppleMessagesConnector;

impl AppleMessagesConnector {
    pub fn new() -> Self {
        Self {}
    }
}

// ============================================================================
// Data Structures
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
struct ChatInfo {
    /// Participant phone numbers/emails (use with get_recent_messages)
    participants: String,
    /// Service type (iMessage, SMS)
    service: String,
    /// Internal chat ID (use with send_to_chat)
    chat_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct SendMessageResult {
    success: bool,
    message: String,
    recipient: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ChatParticipant {
    /// Participant ID (phone/email)
    id: String,
    /// Display name if available
    name: Option<String>,
}

// ============================================================================
// AppleScript Generators
// ============================================================================

#[cfg(target_os = "macos")]
#[allow(dead_code)]
fn script_list_chats() -> String {
    r#"
tell application "Messages"
    set output to ""
    repeat with c in chats
        set chatId to ""
        set chatService to ""
        set participantList to ""
        try
            set chatId to id of c as text
        end try
        try
            set rawService to name of service of c
            if rawService is not missing value then
                set chatService to rawService as text
            end if
        end try
        -- Get participants (phone numbers/emails) - this is what users need
        try
            set chatParticipants to participants of c
            repeat with p in chatParticipants
                try
                    set pHandle to handle of p as text
                    if pHandle is not missing value and pHandle is not "" then
                        if participantList is "" then
                            set participantList to pHandle
                        else
                            set participantList to participantList & ", " & pHandle
                        end if
                    end if
                end try
            end repeat
        end try
        if chatId is not "" then
            if output is not "" then set output to output & "|||"
            set output to output & participantList & ":::" & chatService & ":::" & chatId
        end if
    end repeat
    return output
end tell
"#
    .to_string()
}

#[cfg(target_os = "macos")]
fn script_send_message(recipient: &str, message: &str) -> String {
    format!(
        r#"
tell application "Messages"
    set targetService to 1st service whose service type = iMessage
    set targetBuddy to buddy "{}" of targetService
    send "{}" to targetBuddy
    return "Message sent successfully"
end tell
"#,
        escape_applescript_string(recipient),
        escape_applescript_string(message)
    )
}

#[cfg(target_os = "macos")]
fn script_send_to_chat(chat_id: &str, message: &str) -> String {
    format!(
        r#"
tell application "Messages"
    set targetChat to chat id "{}"
    send "{}" to targetChat
    return "Message sent successfully"
end tell
"#,
        escape_applescript_string(chat_id),
        escape_applescript_string(message)
    )
}

#[cfg(target_os = "macos")]
fn script_get_chat_participants(chat_id: &str) -> String {
    format!(
        r#"
tell application "Messages"
    set targetChat to chat id "{}"
    set output to ""
    repeat with p in participants of targetChat
        set pId to ""
        set pName to ""
        try
            set rawId to id of p
            if rawId is not missing value then
                set pId to rawId as text
            end if
        end try
        try
            set rawName to name of p
            if rawName is not missing value then
                set pName to rawName as text
            end if
        end try
        if pId is not "" then
            if output is not "" then set output to output & "|||"
            set output to output & pId & ":::" & pName
        end if
    end repeat
    return output
end tell
"#,
        escape_applescript_string(chat_id)
    )
}

#[cfg(target_os = "macos")]
fn script_start_new_chat(recipient: &str, message: &str) -> String {
    format!(
        r#"
tell application "Messages"
    set targetService to 1st service whose service type = iMessage
    set targetBuddy to buddy "{}" of targetService
    send "{}" to targetBuddy
    return "Chat started and message sent"
end tell
"#,
        escape_applescript_string(recipient),
        escape_applescript_string(message)
    )
}

/// Chat listing with last message preview
#[derive(Debug, Serialize, Deserialize)]
struct ChatListing {
    /// Use this to get messages: get_recent_messages --chat_identifier "..."
    chat_identifier: String,
    /// Contact name (if in contacts)
    display_name: String,
    /// iMessage or SMS
    service: String,
    /// Preview of last message
    last_message: String,
    /// When last message was sent
    last_message_date: String,
}

// List chats from SQLite database - more reliable than AppleScript
#[cfg(target_os = "macos")]
async fn list_chats_from_db(limit: usize) -> Result<Vec<ChatListing>, ConnectorError> {
    use tokio::process::Command;

    let db_path = dirs::home_dir()
        .ok_or_else(|| ConnectorError::Other("Cannot find home directory".to_string()))?
        .join("Library/Messages/chat.db");

    if !db_path.exists() {
        return Err(ConnectorError::Other(
            "Messages database not found. Make sure Messages.app has been used.".to_string(),
        ));
    }

    // Get chats with their last message (truncated to 80 chars for preview)
    let query = format!(
        r#"
        SELECT
            c.chat_identifier,
            COALESCE(c.display_name, '') as display_name,
            COALESCE(c.service_name, '') as service_name,
            COALESCE(substr((SELECT m.text FROM message m
             JOIN chat_message_join cmj ON m.rowid = cmj.message_id
             WHERE cmj.chat_id = c.rowid
             ORDER BY m.date DESC LIMIT 1), 1, 80), '') as last_message,
            COALESCE((SELECT datetime(m.date/1000000000 + 978307200, 'unixepoch', 'localtime')
             FROM message m
             JOIN chat_message_join cmj ON m.rowid = cmj.message_id
             WHERE cmj.chat_id = c.rowid
             ORDER BY m.date DESC LIMIT 1), '') as last_message_date
        FROM chat c
        WHERE c.chat_identifier IS NOT NULL
        ORDER BY (SELECT m.date FROM message m
                  JOIN chat_message_join cmj ON m.rowid = cmj.message_id
                  WHERE cmj.chat_id = c.rowid
                  ORDER BY m.date DESC LIMIT 1) DESC
        LIMIT {}
        "#,
        limit
    );

    let output = Command::new("sqlite3")
        .arg("-json")
        .arg(&db_path)
        .arg(&query)
        .output()
        .await
        .map_err(|e| ConnectorError::Other(format!("Failed to query messages database: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("unable to open database") || stderr.contains("authorization denied") {
            return Err(ConnectorError::Other(
                "Cannot access Messages database. Grant Full Disk Access to your terminal/app in System Preferences > Security & Privacy > Privacy > Full Disk Access."
                    .to_string(),
            ));
        }
        return Err(ConnectorError::Other(format!(
            "Database query failed: {}",
            stderr
        )));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    if stdout.trim().is_empty() {
        return Ok(Vec::new());
    }

    let raw_chats: Vec<serde_json::Value> = serde_json::from_str(&stdout)
        .map_err(|e| ConnectorError::Other(format!("Failed to parse chats: {}", e)))?;

    // Convert to our struct for better display
    let chats: Vec<ChatListing> = raw_chats
        .into_iter()
        .map(|c| ChatListing {
            chat_identifier: c["chat_identifier"].as_str().unwrap_or("").to_string(),
            display_name: c["display_name"].as_str().unwrap_or("").to_string(),
            service: c["service_name"].as_str().unwrap_or("").to_string(),
            last_message: c["last_message"].as_str().unwrap_or("").to_string(),
            last_message_date: c["last_message_date"].as_str().unwrap_or("").to_string(),
        })
        .collect();

    Ok(chats)
}

// For reading message history, we need to access the SQLite database
// This requires Full Disk Access permission
#[cfg(target_os = "macos")]
async fn read_recent_messages_from_db(
    chat_identifier: Option<&str>,
    limit: usize,
) -> Result<Vec<serde_json::Value>, ConnectorError> {
    use tokio::process::Command;

    let db_path = dirs::home_dir()
        .ok_or_else(|| ConnectorError::Other("Cannot find home directory".to_string()))?
        .join("Library/Messages/chat.db");

    if !db_path.exists() {
        return Err(ConnectorError::Other(
            "Messages database not found. Make sure Messages.app has been used.".to_string(),
        ));
    }

    let query = match chat_identifier {
        Some(chat) => format!(
            r#"
            SELECT
                m.rowid as id,
                m.text,
                datetime(m.date/1000000000 + 978307200, 'unixepoch', 'localtime') as date,
                m.is_from_me,
                h.id as sender
            FROM message m
            LEFT JOIN handle h ON m.handle_id = h.rowid
            LEFT JOIN chat_message_join cmj ON m.rowid = cmj.message_id
            LEFT JOIN chat c ON cmj.chat_id = c.rowid
            WHERE c.chat_identifier = '{}'
            ORDER BY m.date DESC
            LIMIT {}
            "#,
            chat.replace('\'', "''"),
            limit
        ),
        None => format!(
            r#"
            SELECT
                m.rowid as id,
                m.text,
                datetime(m.date/1000000000 + 978307200, 'unixepoch', 'localtime') as date,
                m.is_from_me,
                h.id as sender,
                c.chat_identifier
            FROM message m
            LEFT JOIN handle h ON m.handle_id = h.rowid
            LEFT JOIN chat_message_join cmj ON m.rowid = cmj.message_id
            LEFT JOIN chat c ON cmj.chat_id = c.rowid
            WHERE m.text IS NOT NULL
            ORDER BY m.date DESC
            LIMIT {}
            "#,
            limit
        ),
    };

    let output = Command::new("sqlite3")
        .arg("-json")
        .arg(&db_path)
        .arg(&query)
        .output()
        .await
        .map_err(|e| ConnectorError::Other(format!("Failed to query messages database: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("unable to open database") || stderr.contains("authorization denied") {
            return Err(ConnectorError::Other(
                "Cannot access Messages database. Grant Full Disk Access to your terminal/app in System Preferences > Security & Privacy > Privacy > Full Disk Access."
                    .to_string(),
            ));
        }
        return Err(ConnectorError::Other(format!(
            "Database query failed: {}",
            stderr
        )));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    if stdout.trim().is_empty() {
        return Ok(Vec::new());
    }

    let messages: Vec<serde_json::Value> = serde_json::from_str(&stdout)
        .map_err(|e| ConnectorError::Other(format!("Failed to parse messages: {}", e)))?;

    Ok(messages)
}

// ============================================================================
// Parsing Functions
// ============================================================================

#[cfg(target_os = "macos")]
#[allow(dead_code)]
fn parse_chats(output: &str) -> Vec<ChatInfo> {
    output
        .split("|||")
        .filter(|s| !s.is_empty())
        .filter_map(|entry| {
            let parts: Vec<&str> = entry.split(":::").collect();
            if parts.len() >= 3 {
                Some(ChatInfo {
                    participants: parts[0].to_string(),
                    service: parts[1].to_string(),
                    chat_id: parts[2].to_string(),
                })
            } else {
                None
            }
        })
        .collect()
}

#[cfg(target_os = "macos")]
fn parse_participants(output: &str) -> Vec<ChatParticipant> {
    output
        .split("|||")
        .filter(|s| !s.is_empty())
        .filter_map(|entry| {
            let parts: Vec<&str> = entry.split(":::").collect();
            if parts.len() >= 2 {
                Some(ChatParticipant {
                    id: parts[0].to_string(),
                    name: if parts[1].is_empty() {
                        None
                    } else {
                        Some(parts[1].to_string())
                    },
                })
            } else {
                None
            }
        })
        .collect()
}

// ============================================================================
// Connector Implementation
// ============================================================================

#[async_trait]
impl crate::Connector for AppleMessagesConnector {
    fn name(&self) -> &'static str {
        "apple-messages"
    }

    fn description(&self) -> &'static str {
        "Apple Messages.app connector for macOS. Send iMessages and SMS. Read conversation history (requires Full Disk Access for message history). Works with phone numbers and email addresses."
    }

    async fn capabilities(&self) -> ServerCapabilities {
        #[cfg(target_os = "macos")]
        {
            apple_connector_capabilities()
        }
        #[cfg(not(target_os = "macos"))]
        {
            ServerCapabilities::default()
        }
    }

    async fn get_auth_details(&self) -> Result<crate::auth::AuthDetails, ConnectorError> {
        Ok(crate::auth::AuthDetails::new())
    }

    async fn set_auth_details(
        &mut self,
        _details: crate::auth::AuthDetails,
    ) -> Result<(), ConnectorError> {
        Ok(())
    }

    async fn test_auth(&self) -> Result<(), ConnectorError> {
        #[cfg(target_os = "macos")]
        {
            let _ = run_applescript_output(r#"tell application "Messages" to name"#).await?;
            Ok(())
        }
        #[cfg(not(target_os = "macos"))]
        {
            Err(ConnectorError::Other(
                "Apple Messages is only available on macOS".to_string(),
            ))
        }
    }

    fn config_schema(&self) -> crate::capabilities::ConnectorConfigSchema {
        crate::capabilities::ConnectorConfigSchema { fields: vec![] }
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
                title: Some("Apple Messages".to_string()),
                version: env!("CARGO_PKG_VERSION").to_string(),
                icons: None,
                website_url: None,
            },
            instructions: Some(
                "Native Messages.app integration for iMessage and SMS. First use triggers permission prompts. Message history requires Full Disk Access."
                    .to_string(),
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
            // Chat Management
            Tool {
                name: Cow::Borrowed("list_chats"),
                title: Some("List Chats".to_string()),
                description: Some(Cow::Borrowed(
                    "List recent chat conversations sorted by last message time. Shows chat_identifier (phone/email), display_name, last_message preview, and last_message_date. Use chat_identifier with get_recent_messages to see full conversation. REQUIRES Full Disk Access.",
                )),
                input_schema: Arc::new(
                    json!({
                        "type": "object",
                        "properties": {
                            "limit": {
                                "type": "integer",
                                "description": "Maximum chats to return. Default: 20.",
                                "default": 20
                            }
                        }
                    })
                    .as_object()
                    .unwrap()
                    .clone(),
                ),
                output_schema: None,
                annotations: None,
                icons: None,
            },
            Tool {
                name: Cow::Borrowed("get_chat_participants"),
                title: Some("Get Chat Participants".to_string()),
                description: Some(Cow::Borrowed(
                    "Get participants in a group chat. Returns IDs (phone/email) and display names. Use chat_id from list_chats.",
                )),
                input_schema: Arc::new(
                    json!({
                        "type": "object",
                        "properties": {
                            "chat_id": {
                                "type": "string",
                                "description": "Chat ID obtained from list_chats. Required."
                            }
                        },
                        "required": ["chat_id"]
                    })
                    .as_object()
                    .unwrap()
                    .clone(),
                ),
                output_schema: None,
                annotations: None,
                icons: None,
            },
            // Reading Messages (requires Full Disk Access)
            Tool {
                name: Cow::Borrowed("get_recent_messages"),
                title: Some("Get Recent Messages".to_string()),
                description: Some(Cow::Borrowed(
                    "Read recent message history from Messages database. REQUIRES Full Disk Access permission granted to your terminal/app. Returns message text, sender, date, and direction (sent/received).",
                )),
                input_schema: Arc::new(
                    json!({
                        "type": "object",
                        "properties": {
                            "chat_identifier": {
                                "type": "string",
                                "description": "Filter to specific chat (phone number like +1234567890 or email). If omitted, returns messages from all chats."
                            },
                            "limit": {
                                "type": "integer",
                                "description": "Maximum messages to return. Default: 50.",
                                "default": 50
                            }
                        }
                    })
                    .as_object()
                    .unwrap()
                    .clone(),
                ),
                output_schema: None,
                annotations: None,
                icons: None,
            },
            // Sending Messages
            Tool {
                name: Cow::Borrowed("send_message"),
                title: Some("Send Message".to_string()),
                description: Some(Cow::Borrowed(
                    "Send an iMessage or SMS to a recipient. Use phone number (+1234567890) or email address. The recipient must be in your contacts or have an existing conversation for best results.",
                )),
                input_schema: Arc::new(
                    json!({
                        "type": "object",
                        "properties": {
                            "recipient": {
                                "type": "string",
                                "description": "Phone number (with country code, e.g., +1234567890) or iMessage email address. Required."
                            },
                            "message": {
                                "type": "string",
                                "description": "Message text to send. Required."
                            }
                        },
                        "required": ["recipient", "message"]
                    })
                    .as_object()
                    .unwrap()
                    .clone(),
                ),
                output_schema: None,
                annotations: None,
                icons: None,
            },
            Tool {
                name: Cow::Borrowed("send_to_chat"),
                title: Some("Send to Chat".to_string()),
                description: Some(Cow::Borrowed(
                    "Send a message to an existing chat by chat ID. Useful for group chats. Get chat_id from list_chats.",
                )),
                input_schema: Arc::new(
                    json!({
                        "type": "object",
                        "properties": {
                            "chat_id": {
                                "type": "string",
                                "description": "Chat ID from list_chats. Required."
                            },
                            "message": {
                                "type": "string",
                                "description": "Message text to send. Required."
                            }
                        },
                        "required": ["chat_id", "message"]
                    })
                    .as_object()
                    .unwrap()
                    .clone(),
                ),
                output_schema: None,
                annotations: None,
                icons: None,
            },
            Tool {
                name: Cow::Borrowed("start_new_chat"),
                title: Some("Start New Chat".to_string()),
                description: Some(Cow::Borrowed(
                    "Start a new conversation with a recipient and send the first message. Creates the chat if it doesn't exist.",
                )),
                input_schema: Arc::new(
                    json!({
                        "type": "object",
                        "properties": {
                            "recipient": {
                                "type": "string",
                                "description": "Phone number or iMessage email. Required."
                            },
                            "message": {
                                "type": "string",
                                "description": "Initial message to send. Required."
                            }
                        },
                        "required": ["recipient", "message"]
                    })
                    .as_object()
                    .unwrap()
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
        #[cfg(not(target_os = "macos"))]
        {
            let _ = request;
            return Err(ConnectorError::Other(
                "Apple Messages is only available on macOS".to_string(),
            ));
        }

        #[cfg(target_os = "macos")]
        {
            let name = request.name.as_ref();
            let args = request.arguments.unwrap_or_default();

            match name {
                "list_chats" => {
                    let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(20) as usize;
                    let chats = list_chats_from_db(limit).await?;

                    // ANSI color codes
                    let cyan = "\x1b[36m";
                    let green = "\x1b[32m";
                    let yellow = "\x1b[33m";
                    let dim = "\x1b[2m";
                    let reset = "\x1b[0m";
                    let bold = "\x1b[1m";

                    // Format as readable text with colors
                    let mut lines = Vec::new();
                    for (i, chat) in chats.iter().enumerate() {
                        let name_part = if !chat.display_name.is_empty() {
                            format!(" {green}{}{reset}", chat.display_name)
                        } else {
                            String::new()
                        };
                        let msg_preview = if !chat.last_message.is_empty() {
                            // Truncate to 60 chars for readability
                            let msg = chat.last_message.replace('\n', " ");
                            if msg.len() > 60 {
                                format!("{}...", &msg[..57])
                            } else {
                                msg
                            }
                        } else {
                            format!("{dim}(no message){reset}")
                        };
                        lines.push(format!(
                            "{bold}{:2}.{reset} {cyan}{}{reset}{} │ {} {dim}│ {}{reset}",
                            i + 1,
                            chat.chat_identifier,
                            name_part,
                            msg_preview,
                            chat.last_message_date
                        ));
                    }

                    let output = if chats.is_empty() {
                        json!({"message": "No chats found."})
                    } else {
                        json!({
                            "chats": lines,
                            "hint": format!("{yellow}Tip:{reset} Use get_recent_messages --chat_identifier \"<id>\" to view conversation")
                        })
                    };

                    structured_result_with_text(&output, None)
                }

                "get_chat_participants" => {
                    let chat_id =
                        args.get("chat_id")
                            .and_then(|v| v.as_str())
                            .ok_or_else(|| {
                                ConnectorError::InvalidParams("Missing 'chat_id'".to_string())
                            })?;

                    let output =
                        run_applescript_output(&script_get_chat_participants(chat_id)).await?;
                    let participants = parse_participants(&output);
                    structured_result_with_text(&participants, None)
                }

                "get_recent_messages" => {
                    let chat_identifier = args.get("chat_identifier").and_then(|v| v.as_str());
                    let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(50) as usize;

                    let messages = read_recent_messages_from_db(chat_identifier, limit).await?;
                    structured_result_with_text(&messages, None)
                }

                "send_message" => {
                    let recipient =
                        args.get("recipient")
                            .and_then(|v| v.as_str())
                            .ok_or_else(|| {
                                ConnectorError::InvalidParams("Missing 'recipient'".to_string())
                            })?;
                    let message =
                        args.get("message")
                            .and_then(|v| v.as_str())
                            .ok_or_else(|| {
                                ConnectorError::InvalidParams("Missing 'message'".to_string())
                            })?;

                    let output =
                        run_applescript_output(&script_send_message(recipient, message)).await?;
                    let result = SendMessageResult {
                        success: true,
                        message: output,
                        recipient: recipient.to_string(),
                    };
                    structured_result_with_text(&result, None)
                }

                "send_to_chat" => {
                    let chat_id =
                        args.get("chat_id")
                            .and_then(|v| v.as_str())
                            .ok_or_else(|| {
                                ConnectorError::InvalidParams("Missing 'chat_id'".to_string())
                            })?;
                    let message =
                        args.get("message")
                            .and_then(|v| v.as_str())
                            .ok_or_else(|| {
                                ConnectorError::InvalidParams("Missing 'message'".to_string())
                            })?;

                    let output =
                        run_applescript_output(&script_send_to_chat(chat_id, message)).await?;
                    structured_result_with_text(&json!({"success": true, "message": output}), None)
                }

                "start_new_chat" => {
                    let recipient =
                        args.get("recipient")
                            .and_then(|v| v.as_str())
                            .ok_or_else(|| {
                                ConnectorError::InvalidParams("Missing 'recipient'".to_string())
                            })?;
                    let message =
                        args.get("message")
                            .and_then(|v| v.as_str())
                            .ok_or_else(|| {
                                ConnectorError::InvalidParams("Missing 'message'".to_string())
                            })?;

                    let output =
                        run_applescript_output(&script_start_new_chat(recipient, message)).await?;
                    structured_result_with_text(
                        &json!({"success": true, "message": output, "recipient": recipient}),
                        None,
                    )
                }

                _ => Err(ConnectorError::ToolNotFound),
            }
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
        Err(ConnectorError::ResourceNotFound)
    }
}
