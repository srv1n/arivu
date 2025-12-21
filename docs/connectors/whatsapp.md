# WhatsApp — Design Spec

Status: Draft (Future Implementation)

## Overview

WhatsApp connector enables sending and receiving messages via WhatsApp's multi-device protocol. Unlike the paid WhatsApp Business API, this uses the same protocol as WhatsApp Web/Desktop through the open-source whatsmeow library.

**Key insight**: WhatsApp's multi-device protocol allows authenticated sessions without requiring the phone to be online. Projects like [Beeper](https://www.beeper.com/) (owned by Automattic) successfully use this approach.

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        User's Machine                           │
│                                                                 │
│  ┌─────────────┐      HTTP/REST      ┌──────────────────────┐  │
│  │             │ ──────────────────► │                      │  │
│  │   Arivu     │                     │   WuzAPI (Go)        │  │
│  │   (Rust)    │ ◄────────────────── │   - Session mgmt     │  │
│  │             │      Webhooks       │   - Message queue    │  │
│  └─────────────┘                     │   - SQLite state     │  │
│                                      └──────────┬───────────┘  │
│                                                 │               │
│                                                 │ WebSocket     │
│                                                 │ (encrypted)   │
└─────────────────────────────────────────────────┼───────────────┘
                                                  │
                                                  ▼
                                      ┌───────────────────────┐
                                      │  WhatsApp Servers     │
                                      │  (Signal Protocol)    │
                                      └───────────────────────┘
```

### Component Responsibilities

| Component | Role |
|-----------|------|
| **Arivu Connector** | Manages WuzAPI lifecycle, provides MCP tools, handles webhooks |
| **WuzAPI** | Go sidecar running whatsmeow, exposes REST API, persists sessions |
| **WhatsApp Servers** | Meta's infrastructure, E2E encrypted via Signal Protocol |

## Key Use Cases

- Send text messages to contacts/groups
- Send media (images, documents, audio, video)
- Receive messages via webhooks (for notifications/automation)
- Check message delivery/read status
- Manage group membership
- Search chat history (local)

## Integration Approach: WuzAPI Sidecar

### Why WuzAPI?

| Criteria | WuzAPI | whatsapp-rust |
|----------|--------|---------------|
| Maturity | 703★, 33 contributors | 236★, early v0.1.0 |
| Feature completeness | Full | Missing receipts, presence |
| Base library | whatsmeow (proven) | Port of whatsmeow |
| Production use | Widely used | Experimental |

### Sidecar Management

Arivu will manage the WuzAPI process lifecycle:

```rust
// Conceptual structure
pub struct WhatsAppConnector {
    wuzapi_process: Option<Child>,
    wuzapi_port: u16,
    session_webhook_port: u16,
    state_dir: PathBuf,
}

impl WhatsAppConnector {
    /// Start WuzAPI if not running
    async fn ensure_running(&mut self) -> Result<()>;

    /// Graceful shutdown
    async fn stop(&mut self) -> Result<()>;

    /// Health check
    async fn is_healthy(&self) -> bool;
}
```

### Session Management

WhatsApp requires QR code authentication on first connect:

1. User calls `whatsapp_connect` tool
2. Arivu starts WuzAPI if needed
3. WuzAPI returns QR code (base64 or terminal-renderable)
4. User scans with WhatsApp mobile app
5. Session persists in SQLite (survives restarts)

## MVP Scope (Tools)

### Connection & Auth

- `whatsapp_connect`: Initialize connection, return QR code if needed
  - Inputs: `{}`
  - Output: `{ status: "connected" | "qr_required", qr_code?: string }`

- `whatsapp_status`: Check connection status
  - Inputs: `{}`
  - Output: `{ connected: bool, phone_number?: string, battery?: int }`

- `whatsapp_disconnect`: Logout and clear session
  - Inputs: `{}`
  - Output: `{ success: bool }`

### Messaging

- `whatsapp_send_message`: Send text message
  - Inputs: `{ to: string, message: string }`
  - Output: `{ message_id: string, timestamp: string }`

- `whatsapp_send_media`: Send image/document/audio/video
  - Inputs: `{ to: string, file_path: string, caption?: string, media_type: "image" | "document" | "audio" | "video" }`
  - Output: `{ message_id: string }`

- `whatsapp_send_location`: Send location pin
  - Inputs: `{ to: string, latitude: f64, longitude: f64, name?: string }`
  - Output: `{ message_id: string }`

### Contacts & Groups

- `whatsapp_list_contacts`: List synced contacts
  - Inputs: `{ query?: string }`
  - Output: `{ contacts: [{ jid: string, name: string, phone: string }] }`

- `whatsapp_list_groups`: List group chats
  - Inputs: `{}`
  - Output: `{ groups: [{ jid: string, name: string, participant_count: int }] }`

- `whatsapp_get_group_info`: Get group details
  - Inputs: `{ group_jid: string }`
  - Output: `{ name: string, description: string, participants: [...] }`

### Chat History (Local)

- `whatsapp_get_messages`: Retrieve message history
  - Inputs: `{ chat_jid: string, limit?: int, before?: string }`
  - Output: `{ messages: [...] }`

## WuzAPI REST Endpoints

Reference: https://github.com/asternic/wuzapi

| Arivu Tool | WuzAPI Endpoint |
|------------|-----------------|
| `whatsapp_connect` | `POST /session/connect` |
| `whatsapp_status` | `GET /session/status` |
| `whatsapp_send_message` | `POST /chat/send/text` |
| `whatsapp_send_media` | `POST /chat/send/image`, `/document`, etc. |
| `whatsapp_list_contacts` | `GET /user/contacts` |
| `whatsapp_list_groups` | `GET /group/list` |

## Rust Crates / Deps

```toml
[dependencies]
reqwest = { version = "0.12", features = ["json"] }  # HTTP client for WuzAPI
tokio = { version = "1", features = ["process"] }    # Process management
serde = { version = "1", features = ["derive"] }
serde_json = "1"
base64 = "0.22"                                       # QR code handling
tempfile = "3"                                        # Media temp files
```

No Go dependencies in Arivu itself - WuzAPI is a separate binary.

## Data Model

### Message

```rust
pub struct WhatsAppMessage {
    pub id: String,
    pub chat_jid: String,           // recipient/group ID
    pub sender_jid: Option<String>, // None if sent by us
    pub content: MessageContent,
    pub timestamp: DateTime<Utc>,
    pub status: MessageStatus,      // sent, delivered, read
}

pub enum MessageContent {
    Text(String),
    Image { url: String, caption: Option<String> },
    Document { url: String, filename: String },
    Audio { url: String, duration_secs: u32 },
    Video { url: String, caption: Option<String> },
    Location { lat: f64, lon: f64, name: Option<String> },
}

pub enum MessageStatus {
    Pending,
    Sent,
    Delivered,
    Read,
    Failed(String),
}
```

### JID Format

WhatsApp uses Jabber IDs (JIDs):
- Individual: `1234567890@s.whatsapp.net`
- Group: `123456789-987654321@g.us`
- Broadcast: `status@broadcast`

## Error Handling & Limits

### Error Taxonomy

| Error | Handling |
|-------|----------|
| WuzAPI not running | Auto-start, retry |
| Session expired | Return QR code, require re-auth |
| Rate limited | Exponential backoff (WhatsApp limits ~200 msgs/day to new contacts) |
| Invalid JID | Return validation error |
| Media too large | Return size limit error (16MB images, 100MB video) |
| Network timeout | Retry with backoff |

### Rate Limits

WhatsApp enforces undocumented limits:
- ~200-250 messages/day to new contacts
- Higher limits for existing conversations
- Group messages count per-recipient

## Security & Privacy

### Encryption

- All messages are E2E encrypted via Signal Protocol
- WuzAPI handles encryption/decryption locally
- Keys never leave the user's device

### Session Security

- Session keys stored in local SQLite
- State directory should have restricted permissions (0700)
- Consider encryption-at-rest for session data

### Privacy Considerations

- Message history stored locally only
- No cloud sync (unlike iMessage)
- User controls what data Arivu can access

### Terms of Service Risk

> **Warning**: Using unofficial WhatsApp clients may violate Meta's ToS and could result in account suspension. Users should be aware of this risk.

Mitigation:
- Document the risk clearly
- Don't enable aggressive automation
- Respect rate limits
- Provide easy session cleanup

## Local vs Server

**This connector is local-only by design.**

| Aspect | Implementation |
|--------|----------------|
| WuzAPI process | Runs on user's machine |
| Session state | Local SQLite |
| Message history | Local only |
| Media files | Downloaded to local temp |
| Webhooks | Local HTTP server for incoming |

No server component required or desired.

## WuzAPI Deployment

### Option 1: Bundled Binary

```
arivu/
├── bin/
│   ├── wuzapi-darwin-arm64
│   ├── wuzapi-darwin-amd64
│   ├── wuzapi-linux-amd64
│   └── wuzapi-windows-amd64.exe
```

### Option 2: User-Installed

```bash
# User installs separately
go install github.com/asternic/wuzapi@latest

# Arivu finds it in PATH or config
arivu config set whatsapp.wuzapi_path /path/to/wuzapi
```

### Option 3: Docker (Advanced)

```bash
docker run -d -p 8080:8080 -v wuzapi_data:/data asternic/wuzapi
```

## Testing Plan

### Unit Tests

- JID parsing and validation
- Message serialization
- Error handling

### Integration Tests

- WuzAPI lifecycle management
- HTTP client against mock server
- Webhook reception

### Manual Testing (Required)

Since this accesses personal WhatsApp:
1. Build connector
2. Provide test commands to user
3. User runs with their account
4. User reports results

### Test Commands

```bash
# This connector is not implemented yet, so there is no CLI wrapper.
# When implemented, test via MCP `tools/call` (or a future `arivu whatsapp ...` CLI wrapper).

# Example MCP calls (JSON-RPC):
# {"jsonrpc":"2.0","method":"tools/call","params":{"name":"whatsapp_status","arguments":{}},"id":1}
# {"jsonrpc":"2.0","method":"tools/call","params":{"name":"whatsapp_connect","arguments":{}},"id":2}
# {"jsonrpc":"2.0","method":"tools/call","params":{"name":"whatsapp_send_message","arguments":{"to":"1234567890","message":"Test from Arivu"}},"id":3}
```

## Implementation Checklist

- [ ] Create `whatsapp` feature flag
- [ ] Implement WuzAPI process management
- [ ] Implement `whatsapp_connect` with QR code display
- [ ] Implement `whatsapp_status`
- [ ] Implement `whatsapp_send_message`
- [ ] Implement `whatsapp_send_media`
- [ ] Implement `whatsapp_list_contacts`
- [ ] Implement `whatsapp_list_groups`
- [ ] Implement `whatsapp_get_messages`
- [ ] Add webhook server for incoming messages
- [ ] Add session persistence and recovery
- [ ] Document ToS risks clearly
- [ ] Add to feature matrix and CLI help

## Future Enhancements (Post-MVP)

- [ ] Migrate to native Rust (whatsapp-rust) when mature
- [ ] Message search with full-text indexing
- [ ] Scheduled messages
- [ ] Auto-reply/chatbot integration
- [ ] Multi-account support
- [ ] Read receipt tracking
- [ ] Typing indicators

## References

- [whatsmeow](https://github.com/tulir/whatsmeow) - Go library (foundation)
- [WuzAPI](https://github.com/asternic/wuzapi) - REST API wrapper
- [whatsapp-rust](https://github.com/jlucaso1/whatsapp-rust) - Rust port (future)
- [mautrix-whatsapp](https://github.com/mautrix/whatsapp) - Matrix bridge
- [Beeper](https://www.beeper.com/) - Commercial use case
