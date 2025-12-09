use crate::error::ConnectorError;
use chrono::{Datelike, Duration, Utc};
#[cfg(feature = "browser-cookies")]
use publicsuffix::{List, Psl};
use rmcp::model::CallToolResult;
#[cfg(feature = "browser-cookies")]
use rookie::{brave, chrome, common::enums::CookieToString, firefox, safari};
use serde::Serialize;
use serde_json::{Map as JsonMap, Value as JsonValue};
use thiserror::Error;
#[cfg(feature = "browser-cookies")]
use url::Url;

#[cfg(feature = "browser-cookies")]
#[derive(Debug, Clone)]
pub enum Browser {
    Firefox,
    Chrome,
    Safari,
    Brave,
}

#[cfg(not(feature = "browser-cookies"))]
#[derive(Debug, Clone)]
pub enum Browser {
    Firefox,
    Chrome,
    Safari,
    Brave,
}

#[derive(Debug, Error)]
pub enum ScraperError {
    CookieError(String),
}

impl std::fmt::Display for ScraperError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScraperError::CookieError(msg) => write!(f, "Cookie error: {}", msg),
        }
    }
}

#[cfg(feature = "browser-cookies")]
pub async fn get_cookies(browser: Browser, domain: String) -> Result<String, ScraperError> {
    // Check if domain has a scheme, if not add https://
    let domain_with_scheme = if !domain.starts_with("http://") && !domain.starts_with("https://") {
        format!("https://{}", domain)
    } else {
        domain.to_string()
    };

    let url = Url::parse(&domain_with_scheme)
        .map_err(|e| ScraperError::CookieError(format!("Invalid URL: {}", e)))?;
    let list = List::from_bytes(&include_bytes!("../public_suffix_list.dat")[..]).map_err(|e| {
        ScraperError::CookieError(format!("Failed to parse public suffix list: {}", e))
    })?;
    let domain_str = url
        .host_str()
        .ok_or_else(|| ScraperError::CookieError("URL has no host".to_string()))?;
    let domain = list.domain(domain_str.as_bytes()).ok_or_else(|| {
        ScraperError::CookieError(format!("Could not extract domain from: {}", domain_str))
    })?;

    // Convert suffix bytes to string

    let domain_str = String::from_utf8_lossy(domain.as_bytes()).to_string();
    //    println!("Domain: {}", domain_str);

    let cookies = match browser {
        Browser::Firefox => firefox(Some(vec![domain_str.to_string()])),
        Browser::Chrome => chrome(Some(vec![domain_str.to_string()])),
        Browser::Safari => safari(Some(vec![domain_str.to_string()])),
        Browser::Brave => brave(Some(vec![domain_str.to_string()])),
    }
    .map_err(|e| ScraperError::CookieError(e.to_string()))?;
    //   println!("Cookies: {:?}", cookies);
    Ok(cookies.to_string())
}

#[cfg(not(feature = "browser-cookies"))]
pub async fn get_cookies(_browser: Browser, _domain: String) -> Result<String, ScraperError> {
    Err(ScraperError::CookieError(
        "browser-cookies feature not enabled".to_string(),
    ))
}

#[cfg(feature = "browser-cookies")]
pub async fn match_browser(browser: String) -> Result<Browser, ConnectorError> {
    match browser.as_str() {
        "firefox" => Ok(Browser::Firefox),
        "chrome" => Ok(Browser::Chrome),
        "safari" => Ok(Browser::Safari),
        "brave" => Ok(Browser::Brave),
        _ => Err(ConnectorError::Other(format!(
            "Invalid browser: {}",
            browser
        ))),
    }
}

#[cfg(not(feature = "browser-cookies"))]
pub async fn match_browser(_browser: String) -> Result<Browser, ConnectorError> {
    Err(ConnectorError::Other(
        "browser-cookies feature not enabled".to_string(),
    ))
}

#[cfg(feature = "browser-cookies")]
pub fn get_domain(url: &str) -> Result<String, ConnectorError> {
    let url_with_scheme = if !url.starts_with("http://") && !url.starts_with("https://") {
        format!("https://{}", url)
    } else {
        url.to_string()
    };

    let url = Url::parse(&url_with_scheme)
        .map_err(|e| ConnectorError::Other(format!("Invalid URL: {}", e)))?;

    let host_str = url
        .host_str()
        .ok_or_else(|| ConnectorError::Other("URL has no host".to_string()))?;

    let list = List::from_bytes(&include_bytes!("../public_suffix_list.dat")[..])
        .map_err(|e| ConnectorError::Other(format!("Failed to parse public suffix list: {}", e)))?;

    let domain = list.domain(host_str.as_bytes()).ok_or_else(|| {
        ConnectorError::Other(format!("Could not extract domain from: {}", host_str))
    })?;

    // Convert suffix bytes to string
    let domain_str = String::from_utf8_lossy(domain.as_bytes()).to_string();
    Ok(domain_str)
}

#[cfg(not(feature = "browser-cookies"))]
pub fn get_domain(_url: &str) -> Result<String, ConnectorError> {
    Err(ConnectorError::Other(
        "browser-cookies feature not enabled".to_string(),
    ))
}

#[cfg(feature = "browser-cookies")]
pub fn get_user_agent(browser: Browser) -> String {
    match browser {
        Browser::Firefox => "Mozilla/5.0 (Macintosh; Intel Mac OS X 10.15; rv:136.0) Gecko/20100101 Firefox/136.0".to_string(),
        Browser::Chrome => "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/133.0.0.0 Safari/537.36".to_string(),
        Browser::Safari => "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/16.1 Safari/605.1.15".to_string(),
        Browser::Brave => "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36".to_string(),
    }
}

#[cfg(not(feature = "browser-cookies"))]
pub fn get_user_agent(_browser: Browser) -> String {
    // Return a generic UA; useful for minimal builds.
    "Mozilla/5.0".to_string()
}

pub fn strip_multiple_newlines(text: &str) -> String {
    let mut result = String::new();
    let mut in_code_block = false;
    let mut in_quote_block = false;
    let mut consecutive_newlines = 0;

    for line in text.lines() {
        // Check for code block markers
        if line.trim().starts_with("```") {
            in_code_block = !in_code_block;
            if !result.is_empty() {
                result.push('\n');
            }
            result.push_str(line);
            consecutive_newlines = 0;
            continue;
        }

        // Check for quote block
        if line.trim().starts_with('>') {
            in_quote_block = true;
        } else if in_quote_block && !line.trim().is_empty() {
            in_quote_block = false;
        }

        // Handle line based on context
        if line.trim().is_empty() {
            if !in_code_block && !in_quote_block {
                consecutive_newlines += 1;
                if consecutive_newlines <= 1 {
                    result.push('\n');
                }
            } else {
                // Preserve empty lines in code blocks and quotes
                result.push('\n');
            }
        } else {
            if !result.is_empty() {
                result.push('\n');
            }
            result.push_str(line);
            consecutive_newlines = 0;
        }
    }

    result
}

pub fn clean_html_entities(text: &str) -> String {
    let mut cleaned = text.to_string();
    // Try decoding multiple times in case of double-encoding
    for _ in 0..2 {
        let decoded = html_escape::decode_html_entities(&cleaned).into_owned();
        if decoded == cleaned {
            break;
        }
        cleaned = decoded;
    }

    // Handle any remaining common entities manually
    cleaned
        .replace("&#39;", "'")
        .replace("&quot;", "\"")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
}

//     html_escape::decode_html_entities(text).into_owned().replace("\n", " ").replace("&#39;", "'")
// }

/// Build a CallToolResult that carries only structured JSON (no text fallback).
/// This prioritizes first-class machine-readable results for modern MCP clients.
const RESULT_LIST_KEYS: &[&str] = &[
    "results",
    "articles",
    "items",
    "entries",
    "documents",
    "records",
    "posts",
    "stories",
    "videos",
    "papers",
    "messages",
    "mailboxes",
    "conversations",
    "threads",
    "hits",
    "search_results",
    "content",
    "data",
];

const COUNT_KEYS: &[&str] = &[
    "total_results",
    "total_count",
    "count",
    "results_count",
    "result_count",
    "nbHits",
    "nb_hits",
    "match_count",
    "hits",
];

const QUERY_FIELD_KEYS: &[&str] = &[
    "query",
    "search_query",
    "term",
    "search_term",
    "keywords",
    "keyword",
    "q",
];

fn build_no_results_message(key: &str, query_hint: Option<String>) -> String {
    let label = match key {
        "data" | "results" | "total_results" | "total_count" | "count" | "nbHits" | "nb_hits"
        | "hits" | "result_count" | "results_count" => "results".to_string(),
        other => other.replace('_', " "),
    };

    match query_hint {
        Some(query) => format!("No {} found for \"{}\".", label, query),
        None => format!("No {} found for the requested input.", label),
    }
}

fn maybe_attach_no_results_message(map: &mut JsonMap<String, JsonValue>) -> Option<String> {
    // Any non-empty result list means we have data and should not set a no-results message.
    for key in RESULT_LIST_KEYS {
        if let Some(JsonValue::Array(items)) = map.get(*key) {
            if !items.is_empty() {
                return None;
            }
        }
    }

    // Capture a query hint if the payload includes one.
    let query_hint = map
        .iter()
        .find_map(|(key, value)| {
            if QUERY_FIELD_KEYS.iter().any(|candidate| candidate == key) {
                value.as_str().map(|s| s.trim().to_string())
            } else {
                None
            }
        })
        .filter(|s| !s.is_empty());

    let mut message: Option<String> = None;

    for key in RESULT_LIST_KEYS {
        if let Some(value) = map.get(*key) {
            match value {
                JsonValue::Array(items) if items.is_empty() => {
                    message = Some(build_no_results_message(key, query_hint.clone()));
                    break;
                }
                JsonValue::Null => {
                    message = Some(build_no_results_message(key, query_hint.clone()));
                    break;
                }
                JsonValue::String(s) if s.trim().is_empty() => {
                    message = Some(build_no_results_message(key, query_hint.clone()));
                    break;
                }
                JsonValue::Object(obj) if obj.is_empty() => {
                    message = Some(build_no_results_message(key, query_hint.clone()));
                    break;
                }
                JsonValue::Number(num) if num.as_u64() == Some(0) => {
                    message = Some(build_no_results_message(key, query_hint.clone()));
                    break;
                }
                _ => {}
            }
        }
    }

    if message.is_none() {
        if let Some(JsonValue::Array(items)) = map.get("data") {
            if items.is_empty() {
                message = Some(build_no_results_message("results", query_hint.clone()));
            }
        } else if let Some(JsonValue::Object(obj)) = map.get("data") {
            if obj.is_empty() {
                message = Some(build_no_results_message("results", query_hint.clone()));
            }
        }
    }

    if message.is_none() {
        for key in COUNT_KEYS {
            if let Some(value) = map.get(*key) {
                if value.as_u64() == Some(0) {
                    message = Some(build_no_results_message("results", query_hint.clone()));
                    break;
                }
                if let Some(as_str) = value.as_str() {
                    if as_str.trim() == "0" {
                        message = Some(build_no_results_message("results", query_hint.clone()));
                        break;
                    }
                }
            }
        }
    }

    if message.is_none() && map.is_empty() {
        message = Some(build_no_results_message("results", query_hint.clone()));
    }

    if let Some(message_text) = message.clone() {
        map.entry("message".to_string())
            .or_insert(JsonValue::String(message_text.clone()));
        map.entry("no_results".to_string())
            .or_insert(JsonValue::Bool(true));
    }

    message
}

pub fn structured_result_with_text<T: Serialize>(
    data: &T,
    _text_fallback: Option<String>,
) -> Result<CallToolResult, ConnectorError> {
    let value = serde_json::to_value(data).map_err(|e| ConnectorError::Other(e.to_string()))?;

    // Convert to an object map; if it's not an object, wrap under a `data` key.
    let mut map: JsonMap<String, JsonValue> = match value {
        JsonValue::Object(m) => m,
        other => {
            let mut m = JsonMap::new();
            m.insert("data".to_string(), other);
            m
        }
    };

    maybe_attach_no_results_message(&mut map);

    Ok(CallToolResult {
        content: Vec::new(),
        structured_content: Some(JsonValue::Object(map)),
        is_error: Some(false),
        meta: None,
    })
}

// --- Uniform search filter helpers for connectors ---

#[derive(Debug, Clone)]
pub struct SearchFilters {
    pub language: Option<String>,
    pub region: Option<String>,
    pub since: Option<String>,
    pub until: Option<String>,
    pub include_domains: Vec<String>,
    pub exclude_domains: Vec<String>,
}

fn ymd_string(days_from_now: i64) -> String {
    let d = Utc::now().date_naive() + Duration::days(days_from_now);
    format!("{:04}-{:02}-{:02}", d.year(), d.month(), d.day())
}

fn month_start_string() -> String {
    let d = Utc::now().date_naive();
    format!("{:04}-{:02}-{:02}", d.year(), d.month(), 1)
}

fn parse_date_preset(preset: &str) -> Option<(String, String)> {
    let p = preset.to_lowercase();
    match p.as_str() {
        "last_24_hours" | "past_day" => Some((ymd_string(-1), ymd_string(0))),
        "last_7_days" | "past_week" => Some((ymd_string(-7), ymd_string(0))),
        "last_30_days" | "past_month" => Some((ymd_string(-30), ymd_string(0))),
        "this_month" => Some((month_start_string(), ymd_string(0))),
        "last_365_days" | "past_year" => Some((ymd_string(-365), ymd_string(0))),
        _ => None,
    }
}

fn parse_locale(locale: &str) -> (Option<String>, Option<String>) {
    let loc = locale.replace('_', "-");
    let parts: Vec<&str> = loc.split('-').collect();
    match parts.len() {
        1 => (Some(parts[0].to_lowercase()), None),
        2 => (Some(parts[0].to_lowercase()), Some(parts[1].to_uppercase())),
        _ => (None, None),
    }
}

pub fn resolve_search_filters(args: &JsonMap<String, JsonValue>) -> SearchFilters {
    let mut language = args
        .get("language")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let mut region = args
        .get("region")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let mut since = args
        .get("since")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let mut until = args
        .get("until")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    if language.is_none() || region.is_none() {
        if let Some(loc) = args.get("locale").and_then(|v| v.as_str()) {
            let (lang, reg) = parse_locale(loc);
            if language.is_none() {
                language = lang;
            }
            if region.is_none() {
                region = reg;
            }
        }
    }

    if since.is_none() && until.is_none() {
        if let Some(preset) = args.get("date_preset").and_then(|v| v.as_str()) {
            if let Some((s, u)) = parse_date_preset(preset) {
                since = Some(s);
                until = Some(u);
            }
        }
    }

    let include_domains = args
        .get("include_domains")
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|s| s.as_str().map(|x| x.to_string()))
                .collect()
        })
        .unwrap_or_default();
    let exclude_domains = args
        .get("exclude_domains")
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|s| s.as_str().map(|x| x.to_string()))
                .collect()
        })
        .unwrap_or_default();

    SearchFilters {
        language,
        region,
        since,
        until,
        include_domains,
        exclude_domains,
    }
}

pub fn build_filters_clause(filters: &SearchFilters) -> String {
    let mut parts: Vec<String> = Vec::new();
    if let Some(v) = &filters.language {
        parts.push(format!("language={}", v));
    }
    if let Some(v) = &filters.region {
        parts.push(format!("region={}", v));
    }
    if let Some(v) = &filters.since {
        parts.push(format!("since={}", v));
    }
    if let Some(v) = &filters.until {
        parts.push(format!("until={}", v));
    }
    if !filters.include_domains.is_empty() {
        parts.push(format!("include_domains={:?}", filters.include_domains));
    }
    if !filters.exclude_domains.is_empty() {
        parts.push(format!("exclude_domains={:?}", filters.exclude_domains));
    }
    if parts.is_empty() {
        String::new()
    } else {
        format!("\nFilters: {}", parts.join("; "))
    }
}
