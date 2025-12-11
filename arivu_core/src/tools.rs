use std::collections::HashMap;
use std::sync::Arc;

use crate::{
    auth::AuthDetails, auth_store::AuthStore, capabilities::ConnectorConfigSchema,
    CallToolRequestParam, CallToolResult, Connector, ConnectorError, ListToolsResult,
    PaginatedRequestParam, Tool,
};

use serde_json::{Map, Value};
use std::borrow::Cow;
use tokio::sync::Mutex;

/// A simple facade that exposes a unified tool surface across all enabled connectors.
/// - Tool names are namespaced as `provider.action` (e.g., `wikipedia.search`).
/// - Only connectors compiled in via Cargo features are included.
pub struct Tools {
    connectors: HashMap<String, Arc<Mutex<Box<dyn Connector>>>>,
    store: Option<Arc<dyn AuthStore>>,
}

impl Tools {
    /// Build Tools containing only feature-enabled connectors.
    pub async fn build_enabled_only() -> Self {
        #[allow(unused_mut)]
        let mut connectors: HashMap<String, Arc<Mutex<Box<dyn Connector>>>> = HashMap::new();

        #[cfg(feature = "hackernews")]
        {
            let c = Box::new(crate::connectors::hackernews::HackerNewsConnector::new())
                as Box<dyn Connector>;
            connectors.insert("hackernews".to_string(), Arc::new(Mutex::new(c)));
        }

        #[cfg(feature = "wikipedia")]
        {
            if let Ok(c) =
                crate::connectors::wikipedia::WikipediaConnector::new(AuthDetails::new()).await
            {
                connectors.insert("wikipedia".to_string(), Arc::new(Mutex::new(Box::new(c))));
            }
        }

        #[cfg(feature = "youtube")]
        {
            if let Ok(c) = crate::connectors::youtube::YouTubeConnector::new(None).await {
                connectors.insert("youtube".to_string(), Arc::new(Mutex::new(Box::new(c))));
            }
        }

        #[cfg(feature = "arxiv")]
        {
            if let Ok(c) = crate::connectors::arxiv::ArxivConnector::new(AuthDetails::new()).await {
                connectors.insert("arxiv".to_string(), Arc::new(Mutex::new(Box::new(c))));
            }
        }

        #[cfg(feature = "pubmed")]
        {
            if let Ok(c) = crate::connectors::pubmed::PubMedConnector::new().await {
                connectors.insert("pubmed".to_string(), Arc::new(Mutex::new(Box::new(c))));
            }
        }

        #[cfg(feature = "semantic-scholar")]
        {
            if let Ok(c) = crate::connectors::semantic_scholar::SemanticScholarConnector::new(
                AuthDetails::new(),
            )
            .await
            {
                connectors.insert(
                    "semantic_scholar".to_string(),
                    Arc::new(Mutex::new(Box::new(c))),
                );
            }
        }

        #[cfg(feature = "web")]
        {
            if let Ok(c) = crate::connectors::web::WebConnector::new(AuthDetails::new()).await {
                connectors.insert("web".to_string(), Arc::new(Mutex::new(Box::new(c))));
            }
        }

        #[cfg(feature = "reddit")]
        {
            if let Ok(c) = crate::connectors::reddit::RedditConnector::new(AuthDetails::new()).await
            {
                connectors.insert("reddit".to_string(), Arc::new(Mutex::new(Box::new(c))));
            }
        }

        #[cfg(feature = "x-twitter")]
        {
            if let Ok(c) = crate::connectors::x::XConnector::new(AuthDetails::new()).await {
                connectors.insert("x".to_string(), Arc::new(Mutex::new(Box::new(c))));
            }
        }

        #[cfg(feature = "scihub")]
        {
            if let Ok(c) = crate::connectors::scihub::SciHubConnector::new(AuthDetails::new()).await
            {
                connectors.insert("scihub".to_string(), Arc::new(Mutex::new(Box::new(c))));
            }
        }

        #[cfg(feature = "imap")]
        {
            if let Ok(c) = crate::connectors::imap::ImapConnector::new(AuthDetails::new()).await {
                connectors.insert("imap".to_string(), Arc::new(Mutex::new(Box::new(c))));
            }
        }
        #[cfg(feature = "macos-automation")]
        {
            let c = crate::connectors::macos::MacOsAutomationConnector::new();
            connectors.insert("macos".to_string(), Arc::new(Mutex::new(Box::new(c))));
        }

        // LLM provider web search connectors
        #[cfg(feature = "openai-search")]
        {
            if let Ok(c) =
                crate::connectors::openai_search::OpenAIWebSearchConnector::new(AuthDetails::new())
                    .await
            {
                connectors.insert(
                    "openai-search".to_string(),
                    Arc::new(Mutex::new(Box::new(c))),
                );
            }
        }

        #[cfg(feature = "anthropic-search")]
        {
            if let Ok(c) = crate::connectors::anthropic_search::AnthropicWebSearchConnector::new(
                AuthDetails::new(),
            )
            .await
            {
                connectors.insert(
                    "anthropic-search".to_string(),
                    Arc::new(Mutex::new(Box::new(c))),
                );
            }
        }

        #[cfg(feature = "gemini-search")]
        {
            if let Ok(c) =
                crate::connectors::gemini_search::GeminiSearchConnector::new(AuthDetails::new())
                    .await
            {
                connectors.insert(
                    "gemini-search".to_string(),
                    Arc::new(Mutex::new(Box::new(c))),
                );
            }
        }

        #[cfg(feature = "perplexity-search")]
        {
            if let Ok(c) = crate::connectors::perplexity_search::PerplexitySearchConnector::new(
                AuthDetails::new(),
            )
            .await
            {
                connectors.insert(
                    "perplexity-search".to_string(),
                    Arc::new(Mutex::new(Box::new(c))),
                );
            }
        }

        #[cfg(feature = "xai-search")]
        {
            if let Ok(c) =
                crate::connectors::xai_search::XaiSearchConnector::new(AuthDetails::new()).await
            {
                connectors.insert("xai-search".to_string(), Arc::new(Mutex::new(Box::new(c))));
            }
        }

        #[cfg(feature = "exa-search")]
        {
            if let Ok(c) =
                crate::connectors::exa_search::ExaSearchConnector::new(AuthDetails::new()).await
            {
                connectors.insert("exa-search".to_string(), Arc::new(Mutex::new(Box::new(c))));
            }
        }

        #[cfg(feature = "firecrawl-search")]
        {
            if let Ok(c) = crate::connectors::firecrawl_search::FirecrawlSearchConnector::new(
                AuthDetails::new(),
            )
            .await
            {
                connectors.insert(
                    "firecrawl-search".to_string(),
                    Arc::new(Mutex::new(Box::new(c))),
                );
            }
        }

        #[cfg(feature = "serper-search")]
        {
            if let Ok(c) =
                crate::connectors::serper_search::SerperSearchConnector::new(AuthDetails::new())
                    .await
            {
                connectors.insert(
                    "serper-search".to_string(),
                    Arc::new(Mutex::new(Box::new(c))),
                );
            }
        }

        #[cfg(feature = "tavily-search")]
        {
            if let Ok(c) =
                crate::connectors::tavily_search::TavilySearchConnector::new(AuthDetails::new())
                    .await
            {
                connectors.insert(
                    "tavily-search".to_string(),
                    Arc::new(Mutex::new(Box::new(c))),
                );
            }
        }

        #[cfg(feature = "serpapi-search")]
        {
            if let Ok(c) =
                crate::connectors::serpapi_search::SerpapiSearchConnector::new(AuthDetails::new())
                    .await
            {
                connectors.insert(
                    "serpapi-search".to_string(),
                    Arc::new(Mutex::new(Box::new(c))),
                );
            }
        }

        Tools {
            connectors,
            store: None,
        }
    }

    /// List all tools across connectors, namespaced as "provider.tool".
    pub async fn list(&self) -> Result<ListToolsResult, ConnectorError> {
        let mut all = Vec::new();
        for (provider, conn) in &self.connectors {
            let c = conn.lock().await;
            if let Ok(list) = c
                .list_tools(Some(PaginatedRequestParam { cursor: None }))
                .await
            {
                for t in list.tools {
                    let namespaced = Tool {
                        name: Cow::Owned(format!("{}.{}", provider, t.name)),
                        title: None,
                        description: t.description,
                        input_schema: t.input_schema,
                        output_schema: None,
                        annotations: t.annotations,
                        icons: None,
                    };
                    all.push(namespaced);
                }
            }
        }
        Ok(ListToolsResult {
            tools: all,
            next_cursor: None,
        })
    }

    /// Call a tool by its namespaced name ("provider.tool").
    pub async fn call(&self, name: &str, args: Value) -> Result<CallToolResult, ConnectorError> {
        let (provider, tool) = match name.split_once('.') {
            Some((p, t)) if !p.is_empty() && !t.is_empty() => (p, t),
            _ => {
                return Err(ConnectorError::InvalidParams(
                    "Tool name must be 'provider.tool'".to_string(),
                ))
            }
        };
        let conn = self
            .connectors
            .get(provider)
            .ok_or_else(|| ConnectorError::ToolNotFound)?
            .clone();

        let arg_map: Map<String, Value> = match args {
            Value::Object(map) => map,
            _ => Map::new(),
        };
        let req = CallToolRequestParam {
            name: tool.to_string().into(),
            arguments: Some(arg_map),
        };

        let c = conn.lock().await;
        c.call_tool(req).await
    }

    /// Set authentication details for a specific provider.
    pub async fn set_auth(
        &self,
        provider: &str,
        details: AuthDetails,
    ) -> Result<(), ConnectorError> {
        let conn = self
            .connectors
            .get(provider)
            .ok_or_else(|| ConnectorError::ToolNotFound)?
            .clone();
        let mut c = conn.lock().await;
        c.set_auth_details(details.clone()).await?;
        if let Some(store) = &self.store {
            let _ = store.save(provider, &details);
        }
        Ok(())
    }

    /// Return a connector's config schema to drive UIs.
    pub async fn config_schema(
        &self,
        provider: &str,
    ) -> Result<ConnectorConfigSchema, ConnectorError> {
        let conn = self
            .connectors
            .get(provider)
            .ok_or_else(|| ConnectorError::ToolNotFound)?
            .clone();
        let c = conn.lock().await;
        Ok(c.config_schema())
    }

    /// Return the provider names compiled in this build.
    pub fn list_providers(&self) -> Vec<String> {
        let mut v: Vec<String> = self.connectors.keys().cloned().collect();
        v.sort();
        v
    }

    /// Describe a namespaced tool (provider.tool).
    pub async fn describe(&self, name: &str) -> Result<Tool, ConnectorError> {
        let (provider, tool) = match name.split_once('.') {
            Some((p, t)) if !p.is_empty() && !t.is_empty() => (p, t),
            _ => {
                return Err(ConnectorError::InvalidParams(
                    "Tool name must be 'provider.tool'".to_string(),
                ))
            }
        };
        let conn = self
            .connectors
            .get(provider)
            .ok_or_else(|| ConnectorError::ToolNotFound)?
            .clone();
        let c = conn.lock().await;
        let list = c
            .list_tools(Some(PaginatedRequestParam { cursor: None }))
            .await?;
        for t in list.tools {
            if t.name == tool {
                return Ok(Tool {
                    name: t.name,
                    title: None,
                    description: t.description,
                    input_schema: t.input_schema,
                    output_schema: None,
                    annotations: t.annotations,
                    icons: None,
                });
            }
        }
        Err(ConnectorError::ToolNotFound)
    }

    /// Convenience for dev shells; desktop apps should prefer AuthStore/with_auth.
    pub async fn auth_from_env(&self) {
        use std::env;
        // Reddit
        if let (Ok(id), Ok(secret)) = (
            env::var("REDDIT_CLIENT_ID"),
            env::var("REDDIT_CLIENT_SECRET"),
        ) {
            let mut auth = AuthDetails::new();
            auth.insert("client_id".into(), id);
            auth.insert("client_secret".into(), secret);
            if let Ok(user) = env::var("REDDIT_USERNAME") {
                auth.insert("username".into(), user);
            }
            if let Ok(pass) = env::var("REDDIT_PASSWORD") {
                auth.insert("password".into(), pass);
            }
            let _ = self.set_auth("reddit", auth).await;
        }
        // X/Twitter (bearer or username/password depending on connector expectations)
        if let Ok(bearer) = env::var("X_BEARER_TOKEN") {
            let mut auth = AuthDetails::new();
            auth.insert("bearer_token".into(), bearer);
            let _ = self.set_auth("x", auth).await;
        }
        // Wikipedia options (language)
        if let Ok(lang) = env::var("WIKIPEDIA_LANG") {
            let mut auth = AuthDetails::new();
            auth.insert("language".into(), lang);
            let _ = self.set_auth("wikipedia", auth).await;
        }
    }
}

/// Builder for Tools with app-managed auth flow and optional store persistence.
pub struct ToolsBuilder {
    auths: HashMap<String, AuthDetails>,
    store: Option<Arc<dyn AuthStore>>,
}

impl ToolsBuilder {
    pub fn new() -> Self {
        Self {
            auths: HashMap::new(),
            store: None,
        }
    }

    pub fn with_auth(mut self, provider: &str, details: AuthDetails) -> Self {
        self.auths.insert(provider.to_string(), details);
        self
    }

    pub fn with_auth_bulk(mut self, map: HashMap<String, AuthDetails>) -> Self {
        self.auths.extend(map);
        self
    }

    pub fn with_auth_store(mut self, store: Arc<dyn AuthStore>) -> Self {
        self.store = Some(store);
        self
    }

    pub async fn build(self) -> Result<Tools, ConnectorError> {
        let mut tools = Tools::build_enabled_only().await;
        tools.store = self.store.clone();

        // Load persisted auths first
        if let Some(store) = &self.store {
            for provider in tools.connectors.keys() {
                if let Some(auth) = store.load(provider) {
                    let _ = tools.set_auth(provider, auth).await; // ignore errors to keep building
                }
            }
        }

        // Overlay with explicitly provided auths
        for (provider, auth) in self.auths.into_iter() {
            let _ = tools.set_auth(&provider, auth).await;
        }

        Ok(tools)
    }
}

impl Default for ToolsBuilder {
    fn default() -> Self {
        Self::new()
    }
}
