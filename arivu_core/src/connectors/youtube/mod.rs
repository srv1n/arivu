// src/connectors/youtube/mod.rs

use crate::capabilities::ConnectorConfigSchema;
use crate::error::ConnectorError;
use crate::utils::structured_result_with_text;
use crate::utils::{clean_html_entities, get_cookies, match_browser};
use crate::{auth::AuthDetails, Connector};
use async_trait::async_trait;
use chrono::TimeZone;
use chrono::{DateTime, Duration, NaiveDate, Utc};
use futures::FutureExt;
use once_cell::sync::Lazy;
use regex::Regex;
use rmcp::model::*;
use rusty_ytdl::search::{SearchOptions, SearchResult, SearchType, YouTube};
use rusty_ytdl::{RequestOptions, Video, VideoOptions};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::borrow::Cow;
use std::cmp::Ordering;
use std::panic::AssertUnwindSafe;
use std::sync::Arc;
use url::Url;
use yt_transcript_rs::YouTubeTranscriptApi;

// Input/Output structs for tools
/// Response format for controlling output verbosity
#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ResponseFormat {
    /// Minimal response for token efficiency - only essential fields (default)
    #[default]
    Concise,
    /// Full response with all metadata
    Detailed,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct GetVideoDetailsInput {
    /// The YouTube video ID (e.g., 'dQw4w9WgXcQ') or full URL
    pub video_id: String,
    /// Response verbosity: 'concise' returns only title and transcript/chapters, 'detailed' includes description and all metadata
    #[serde(default)]
    pub response_format: ResponseFormat,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct SearchVideosInput {
    /// Search query string
    pub query: String,
    /// Maximum number of results to return
    #[serde(default = "default_limit")]
    #[schemars(default = "default_limit")]
    pub limit: u64,
    /// Type of content to search for
    #[serde(default = "default_search_category")]
    #[schemars(default = "default_search_category")]
    pub search_type: SearchCategory,
    /// Sort order for results
    #[serde(default)]
    pub sort: Option<SearchSort>,
    /// Filter by upload date
    #[serde(default)]
    pub upload_date: Option<UploadDateFilter>,
    /// Response verbosity: 'concise' returns only id/title/url, 'detailed' includes all metadata
    #[serde(default)]
    pub response_format: ResponseFormat,
}

fn default_limit() -> u64 {
    5
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct SearchVideosOutput {
    pub results: Vec<SearchResultItem>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct VideoSearchResult {
    pub id: String,
    pub title: String,
    pub description: String,
    pub thumbnail: String,
    pub url: String,
    pub duration_seconds: u64,
    pub views: u64,
    pub uploaded_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct PlaylistSearchResult {
    pub id: String,
    pub title: String,
    pub url: String,
    pub thumbnail: String,
    pub channel: ChannelSearchResult,
    pub video_count: u64,
    pub views: u64,
    pub last_update: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ChannelSearchResult {
    pub id: String,
    pub title: String,
    pub url: String,
    pub thumbnail: String,
    pub verified: bool,
    pub subscribers: u64,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum SearchResultItem {
    Video(VideoSearchResult),
    Playlist(PlaylistSearchResult),
    Channel(ChannelSearchResult),
}

#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SearchCategory {
    Video,
    Playlist,
    Channel,
    All,
}

fn default_search_category() -> SearchCategory {
    SearchCategory::Video
}

#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SearchSort {
    Relevance,
    ViewsDesc,
    ViewsAsc,
    DurationDesc,
    DurationAsc,
    SubscribersDesc,
    SubscribersAsc,
    PlaylistVideosDesc,
    PlaylistVideosAsc,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UploadDateFilter {
    Any,
    LastHour,
    Today,
    ThisWeek,
    ThisMonth,
    ThisYear,
}

impl From<rusty_ytdl::search::Video> for VideoSearchResult {
    fn from(video: rusty_ytdl::search::Video) -> Self {
        let thumbnail = video
            .thumbnails
            .first()
            .map(|t| t.url.clone())
            .unwrap_or_default();

        Self {
            id: video.id.clone(),
            title: video.title.clone(),
            description: video.description.clone(),
            thumbnail,
            url: format!("https://www.youtube.com/watch?v={}", video.id),
            duration_seconds: video.duration,
            views: video.views,
            uploaded_at: video.uploaded_at.clone(),
        }
    }
}

impl From<rusty_ytdl::search::Channel> for ChannelSearchResult {
    fn from(channel: rusty_ytdl::search::Channel) -> Self {
        let thumbnail = channel
            .icon
            .first()
            .map(|t| t.url.clone())
            .unwrap_or_default();

        Self {
            id: channel.id,
            title: channel.name,
            url: channel.url,
            thumbnail,
            verified: channel.verified,
            subscribers: channel.subscribers,
        }
    }
}

impl From<rusty_ytdl::search::Playlist> for PlaylistSearchResult {
    fn from(playlist: rusty_ytdl::search::Playlist) -> Self {
        let thumbnail = playlist
            .thumbnails
            .first()
            .map(|t| t.url.clone())
            .unwrap_or_default();

        Self {
            id: playlist.id,
            title: playlist.name,
            url: playlist.url,
            thumbnail,
            channel: playlist.channel.clone().into(),
            video_count: playlist.videos.len() as u64,
            views: playlist.views,
            last_update: playlist.last_update,
        }
    }
}

impl From<rusty_ytdl::search::Channel> for SearchResultItem {
    fn from(value: rusty_ytdl::search::Channel) -> Self {
        SearchResultItem::Channel(value.into())
    }
}

impl From<rusty_ytdl::search::Video> for SearchResultItem {
    fn from(value: rusty_ytdl::search::Video) -> Self {
        SearchResultItem::Video(value.into())
    }
}

impl From<rusty_ytdl::search::Playlist> for SearchResultItem {
    fn from(value: rusty_ytdl::search::Playlist) -> Self {
        SearchResultItem::Playlist(value.into())
    }
}

fn to_rusty_search_type(category: SearchCategory) -> SearchType {
    match category {
        SearchCategory::Video => SearchType::Video,
        SearchCategory::Playlist => SearchType::Playlist,
        SearchCategory::Channel => SearchType::Channel,
        SearchCategory::All => SearchType::All,
    }
}

fn apply_sort(
    results: &mut [SearchResultItem],
    sort: Option<SearchSort>,
    category: SearchCategory,
) -> Result<(), ConnectorError> {
    let Some(sort) = sort else {
        return Ok(());
    };

    if sort == SearchSort::Relevance {
        return Ok(());
    }

    match category {
        SearchCategory::Video => sort_videos(results, sort),
        SearchCategory::Playlist => sort_playlists(results, sort),
        SearchCategory::Channel => sort_channels(results, sort),
        SearchCategory::All => Err(ConnectorError::InvalidParams(
            "Sorting is only supported when search_type is video, playlist, or channel".to_string(),
        )),
    }
}

fn sort_videos(results: &mut [SearchResultItem], sort: SearchSort) -> Result<(), ConnectorError> {
    use SearchSort::*;

    match sort {
        ViewsDesc => {
            results.sort_by(|a, b| match (a, b) {
                (SearchResultItem::Video(a), SearchResultItem::Video(b)) => b.views.cmp(&a.views),
                _ => Ordering::Equal,
            });
            Ok(())
        }
        ViewsAsc => {
            results.sort_by(|a, b| match (a, b) {
                (SearchResultItem::Video(a), SearchResultItem::Video(b)) => a.views.cmp(&b.views),
                _ => Ordering::Equal,
            });
            Ok(())
        }
        DurationDesc => {
            results.sort_by(|a, b| match (a, b) {
                (SearchResultItem::Video(a), SearchResultItem::Video(b)) => {
                    b.duration_seconds.cmp(&a.duration_seconds)
                }
                _ => Ordering::Equal,
            });
            Ok(())
        }
        DurationAsc => {
            results.sort_by(|a, b| match (a, b) {
                (SearchResultItem::Video(a), SearchResultItem::Video(b)) => {
                    a.duration_seconds.cmp(&b.duration_seconds)
                }
                _ => Ordering::Equal,
            });
            Ok(())
        }
        SubscribersDesc | SubscribersAsc | PlaylistVideosDesc | PlaylistVideosAsc | Relevance => {
            Err(ConnectorError::InvalidParams(format!(
                "Sort {:?} is not supported for video search",
                sort
            )))
        }
    }
}

fn sort_playlists(
    results: &mut [SearchResultItem],
    sort: SearchSort,
) -> Result<(), ConnectorError> {
    use SearchSort::*;

    match sort {
        ViewsDesc => {
            results.sort_by(|a, b| match (a, b) {
                (SearchResultItem::Playlist(a), SearchResultItem::Playlist(b)) => {
                    b.views.cmp(&a.views)
                }
                _ => Ordering::Equal,
            });
            Ok(())
        }
        ViewsAsc => {
            results.sort_by(|a, b| match (a, b) {
                (SearchResultItem::Playlist(a), SearchResultItem::Playlist(b)) => {
                    a.views.cmp(&b.views)
                }
                _ => Ordering::Equal,
            });
            Ok(())
        }
        PlaylistVideosDesc => {
            results.sort_by(|a, b| match (a, b) {
                (SearchResultItem::Playlist(a), SearchResultItem::Playlist(b)) => {
                    b.video_count.cmp(&a.video_count)
                }
                _ => Ordering::Equal,
            });
            Ok(())
        }
        PlaylistVideosAsc => {
            results.sort_by(|a, b| match (a, b) {
                (SearchResultItem::Playlist(a), SearchResultItem::Playlist(b)) => {
                    a.video_count.cmp(&b.video_count)
                }
                _ => Ordering::Equal,
            });
            Ok(())
        }
        DurationDesc | DurationAsc | SubscribersDesc | SubscribersAsc | Relevance => {
            Err(ConnectorError::InvalidParams(format!(
                "Sort {:?} is not supported for playlist search",
                sort
            )))
        }
    }
}

fn sort_channels(results: &mut [SearchResultItem], sort: SearchSort) -> Result<(), ConnectorError> {
    use SearchSort::*;

    match sort {
        SubscribersDesc => {
            results.sort_by(|a, b| match (a, b) {
                (SearchResultItem::Channel(a), SearchResultItem::Channel(b)) => {
                    b.subscribers.cmp(&a.subscribers)
                }
                _ => Ordering::Equal,
            });
            Ok(())
        }
        SubscribersAsc => {
            results.sort_by(|a, b| match (a, b) {
                (SearchResultItem::Channel(a), SearchResultItem::Channel(b)) => {
                    a.subscribers.cmp(&b.subscribers)
                }
                _ => Ordering::Equal,
            });
            Ok(())
        }
        ViewsDesc | ViewsAsc | DurationDesc | DurationAsc | PlaylistVideosDesc
        | PlaylistVideosAsc | Relevance => Err(ConnectorError::InvalidParams(format!(
            "Sort {:?} is not supported for channel search",
            sort
        ))),
    }
}

fn apply_upload_date_filter(
    results: &mut Vec<SearchResultItem>,
    filter: Option<UploadDateFilter>,
    category: SearchCategory,
) -> Result<(), ConnectorError> {
    let Some(filter) = filter else {
        return Ok(());
    };

    if filter == UploadDateFilter::Any {
        return Ok(());
    }

    if category == SearchCategory::Channel {
        return Err(ConnectorError::InvalidParams(
            "upload_date filter is not supported for channel search".to_string(),
        ));
    }

    let Some(cutoff) = cutoff_for_filter(filter) else {
        return Ok(());
    };

    results.retain(|item| match item {
        SearchResultItem::Video(video) => video
            .uploaded_at
            .as_deref()
            .and_then(parse_uploaded_timestamp)
            .map(|timestamp| timestamp >= cutoff)
            .unwrap_or(false),
        SearchResultItem::Playlist(playlist) => playlist
            .last_update
            .as_deref()
            .and_then(parse_uploaded_timestamp)
            .map(|timestamp| timestamp >= cutoff)
            .unwrap_or(false),
        SearchResultItem::Channel(_) => false,
    });

    Ok(())
}

fn cutoff_for_filter(filter: UploadDateFilter) -> Option<DateTime<Utc>> {
    let now = Utc::now();

    match filter {
        UploadDateFilter::Any => None,
        UploadDateFilter::LastHour => Some(now - Duration::hours(1)),
        UploadDateFilter::Today => {
            let midnight = now.date_naive().and_hms_opt(0, 0, 0)?;
            Some(Utc.from_utc_datetime(&midnight))
        }
        UploadDateFilter::ThisWeek => Some(now - Duration::days(7)),
        UploadDateFilter::ThisMonth => Some(now - Duration::days(30)),
        UploadDateFilter::ThisYear => Some(now - Duration::days(365)),
    }
}

fn parse_uploaded_timestamp(raw: &str) -> Option<DateTime<Utc>> {
    let cleaned = raw.trim();
    if cleaned.is_empty() {
        return None;
    }

    let base = cleaned.split('•').next().unwrap_or(cleaned).trim();
    if base.eq_ignore_ascii_case("live") {
        return None;
    }

    let mut normalized = base;

    static PREFIXES: [&str; 8] = [
        "Streamed live on ",
        "Streamed live ",
        "Streamed ",
        "Premiered ",
        "Uploaded ",
        "Live streamed on ",
        "Last updated on ",
        "Last update on ",
    ];

    loop {
        let mut stripped = None;
        for prefix in PREFIXES {
            if normalized.len() >= prefix.len()
                && normalized[..prefix.len()].eq_ignore_ascii_case(prefix)
            {
                stripped = Some(normalized[prefix.len()..].trim());
                break;
            }
        }

        if let Some(rest) = stripped {
            normalized = rest;
        } else {
            break;
        }
    }

    // Handle relative expressions like "3 years ago"
    static RELATIVE_RE: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"(?i)(?P<num>\d+)\s+(?P<unit>second|minute|hour|day|week|month|year)s?\s+ago")
            .unwrap()
    });

    if let Some(caps) = RELATIVE_RE.captures(normalized) {
        let num = caps.name("num")?.as_str().parse::<i64>().ok()?;
        let unit = caps.name("unit")?.as_str().to_lowercase();
        let duration = match unit.as_str() {
            "second" => Duration::seconds(num),
            "minute" => Duration::minutes(num),
            "hour" => Duration::hours(num),
            "day" => Duration::days(num),
            "week" => Duration::weeks(num),
            "month" => Duration::days(num * 30),
            "year" => Duration::days(num * 365),
            _ => return None,
        };
        return Some(Utc::now() - duration);
    }

    // Handle absolute dates like "Jan 1, 2023"
    static DATE_FORMATS: Lazy<Vec<&'static str>> = Lazy::new(|| {
        vec![
            "%b %e, %Y",
            "%b %d, %Y",
            "%B %e, %Y",
            "%B %d, %Y",
            "%b %Y",
            "%B %Y",
            "%Y",
        ]
    });

    for format in DATE_FORMATS.iter() {
        if let Ok(date) = NaiveDate::parse_from_str(normalized, format) {
            let midnight = date.and_hms_opt(0, 0, 0)?;
            return Some(Utc.from_utc_datetime(&midnight));
        }
    }

    None
}

#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone)]
pub struct YouTubeContent {
    pub id: String,
    pub title: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transcript: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub chapters: Vec<ChapterContent>,
}

/// Concise version of YouTubeContent for token efficiency
#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone)]
pub struct YouTubeContentConcise {
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transcript: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub chapters: Vec<ChapterContentConcise>,
}

/// Concise chapter content - just heading and content
#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
pub struct ChapterContentConcise {
    pub heading: String,
    pub content: String,
}

/// Concise video search result
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct VideoSearchResultConcise {
    pub id: String,
    pub title: String,
    pub url: String,
}

/// Concise playlist search result
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct PlaylistSearchResultConcise {
    pub id: String,
    pub title: String,
    pub url: String,
}

/// Concise channel search result
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ChannelSearchResultConcise {
    pub id: String,
    pub title: String,
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum SearchResultItemConcise {
    Video(VideoSearchResultConcise),
    Playlist(PlaylistSearchResultConcise),
    Channel(ChannelSearchResultConcise),
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct SearchVideosOutputConcise {
    pub results: Vec<SearchResultItemConcise>,
}

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
pub struct ChapterContent {
    pub heading: String,
    pub start_time: i32,
    pub content: String,
}

#[derive(Clone)]
pub struct YouTubeConnector {
    video_options: VideoOptions,
}

impl YouTubeConnector {
    pub async fn new(auth: Option<AuthDetails>) -> Result<Self, ConnectorError> {
        let mut connector = YouTubeConnector {
            video_options: VideoOptions::default(), // Default quality
        };

        if let Some(auth) = auth {
            connector.set_auth_details(auth).await?;
        }

        Ok(connector)
    }
}

#[async_trait]
impl Connector for YouTubeConnector {
    fn name(&self) -> &'static str {
        "youtube"
    }

    fn description(&self) -> &'static str {
        "A connector for interacting with YouTube."
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
        if let Some(browser) = details.get("browser") {
            let browser = match_browser(browser.to_string())
                .await
                .map_err(|e| ConnectorError::Other(e.to_string()))?;
            let cookies = get_cookies(browser, "youtube.com".to_string())
                .await
                .map_err(|e| ConnectorError::Other(e.to_string()))?;

            self.video_options = VideoOptions {
                request_options: RequestOptions {
                    cookies: Some(cookies),
                    ..Default::default()
                },
                ..Default::default()
            };
            return Ok(());
        }

        Ok(()) // No auth
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
                "YouTube connector for accessing video metadata, transcripts, and details"
                    .to_string(),
            ),
        })
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParam>,
    ) -> Result<ListResourcesResult, ConnectorError> {
        let resources = vec![];

        Ok(ListResourcesResult {
            resources,
            next_cursor: None,
        })
    }

    async fn read_resource(
        &self,
        _request: ReadResourceRequestParam,
    ) -> Result<Vec<ResourceContents>, ConnectorError> {
        // let uri_str = request.uri.as_str();

        // if uri_str.starts_with("youtube://video/") {
        //     let parts: Vec<&str> = uri_str.split('/').collect();
        //     if parts.len() < 4 {
        //         return Err(ConnectorError::InvalidInput(format!("Invalid resource URI: {}", uri_str)));
        //     }
        //     let video_id = parts[3];

        //     let video_options = VideoOptions {
        //         quality: self.video_quality.clone(),
        //         filter: VideoSearchOptions::Video, // or Audio, depending on what you need
        //         ..Default::default()
        //     };
        //     let video = Video::new_with_options(format!("https://www.youtube.com/watch?v={}", video_id).as_str(), video_options)
        //         .map_err(|e| ConnectorError::Other(e.to_string()))?;

        //     let video_info = video.get_info().await.map_err(|e| ConnectorError::Other(e.to_string()))?;

        //     let chapters = video_info.video_details.chapters.clone();
        //      let transcript = match YoutubeTranscript::fetch_transcript(&format!("https://www.youtube.com/watch?v={}", video_id), None).await {
        //         Ok(transcript) => {
        //             let chapter_contents = self.group_transcript_by_chapters(&chapters, transcript);
        //             Some(chapter_contents)
        //         }
        //         Err(e) => {
        //             eprintln!("Error fetching transcript: {}", e);
        //             None
        //         }
        //     };

        //     let youtube_content =  YouTubeContent {
        //         title: video_info.video_details.title.clone(),
        //         description: video_info.video_details.description.clone(),
        //         transcript: None, // Populated below if available
        //         chapters: transcript.unwrap_or_default()
        //     };

        Ok(vec![])
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParam>,
    ) -> Result<ListToolsResult, ConnectorError> {
        let tools = vec![
            Tool {
                name: Cow::Borrowed("get_video_details"),
                title: None,
                description: Some(Cow::Borrowed("Given a YouTube video id or URL, returns video details: title, author, description, and either chapters (preferred) or a raw transcript.")),
                input_schema: Arc::new(serde_json::to_value(schemars::schema_for!(GetVideoDetailsInput)).map_err(|e| ConnectorError::Other(e.to_string()))?.as_object().expect("Schema object").clone()),
                output_schema: None,
                annotations: None,
                icons: None,
            },
            Tool {
                name: Cow::Borrowed("search_videos"),
                title: None,
                description: Some(Cow::Borrowed(
                    "Search YouTube with filters and sorting. Supports search_type (video|playlist|channel|all), upload_date (e.g., today/this_week), and sort (views/duration/subscribers). Returns mixed results with rich fields."
                )),
                input_schema: Arc::new(serde_json::to_value(schemars::schema_for!(SearchVideosInput)).map_err(|e| ConnectorError::Other(e.to_string()))?.as_object().expect("Schema object").clone()),
                output_schema: None,
                annotations: None,
                icons: None,
            }
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
        let args_map = serde_json::Map::from_iter(args);

        match name {
            "get_video_details" => {
                let input: GetVideoDetailsInput =
                    serde_json::from_value(Value::Object(args_map))
                        .map_err(|e| ConnectorError::InvalidParams(e.to_string()))?;

                let video_id = extract_video_id(&input.video_id);

                let video = Video::new_with_options(
                    format!("https://www.youtube.com/watch?v={}", video_id).as_str(),
                    self.video_options.clone(),
                )
                .map_err(|e| ConnectorError::Other(e.to_string()))?;

                // Guard against upstream panics in rusty_ytdl
                let video_info = AssertUnwindSafe(video.get_info())
                    .catch_unwind()
                    .await
                    .map_err(|_| ConnectorError::Other("YouTube get_info panicked".to_string()))?
                    .map_err(|e| ConnectorError::Other(e.to_string()))?;

                let chapters = video_info.video_details.chapters.clone();
                let api = YouTubeTranscriptApi::new(None, None, None)
                    .map_err(|e| ConnectorError::Other(e.to_string()))?;

                // Fetch transcript parts once; we will decide whether to expose
                // chapterized content or a raw transcript, but never both.
                let (chapters_out, transcript_out) =
                    match api.fetch_transcript(&video_id, &["en"], false).await {
                        Ok(fetched) => {
                            // Build a raw transcript string from parts (cleaned) for fallback.
                            let parts = fetched.parts();
                            let raw_text = parts
                                .iter()
                                .map(|p| p.text.clone())
                                .collect::<Vec<_>>()
                                .join(" ");
                            let cleaned = clean_html_entities(&raw_text);

                            if !chapters.is_empty() {
                                // Prefer chapterized content when real chapter metadata exists.
                                let grouped = group_transcript_by_chapters_new(&chapters, fetched);
                                if !grouped.is_empty() {
                                    (grouped, None)
                                } else if !cleaned.is_empty() {
                                    (Vec::new(), Some(cleaned))
                                } else {
                                    (Vec::new(), None)
                                }
                            } else if !cleaned.is_empty() {
                                // No chapters metadata → provide raw transcript only.
                                (Vec::new(), Some(cleaned))
                            } else {
                                (Vec::new(), None)
                            }
                        }
                        Err(e) => {
                            tracing::warn!(
                                error = %e,
                                video_id = %video_id,
                                "Failed to fetch YouTube transcript"
                            );
                            (Vec::new(), None)
                        }
                    };

                // Return concise or detailed based on response_format
                if input.response_format == ResponseFormat::Concise {
                    let concise_chapters: Vec<ChapterContentConcise> = chapters_out
                        .iter()
                        .map(|c| ChapterContentConcise {
                            heading: c.heading.clone(),
                            content: c.content.clone(),
                        })
                        .collect();
                    let youtube_content = YouTubeContentConcise {
                        title: video_info.video_details.title.clone(),
                        transcript: transcript_out,
                        chapters: concise_chapters,
                    };
                    let text = serde_json::to_string(&youtube_content)?;
                    Ok(structured_result_with_text(&youtube_content, Some(text))?)
                } else {
                    let youtube_content = YouTubeContent {
                        id: video_id,
                        title: video_info.video_details.title.clone(),
                        description: video_info.video_details.description.clone(),
                        transcript: transcript_out,
                        chapters: chapters_out,
                    };
                    let text = serde_json::to_string(&youtube_content)?;
                    Ok(structured_result_with_text(&youtube_content, Some(text))?)
                }
            }
            "search_videos" => {
                let input: SearchVideosInput = serde_json::from_value(Value::Object(args_map))
                    .map_err(|e| ConnectorError::InvalidParams(e.to_string()))?;

                let youtube = YouTube::new().map_err(|e| ConnectorError::Other(e.to_string()))?;

                let search_options = SearchOptions {
                    limit: input.limit,
                    search_type: to_rusty_search_type(input.search_type),
                    ..Default::default()
                };

                // Guard against upstream panics in rusty_ytdl search path
                let results: Vec<SearchResult> =
                    AssertUnwindSafe(youtube.search(&input.query, Some(&search_options)))
                        .catch_unwind()
                        .await
                        .map_err(|_| ConnectorError::Other("YouTube search panicked".to_string()))?
                        .map_err(|e| ConnectorError::Other(e.to_string()))?;

                let mut mapped_results: Vec<SearchResultItem> = results
                    .into_iter()
                    .filter_map(|result| match result {
                        SearchResult::Video(video)
                            if matches!(
                                input.search_type,
                                SearchCategory::Video | SearchCategory::All
                            ) =>
                        {
                            Some(SearchResultItem::from(video))
                        }
                        SearchResult::Playlist(playlist)
                            if matches!(
                                input.search_type,
                                SearchCategory::Playlist | SearchCategory::All
                            ) =>
                        {
                            Some(SearchResultItem::from(playlist))
                        }
                        SearchResult::Channel(channel)
                            if matches!(
                                input.search_type,
                                SearchCategory::Channel | SearchCategory::All
                            ) =>
                        {
                            Some(SearchResultItem::from(channel))
                        }
                        SearchResult::Video(_)
                        | SearchResult::Playlist(_)
                        | SearchResult::Channel(_) => None,
                    })
                    .collect();

                apply_upload_date_filter(
                    &mut mapped_results,
                    input.upload_date,
                    input.search_type,
                )?;

                apply_sort(&mut mapped_results, input.sort, input.search_type)?;

                if input.limit > 0 && mapped_results.len() > input.limit as usize {
                    mapped_results.truncate(input.limit as usize);
                }

                // Return concise or detailed based on response_format
                if input.response_format == ResponseFormat::Concise {
                    let concise_results: Vec<SearchResultItemConcise> = mapped_results
                        .iter()
                        .map(|r| match r {
                            SearchResultItem::Video(v) => {
                                SearchResultItemConcise::Video(VideoSearchResultConcise {
                                    id: v.id.clone(),
                                    title: v.title.clone(),
                                    url: v.url.clone(),
                                })
                            }
                            SearchResultItem::Playlist(p) => {
                                SearchResultItemConcise::Playlist(PlaylistSearchResultConcise {
                                    id: p.id.clone(),
                                    title: p.title.clone(),
                                    url: p.url.clone(),
                                })
                            }
                            SearchResultItem::Channel(c) => {
                                SearchResultItemConcise::Channel(ChannelSearchResultConcise {
                                    id: c.id.clone(),
                                    title: c.title.clone(),
                                    url: c.url.clone(),
                                })
                            }
                        })
                        .collect();
                    let output = SearchVideosOutputConcise {
                        results: concise_results,
                    };
                    let text = serde_json::to_string(&output)?;
                    Ok(structured_result_with_text(&output, Some(text))?)
                } else {
                    let output = SearchVideosOutput {
                        results: mapped_results,
                    };
                    let text = serde_json::to_string(&output)?;
                    Ok(structured_result_with_text(&output, Some(text))?)
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
            prompts: vec![], // No prompts for now.  Add if you have use cases.
            next_cursor: None,
        })
    }

    async fn get_prompt(&self, _name: &str) -> Result<Prompt, ConnectorError> {
        Err(ConnectorError::MethodNotFound) //  No prompts implemented
    }

    async fn test_auth(&self) -> Result<(), ConnectorError> {
        Ok(())
    }
}

// Helper function to extract video ID from either a full URL or just the ID
fn extract_video_id(input: &str) -> String {
    // Check if the input is a URL
    if input.starts_with("http") {
        // Try to parse as URL
        if let Ok(url) = Url::parse(input) {
            // Extract video ID from query parameters (youtube.com/watch?v=VIDEO_ID)
            if let Some(pairs) = url.query_pairs().find(|(key, _)| key == "v") {
                return pairs.1.to_string();
            }

            // Extract from path segments (youtu.be/VIDEO_ID)
            let path = url.path();
            if url.host_str() == Some("youtu.be") && path.len() > 1 {
                return path[1..].to_string();
            }
        }
    }

    // If not a URL or couldn't extract ID, assume the input is already a video ID
    input.to_string()
}

fn group_transcript_by_chapters_new(
    chapters: &[rusty_ytdl::Chapter],
    transcript: yt_transcript_rs::FetchedTranscript,
) -> Vec<ChapterContent> {
    let parts = transcript.parts();

    if chapters.is_empty() {
        let raw_text = parts
            .iter()
            .map(|p| p.text.clone())
            .collect::<Vec<_>>()
            .join(" ");
        let cleaned_text = clean_html_entities(&raw_text);
        return vec![ChapterContent {
            heading: "Full Video".to_string(),
            start_time: 0,
            content: cleaned_text,
        }];
    }

    let mut chapter_contents = Vec::new();

    for (i, chapter) in chapters.iter().enumerate() {
        let next_start_time = chapters
            .get(i + 1)
            .map(|next| next.start_time)
            .unwrap_or(i32::MAX);

        let content: Vec<String> = parts
            .iter()
            .filter(|p| {
                let p_time = p.start as i32;
                p_time >= chapter.start_time && p_time < next_start_time
            })
            .map(|p| p.text.clone())
            .collect();

        let raw_text = content.join(" ").replace("\n", " ");
        let cleaned_text = clean_html_entities(&raw_text);

        chapter_contents.push(ChapterContent {
            heading: chapter.title.clone(),
            start_time: chapter.start_time,
            content: cleaned_text,
        });
    }

    chapter_contents
}
