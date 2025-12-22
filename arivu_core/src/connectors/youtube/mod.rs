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
use {
    quick_xml::events::Event as XmlEvent, quick_xml::reader::Reader as XmlReader,
    reqwest::Client as HttpClient,
};

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

fn default_list_limit() -> u64 {
    5
}

#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ListSource {
    Channel,
    Playlist,
}

fn default_list_source() -> ListSource {
    ListSource::Channel
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ListVideosInput {
    /// What you are listing: a channel's uploads or a playlist's items.
    #[serde(default = "default_list_source")]
    #[schemars(default = "default_list_source")]
    pub source: ListSource,

    /// Channel identifier. Accepts a channel ID (UC...), a channel URL, or a handle like "@hubermanlab".
    #[serde(default)]
    pub channel: Option<String>,

    /// Playlist identifier. Accepts a playlist ID (PL.../UU...) or a playlist URL.
    #[serde(default)]
    pub playlist: Option<String>,

    /// Max number of videos to return (default: 5).
    #[serde(default = "default_list_limit")]
    #[schemars(default = "default_list_limit")]
    pub limit: u64,

    /// Optional RFC3339 timestamp; only include videos published at/after this time.
    #[serde(default)]
    pub published_after: Option<String>,

    /// Optional relative filter; only include videos published in the last N days (UTC, relative to now).
    /// If provided, this overrides published_after.
    #[serde(default)]
    pub published_within_days: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ListedVideo {
    pub id: String,
    pub title: String,
    pub url: String,
    pub published_at: Option<String>,
    pub channel_title: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ListVideosOutput {
    pub videos: Vec<ListedVideo>,
    pub source: ListSource,
    pub channel_id: Option<String>,
    pub playlist_id: Option<String>,
}

fn default_resolve_limit() -> u64 {
    5
}

fn default_prefer_verified() -> bool {
    true
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ResolveChannelInput {
    /// Free-text channel query (e.g., "Andrew Huberman"). Returns candidate channels.
    #[serde(default)]
    pub query: Option<String>,

    /// Explicit channel identifier to normalize to a channel ID (UC...) when possible.
    /// Accepts a channel ID, channel URL, or handle like "@hubermanlab".
    #[serde(default)]
    pub channel: Option<String>,

    /// Max candidates to return (default: 5).
    #[serde(default = "default_resolve_limit")]
    #[schemars(default = "default_resolve_limit")]
    pub limit: u64,

    /// Prefer verified channels when ranking candidates (default: true).
    #[serde(default = "default_prefer_verified")]
    #[schemars(default = "default_prefer_verified")]
    pub prefer_verified: bool,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone)]
pub struct ChannelCandidate {
    pub channel_id: String,
    pub title: String,
    pub url: String,
    pub verified: bool,
    pub subscribers: u64,
    pub score: f64,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ResolveChannelOutput {
    /// Heuristic best guess (may be None if no candidates found).
    pub recommended: Option<ChannelCandidate>,
    /// Ranked candidates (best first).
    pub candidates: Vec<ChannelCandidate>,
    /// When `channel` was provided, this is the normalized UC... ID if resolved.
    pub resolved_channel_id: Option<String>,
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
    pub channel_name: String,
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
            channel_name: video.channel.name.clone(),
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

/// Concise video search result - includes key metadata for LLM decision-making
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct VideoSearchResultConcise {
    pub id: String,
    pub title: String,
    pub url: String,
    pub channel_name: String,
    pub views: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uploaded_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snippet: Option<String>,
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
                name: Cow::Borrowed("get"),
                title: None,
                description: Some(Cow::Borrowed(
                    "Get title/description plus transcript + chapters (when available). Input is a \
	video ID or URL. Example: video_id=\"dQw4w9WgXcQ\" response_format=\"concise\".",
                )),
                input_schema: Arc::new(
                    serde_json::to_value(schemars::schema_for!(GetVideoDetailsInput))
                        .map_err(|e| ConnectorError::Other(e.to_string()))?
                        .as_object()
                        .expect("Schema object")
                        .clone(),
                ),
                output_schema: None,
                annotations: None,
                icons: None,
            },
            Tool {
                name: Cow::Borrowed("search"),
                title: None,
                description: Some(Cow::Borrowed(
                    "Search YouTube. Returns IDs/URLs you can pass to get. Tip: set \
	search_type=\"video\" unless you explicitly want playlists/channels. Example: query=\"rust\" limit=5.",
                )),
                input_schema: Arc::new(
                    serde_json::to_value(schemars::schema_for!(SearchVideosInput))
                        .map_err(|e| ConnectorError::Other(e.to_string()))?
                        .as_object()
                        .expect("Schema object")
                        .clone(),
                ),
                output_schema: None,
                annotations: None,
                icons: None,
            },
            Tool {
                name: Cow::Borrowed("list"),
                title: None,
                description: Some(Cow::Borrowed(
                    "List recent uploads from a channel or playlist. Use this to answer queries like \
\"last 5 videos from @hubermanlab\" or \"videos from the last week\". Example: \
source=\"channel\" channel=\"@hubermanlab\" limit=5 published_within_days=7.",
                )),
                input_schema: Arc::new(
                    serde_json::to_value(schemars::schema_for!(ListVideosInput))
                        .map_err(|e| ConnectorError::Other(e.to_string()))?
                        .as_object()
                        .expect("Schema object")
                        .clone(),
                ),
                output_schema: None,
                annotations: None,
                icons: None,
            },
            Tool {
                name: Cow::Borrowed("resolve_channel"),
                title: None,
                description: Some(Cow::Borrowed(
                    "Resolve an \"official\" channel. Use this when you have a channel name/handle and want a \
stable UC... channel ID. If query is provided, returns ranked candidates (verified preferred) to reduce ambiguity. \
Example: query=\"Andrew Huberman\" limit=5 prefer_verified=true.",
                )),
                input_schema: Arc::new(
                    serde_json::to_value(schemars::schema_for!(ResolveChannelInput))
                        .map_err(|e| ConnectorError::Other(e.to_string()))?
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
        let name = request.name.as_ref();
        let args = request.arguments.unwrap_or_default();
        let args_map = serde_json::Map::from_iter(args);

        match name {
            "get" | "get_video_details" => {
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
            "search" | "search_videos" => {
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
                                // Create a snippet from description (first ~150 chars)
                                let snippet = if v.description.is_empty() {
                                    None
                                } else {
                                    let clean = v.description.replace('\n', " ");
                                    let truncated: String = clean.chars().take(150).collect();
                                    if clean.chars().count() > 150 {
                                        Some(format!("{}...", truncated))
                                    } else {
                                        Some(truncated)
                                    }
                                };
                                SearchResultItemConcise::Video(VideoSearchResultConcise {
                                    id: v.id.clone(),
                                    title: v.title.clone(),
                                    url: v.url.clone(),
                                    channel_name: v.channel_name.clone(),
                                    views: v.views,
                                    uploaded_at: v.uploaded_at.clone(),
                                    snippet,
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
            "list" | "list_videos" => {
                let input: ListVideosInput = serde_json::from_value(Value::Object(args_map))
                    .map_err(|e| ConnectorError::InvalidParams(e.to_string()))?;

                let limit = input.limit.clamp(1, 50) as usize;
                let client = HttpClient::builder()
                    .user_agent("rzn-datasourcer/0.2.x youtube-connector")
                    .timeout(std::time::Duration::from_secs(20))
                    .build()
                    .map_err(|e| ConnectorError::Other(e.to_string()))?;

                let (feed_url, channel_id, playlist_id) = match input.source {
                    ListSource::Channel => {
                        let Some(ch) = input.channel.as_deref() else {
                            return Err(ConnectorError::InvalidParams(
                                "source='channel' requires 'channel'".to_string(),
                            ));
                        };
                        let cid =
                            resolve_channel_id_best_effort(&client, ch).await.ok_or_else(|| {
                                ConnectorError::InvalidInput(
                                    "Could not resolve channel_id from channel input. Provide a UC... channel ID or a full channel URL."
                                        .to_string(),
                                )
                            })?;
                        (feed_url_for_channel(&cid), Some(cid), None)
                    }
                    ListSource::Playlist => {
                        let Some(pl) = input.playlist.as_deref() else {
                            return Err(ConnectorError::InvalidParams(
                                "source='playlist' requires 'playlist'".to_string(),
                            ));
                        };
                        let pid = extract_playlist_id_from_str(pl).ok_or_else(|| {
                            ConnectorError::InvalidInput(
                                "Could not parse playlist ID from playlist input. Provide a playlist ID or playlist URL."
                                    .to_string(),
                            )
                        })?;
                        (feed_url_for_playlist(&pid), None, Some(pid))
                    }
                };

                let xml = client
                    .get(&feed_url)
                    .send()
                    .await
                    .map_err(ConnectorError::HttpRequest)?
                    .text()
                    .await
                    .map_err(ConnectorError::HttpRequest)?;

                let mut videos = parse_youtube_atom_feed(&xml)?;

                let after = if let Some(days) = input.published_within_days {
                    Some(Utc::now() - Duration::days(days as i64))
                } else {
                    input.published_after.as_deref().and_then(parse_rfc3339)
                };
                if let Some(after) = after {
                    videos.retain(|v| {
                        v.published_at
                            .as_deref()
                            .and_then(parse_rfc3339)
                            .map(|dt| dt >= after)
                            .unwrap_or(false)
                    });
                }

                videos.sort_by(|a, b| {
                    let ad = a
                        .published_at
                        .as_deref()
                        .and_then(parse_rfc3339)
                        .unwrap_or_else(|| DateTime::<Utc>::from_timestamp(0, 0).unwrap());
                    let bd = b
                        .published_at
                        .as_deref()
                        .and_then(parse_rfc3339)
                        .unwrap_or_else(|| DateTime::<Utc>::from_timestamp(0, 0).unwrap());
                    bd.cmp(&ad)
                });

                videos.truncate(limit);

                let out = ListVideosOutput {
                    videos,
                    source: input.source,
                    channel_id,
                    playlist_id,
                };
                let text = serde_json::to_string(&out)?;
                Ok(structured_result_with_text(&out, Some(text))?)
            }
            "resolve_channel" => {
                let input: ResolveChannelInput = serde_json::from_value(Value::Object(args_map))
                    .map_err(|e| ConnectorError::InvalidParams(e.to_string()))?;

                let limit = input.limit.clamp(1, 10) as usize;

                let client = HttpClient::builder()
                    .user_agent("rzn-datasourcer/0.2.x youtube-connector")
                    .timeout(std::time::Duration::from_secs(20))
                    .build()
                    .map_err(|e| ConnectorError::Other(e.to_string()))?;

                // If a concrete channel identifier was provided, normalize to UC... when possible.
                let resolved_channel_id = if let Some(ch) = input.channel.as_deref() {
                    resolve_channel_id_best_effort(&client, ch).await
                } else {
                    None
                };

                // If query is provided, return ranked candidates.
                let mut candidates: Vec<ChannelCandidate> = Vec::new();
                if let Some(q) = input.query.as_deref() {
                    let qn = normalize_ws(q);
                    if !qn.is_empty() {
                        let youtube =
                            YouTube::new().map_err(|e| ConnectorError::Other(e.to_string()))?;
                        let search_options = SearchOptions {
                            limit: limit as u64,
                            search_type: SearchType::Channel,
                            ..Default::default()
                        };

                        let results: Vec<SearchResult> =
                            AssertUnwindSafe(youtube.search(&qn, Some(&search_options)))
                                .catch_unwind()
                                .await
                                .map_err(|_| {
                                    ConnectorError::Other("YouTube search panicked".to_string())
                                })?
                                .map_err(|e| ConnectorError::Other(e.to_string()))?;

                        for r in results {
                            if let SearchResult::Channel(channel) = r {
                                let mapped: ChannelSearchResult = channel.into();
                                let score = score_channel_candidate(
                                    &qn,
                                    &mapped.title,
                                    mapped.verified,
                                    mapped.subscribers,
                                    input.prefer_verified,
                                );
                                candidates.push(ChannelCandidate {
                                    channel_id: mapped.id,
                                    title: mapped.title,
                                    url: mapped.url,
                                    verified: mapped.verified,
                                    subscribers: mapped.subscribers,
                                    score,
                                });
                            }
                        }

                        candidates.sort_by(|a, b| {
                            b.score.partial_cmp(&a.score).unwrap_or(Ordering::Equal)
                        });
                        candidates.truncate(limit);
                    }
                }

                let recommended = candidates.first().cloned();

                let out = ResolveChannelOutput {
                    recommended,
                    candidates,
                    resolved_channel_id,
                };
                let text = serde_json::to_string(&out)?;
                Ok(structured_result_with_text(&out, Some(text))?)
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

fn extract_channel_id_from_str(s: &str) -> Option<String> {
    let trimmed = s.trim();
    if trimmed.starts_with("UC") && trimmed.len() >= 20 {
        return Some(trimmed.to_string());
    }

    if let Ok(url) = Url::parse(trimmed) {
        if let Some(seg) = url.path_segments() {
            let segs: Vec<_> = seg.collect();
            if let Some(idx) = segs.iter().position(|p| *p == "channel") {
                if let Some(cid) = segs.get(idx + 1) {
                    if cid.starts_with("UC") {
                        return Some((*cid).to_string());
                    }
                }
            }
        }
    }
    None
}

fn extract_playlist_id_from_str(s: &str) -> Option<String> {
    let trimmed = s.trim();
    if trimmed.starts_with("PL") || trimmed.starts_with("UU") || trimmed.starts_with("OLAK5") {
        return Some(trimmed.to_string());
    }
    if let Ok(url) = Url::parse(trimmed) {
        for (k, v) in url.query_pairs() {
            if k == "list" && !v.is_empty() {
                return Some(v.to_string());
            }
        }
    }
    None
}

async fn resolve_channel_id_best_effort(client: &HttpClient, channel: &str) -> Option<String> {
    if let Some(cid) = extract_channel_id_from_str(channel) {
        return Some(cid);
    }

    let trimmed = channel.trim();
    let url = if trimmed.starts_with("@") {
        format!("https://www.youtube.com/{}", trimmed)
    } else if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        trimmed.to_string()
    } else {
        format!("https://www.youtube.com/{}", trimmed)
    };

    let html = client.get(url).send().await.ok()?.text().await.ok()?;

    static RE: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r#""channelId"\s*:\s*"(?P<id>UC[a-zA-Z0-9_-]{10,})""#).expect("channelId regex")
    });
    RE.captures(&html)
        .and_then(|c| c.name("id").map(|m| m.as_str().to_string()))
}

fn feed_url_for_channel(channel_id: &str) -> String {
    format!(
        "https://www.youtube.com/feeds/videos.xml?channel_id={}",
        channel_id
    )
}

fn feed_url_for_playlist(playlist_id: &str) -> String {
    format!(
        "https://www.youtube.com/feeds/videos.xml?playlist_id={}",
        playlist_id
    )
}

fn parse_youtube_atom_feed(xml: &str) -> Result<Vec<ListedVideo>, ConnectorError> {
    let mut reader = XmlReader::from_str(xml);
    reader.trim_text(true);

    let mut buf = Vec::new();
    let mut in_entry = false;
    let mut current_tag: Option<String> = None;

    let mut cur_video_id: Option<String> = None;
    let mut cur_title: Option<String> = None;
    let mut cur_published: Option<String> = None;
    let mut cur_author: Option<String> = None;

    let mut out: Vec<ListedVideo> = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(XmlEvent::Start(e)) => {
                let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
                if tag.ends_with("entry") {
                    in_entry = true;
                    cur_video_id = None;
                    cur_title = None;
                    cur_published = None;
                    cur_author = None;
                    current_tag = None;
                } else if in_entry
                    && matches!(
                        tag.as_str(),
                        "yt:videoId" | "title" | "published" | "updated" | "name"
                    )
                {
                    current_tag = Some(tag);
                }
            }
            Ok(XmlEvent::Text(e)) => {
                if !in_entry {
                    buf.clear();
                    continue;
                }
                let Some(tag) = current_tag.as_deref() else {
                    buf.clear();
                    continue;
                };
                let text = e.unescape().unwrap_or(Cow::Borrowed("")).to_string();
                match tag {
                    "yt:videoId" => cur_video_id = Some(text),
                    "title" => cur_title = Some(text),
                    "published" => cur_published = Some(text),
                    "updated" => {
                        if cur_published.is_none() {
                            cur_published = Some(text);
                        }
                    }
                    "name" => cur_author = Some(text),
                    _ => {}
                }
            }
            Ok(XmlEvent::End(e)) => {
                let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
                if tag.ends_with("entry") && in_entry {
                    in_entry = false;
                    current_tag = None;
                    if let (Some(id), Some(title)) = (cur_video_id.take(), cur_title.take()) {
                        out.push(ListedVideo {
                            url: format!("https://www.youtube.com/watch?v={}", id),
                            id,
                            title,
                            published_at: cur_published.take(),
                            channel_title: cur_author.take(),
                        });
                    }
                } else if matches!(
                    tag.as_str(),
                    "yt:videoId" | "title" | "published" | "updated" | "name"
                ) {
                    current_tag = None;
                }
            }
            Ok(XmlEvent::Eof) => break,
            Err(e) => {
                return Err(ConnectorError::Other(format!(
                    "Failed to parse YouTube feed XML: {}",
                    e
                )));
            }
            _ => {}
        }
        buf.clear();
    }

    Ok(out)
}

fn parse_rfc3339(s: &str) -> Option<DateTime<Utc>> {
    chrono::DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}

fn normalize_ws(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn string_tokens(s: &str) -> Vec<String> {
    normalize_ws(&s.to_lowercase())
        .split_whitespace()
        .map(|t| t.trim_matches(|c: char| !c.is_alphanumeric()).to_string())
        .filter(|t| !t.is_empty())
        .collect()
}

fn token_overlap_score(query: &str, title: &str) -> f64 {
    let q = string_tokens(query);
    let t = string_tokens(title);
    if q.is_empty() || t.is_empty() {
        return 0.0;
    }
    let mut overlap = 0usize;
    for qt in &q {
        if t.iter().any(|tt| tt == qt) {
            overlap += 1;
        }
    }
    overlap as f64 / (q.len() as f64)
}

fn score_channel_candidate(
    query: &str,
    title: &str,
    verified: bool,
    subscribers: u64,
    prefer_verified: bool,
) -> f64 {
    let overlap = token_overlap_score(query, title);
    let mut score = overlap * 10.0;
    if prefer_verified && verified {
        score += 3.0;
    }
    // Subscribers saturate quickly; use log scale.
    let subs = (subscribers as f64).max(1.0);
    score += subs.log10().min(8.0);
    score
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
