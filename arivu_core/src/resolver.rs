//! Smart input resolver that detects URLs, IDs, and queries and routes them to the appropriate connector.
//!
//! This module provides a pattern-matching layer on top of connectors. Given an arbitrary input string,
//! it determines which connector and tool to use, and extracts the relevant parameters.
//!
//! # Example
//!
//! ```rust,ignore
//! use arivu_core::resolver::{SmartResolver, ResolvedAction};
//!
//! let resolver = SmartResolver::new();
//!
//! // YouTube URL -> get_video_details
//! let action = resolver.resolve("https://www.youtube.com/watch?v=dQw4w9WgXcQ");
//! assert_eq!(action.connector, "youtube");
//! assert_eq!(action.tool, "get_video_details");
//!
//! // PubMed ID -> get_article
//! let action = resolver.resolve("PMID:12345678");
//! assert_eq!(action.connector, "pubmed");
//!
//! // ArXiv ID -> get_paper
//! let action = resolver.resolve("arXiv:2301.07041");
//! assert_eq!(action.connector, "arxiv");
//! ```

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A resolved action ready to be executed against a connector
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedAction {
    /// The connector to use (e.g., "youtube", "pubmed")
    pub connector: String,
    /// The tool to call on the connector (e.g., "get_video_details", "search")
    pub tool: String,
    /// Arguments to pass to the tool
    pub arguments: HashMap<String, serde_json::Value>,
    /// Confidence score (0.0 - 1.0) for this match
    pub confidence: f32,
    /// Human-readable description of what was detected
    pub description: String,
}

/// Pattern definition for matching inputs
#[derive(Debug, Clone)]
pub struct InputPattern {
    /// Unique identifier for this pattern
    pub id: &'static str,
    /// The connector this pattern routes to
    pub connector: &'static str,
    /// The tool to call when matched
    pub tool: &'static str,
    /// Regex pattern to match against input
    pub pattern: Regex,
    /// Names of capture groups to extract as arguments
    pub captures: &'static [&'static str],
    /// How to map captures to tool arguments (capture_name -> arg_name)
    pub arg_mapping: &'static [(&'static str, &'static str)],
    /// Priority (higher = checked first)
    pub priority: u32,
    /// Human-readable description
    pub description: &'static str,
}

/// Smart resolver that matches inputs to connector actions
pub struct SmartResolver {
    patterns: Vec<InputPattern>,
}

impl Default for SmartResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl SmartResolver {
    /// Create a new resolver with all default patterns
    pub fn new() -> Self {
        Self {
            patterns: build_default_patterns(),
        }
    }

    /// Resolve an input string to an action
    ///
    /// Returns `None` if no pattern matches the input.
    pub fn resolve(&self, input: &str) -> Option<ResolvedAction> {
        let input = input.trim();

        for pattern in &self.patterns {
            if let Some(captures) = pattern.pattern.captures(input) {
                let mut arguments = HashMap::new();

                // Extract captures and map to arguments
                for (capture_name, arg_name) in pattern.arg_mapping {
                    if let Some(m) = captures.name(capture_name) {
                        arguments.insert(
                            arg_name.to_string(),
                            serde_json::Value::String(m.as_str().to_string()),
                        );
                    }
                }

                return Some(ResolvedAction {
                    connector: pattern.connector.to_string(),
                    tool: pattern.tool.to_string(),
                    arguments,
                    confidence: 1.0,
                    description: pattern.description.to_string(),
                });
            }
        }

        None
    }

    /// Resolve input, returning all possible matches sorted by confidence
    pub fn resolve_all(&self, input: &str) -> Vec<ResolvedAction> {
        let input = input.trim();
        let mut results = Vec::new();

        for pattern in &self.patterns {
            if let Some(captures) = pattern.pattern.captures(input) {
                let mut arguments = HashMap::new();

                for (capture_name, arg_name) in pattern.arg_mapping {
                    if let Some(m) = captures.name(capture_name) {
                        arguments.insert(
                            arg_name.to_string(),
                            serde_json::Value::String(m.as_str().to_string()),
                        );
                    }
                }

                results.push(ResolvedAction {
                    connector: pattern.connector.to_string(),
                    tool: pattern.tool.to_string(),
                    arguments,
                    confidence: 1.0,
                    description: pattern.description.to_string(),
                });
            }
        }

        results
    }

    /// Check if an input matches any pattern
    pub fn can_resolve(&self, input: &str) -> bool {
        let input = input.trim();
        self.patterns.iter().any(|p| p.pattern.is_match(input))
    }

    /// Get list of all supported patterns (for documentation/help)
    pub fn list_patterns(&self) -> Vec<PatternInfo> {
        self.patterns
            .iter()
            .map(|p| PatternInfo {
                id: p.id.to_string(),
                connector: p.connector.to_string(),
                tool: p.tool.to_string(),
                description: p.description.to_string(),
                example: get_pattern_example(p.id),
            })
            .collect()
    }
}

/// Information about a pattern for documentation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternInfo {
    pub id: String,
    pub connector: String,
    pub tool: String,
    pub description: String,
    pub example: String,
}

/// Build the default set of patterns
fn build_default_patterns() -> Vec<InputPattern> {
    let mut patterns = vec![
        // === YouTube ===
        InputPattern {
            id: "youtube_url_watch",
            connector: "youtube",
            tool: "get_video_details",
            pattern: Regex::new(r"(?:https?://)?(?:www\.)?youtube\.com/watch\?v=(?P<video_id>[a-zA-Z0-9_-]{11})").unwrap(),
            captures: &["video_id"],
            arg_mapping: &[("video_id", "video_id")],
            priority: 100,
            description: "YouTube video URL (youtube.com/watch?v=...)",
        },
        InputPattern {
            id: "youtube_url_short",
            connector: "youtube",
            tool: "get_video_details",
            pattern: Regex::new(r"(?:https?://)?youtu\.be/(?P<video_id>[a-zA-Z0-9_-]{11})").unwrap(),
            captures: &["video_id"],
            arg_mapping: &[("video_id", "video_id")],
            priority: 100,
            description: "YouTube short URL (youtu.be/...)",
        },
        InputPattern {
            id: "youtube_url_embed",
            connector: "youtube",
            tool: "get_video_details",
            pattern: Regex::new(r"(?:https?://)?(?:www\.)?youtube\.com/embed/(?P<video_id>[a-zA-Z0-9_-]{11})").unwrap(),
            captures: &["video_id"],
            arg_mapping: &[("video_id", "video_id")],
            priority: 100,
            description: "YouTube embed URL",
        },
        InputPattern {
            id: "youtube_video_id",
            connector: "youtube",
            tool: "get_video_details",
            pattern: Regex::new(r"^(?P<video_id>[a-zA-Z0-9_-]{11})$").unwrap(),
            captures: &["video_id"],
            arg_mapping: &[("video_id", "video_id")],
            priority: 10, // Low priority - only match bare 11-char strings
            description: "YouTube video ID (11 characters)",
        },
        InputPattern {
            id: "youtube_playlist",
            connector: "youtube",
            tool: "get_playlist",
            pattern: Regex::new(r"(?:https?://)?(?:www\.)?youtube\.com/playlist\?list=(?P<playlist_id>[a-zA-Z0-9_-]+)").unwrap(),
            captures: &["playlist_id"],
            arg_mapping: &[("playlist_id", "playlist_id")],
            priority: 100,
            description: "YouTube playlist URL",
        },
        InputPattern {
            id: "youtube_channel",
            connector: "youtube",
            tool: "get_channel",
            pattern: Regex::new(r"(?:https?://)?(?:www\.)?youtube\.com/(?:@|channel/)(?P<channel_id>[a-zA-Z0-9_-]+)").unwrap(),
            captures: &["channel_id"],
            arg_mapping: &[("channel_id", "channel_id")],
            priority: 100,
            description: "YouTube channel URL",
        },

        // === Hacker News ===
        InputPattern {
            id: "hackernews_url",
            connector: "hackernews",
            tool: "get_post",
            pattern: Regex::new(r"(?:https?://)?news\.ycombinator\.com/item\?id=(?P<item_id>\d+)").unwrap(),
            captures: &["item_id"],
            arg_mapping: &[("item_id", "id")],
            priority: 100,
            description: "Hacker News item URL",
        },
        InputPattern {
            id: "hackernews_id",
            connector: "hackernews",
            tool: "get_post",
            pattern: Regex::new(r"^(?:hn:|HN:)?(?P<item_id>\d{7,9})$").unwrap(),
            captures: &["item_id"],
            arg_mapping: &[("item_id", "id")],
            priority: 50,
            description: "Hacker News item ID (7-9 digits, optionally prefixed with hn:)",
        },

        // === ArXiv ===
        InputPattern {
            id: "arxiv_url",
            connector: "arxiv",
            tool: "get_paper",
            pattern: Regex::new(r"(?:https?://)?arxiv\.org/(?:abs|pdf)/(?P<arxiv_id>\d{4}\.\d{4,5}(?:v\d+)?)").unwrap(),
            captures: &["arxiv_id"],
            arg_mapping: &[("arxiv_id", "id")],
            priority: 100,
            description: "ArXiv paper URL",
        },
        InputPattern {
            id: "arxiv_id",
            connector: "arxiv",
            tool: "get_paper",
            pattern: Regex::new(r"^(?:arXiv:|arxiv:)?(?P<arxiv_id>\d{4}\.\d{4,5}(?:v\d+)?)$").unwrap(),
            captures: &["arxiv_id"],
            arg_mapping: &[("arxiv_id", "id")],
            priority: 90,
            description: "ArXiv paper ID (e.g., 2301.07041 or arXiv:2301.07041)",
        },
        InputPattern {
            id: "arxiv_old_id",
            connector: "arxiv",
            tool: "get_paper",
            pattern: Regex::new(r"^(?:arXiv:|arxiv:)?(?P<arxiv_id>[a-z-]+/\d{7})$").unwrap(),
            captures: &["arxiv_id"],
            arg_mapping: &[("arxiv_id", "id")],
            priority: 90,
            description: "ArXiv old-style ID (e.g., hep-th/9901001)",
        },

        // === PubMed ===
        InputPattern {
            id: "pubmed_url",
            connector: "pubmed",
            tool: "get_article",
            pattern: Regex::new(r"(?:https?://)?(?:www\.)?(?:ncbi\.nlm\.nih\.gov/pubmed/|pubmed\.ncbi\.nlm\.nih\.gov/)(?P<pmid>\d+)").unwrap(),
            captures: &["pmid"],
            arg_mapping: &[("pmid", "pmid")],
            priority: 100,
            description: "PubMed article URL",
        },
        InputPattern {
            id: "pubmed_id",
            connector: "pubmed",
            tool: "get_article",
            pattern: Regex::new(r"^(?:PMID:|pmid:|PubMed:)?(?P<pmid>\d{7,8})$").unwrap(),
            captures: &["pmid"],
            arg_mapping: &[("pmid", "pmid")],
            priority: 80,
            description: "PubMed ID (7-8 digits, optionally prefixed with PMID:)",
        },

        // === DOI ===
        InputPattern {
            id: "doi_url",
            connector: "semantic-scholar",
            tool: "get_paper",
            pattern: Regex::new(r"(?:https?://)?(?:dx\.)?doi\.org/(?P<doi>10\.\d{4,}/[^\s]+)").unwrap(),
            captures: &["doi"],
            arg_mapping: &[("doi", "paper_id")],
            priority: 100,
            description: "DOI URL (doi.org/...)",
        },
        InputPattern {
            id: "doi_bare",
            connector: "semantic-scholar",
            tool: "get_paper",
            pattern: Regex::new(r"^(?:doi:|DOI:)?(?P<doi>10\.\d{4,}/[^\s]+)$").unwrap(),
            captures: &["doi"],
            arg_mapping: &[("doi", "paper_id")],
            priority: 90,
            description: "DOI (e.g., 10.1234/example)",
        },

        // === Semantic Scholar ===
        InputPattern {
            id: "semantic_scholar_url",
            connector: "semantic-scholar",
            tool: "get_paper",
            pattern: Regex::new(r"(?:https?://)?(?:www\.)?semanticscholar\.org/paper/[^/]+/(?P<paper_id>[a-f0-9]{40})").unwrap(),
            captures: &["paper_id"],
            arg_mapping: &[("paper_id", "paper_id")],
            priority: 100,
            description: "Semantic Scholar paper URL",
        },

        // === Wikipedia ===
        InputPattern {
            id: "wikipedia_url",
            connector: "wikipedia",
            tool: "get_page",
            pattern: Regex::new(r"(?:https?://)?(?P<lang>[a-z]{2})\.wikipedia\.org/wiki/(?P<title>[^\s?#]+)").unwrap(),
            captures: &["lang", "title"],
            arg_mapping: &[("title", "title")],
            priority: 100,
            description: "Wikipedia article URL",
        },

        // === GitHub ===
        InputPattern {
            id: "github_repo_url",
            connector: "github",
            tool: "get_repository",
            pattern: Regex::new(r"(?:https?://)?github\.com/(?P<owner>[a-zA-Z0-9_-]+)/(?P<repo>[a-zA-Z0-9_.-]+)/?$").unwrap(),
            captures: &["owner", "repo"],
            arg_mapping: &[("owner", "owner"), ("repo", "repo")],
            priority: 100,
            description: "GitHub repository URL",
        },
        InputPattern {
            id: "github_issue_url",
            connector: "github",
            tool: "get_issue",
            pattern: Regex::new(r"(?:https?://)?github\.com/(?P<owner>[a-zA-Z0-9_-]+)/(?P<repo>[a-zA-Z0-9_.-]+)/issues/(?P<issue_number>\d+)").unwrap(),
            captures: &["owner", "repo", "issue_number"],
            arg_mapping: &[("owner", "owner"), ("repo", "repo"), ("issue_number", "issue_number")],
            priority: 100,
            description: "GitHub issue URL",
        },
        InputPattern {
            id: "github_pr_url",
            connector: "github",
            tool: "get_pull_request",
            pattern: Regex::new(r"(?:https?://)?github\.com/(?P<owner>[a-zA-Z0-9_-]+)/(?P<repo>[a-zA-Z0-9_.-]+)/pull/(?P<pr_number>\d+)").unwrap(),
            captures: &["owner", "repo", "pr_number"],
            arg_mapping: &[("owner", "owner"), ("repo", "repo"), ("pr_number", "pr_number")],
            priority: 100,
            description: "GitHub pull request URL",
        },
        InputPattern {
            id: "github_repo_shorthand",
            connector: "github",
            tool: "get_repository",
            pattern: Regex::new(r"^(?P<owner>[a-zA-Z0-9_-]+)/(?P<repo>[a-zA-Z0-9_.-]+)$").unwrap(),
            captures: &["owner", "repo"],
            arg_mapping: &[("owner", "owner"), ("repo", "repo")],
            priority: 50,
            description: "GitHub repository shorthand (owner/repo)",
        },

        // === Reddit ===
        InputPattern {
            id: "reddit_post_url",
            connector: "reddit",
            tool: "get_post",
            pattern: Regex::new(r"(?:https?://)?(?:www\.)?reddit\.com/r/(?P<subreddit>[a-zA-Z0-9_]+)/comments/(?P<post_id>[a-z0-9]+)").unwrap(),
            captures: &["subreddit", "post_id"],
            arg_mapping: &[("subreddit", "subreddit"), ("post_id", "post_id")],
            priority: 100,
            description: "Reddit post URL",
        },
        InputPattern {
            id: "reddit_subreddit_url",
            connector: "reddit",
            tool: "get_subreddit",
            pattern: Regex::new(r"(?:https?://)?(?:www\.)?reddit\.com/r/(?P<subreddit>[a-zA-Z0-9_]+)/?$").unwrap(),
            captures: &["subreddit"],
            arg_mapping: &[("subreddit", "subreddit")],
            priority: 100,
            description: "Reddit subreddit URL",
        },
        InputPattern {
            id: "reddit_subreddit_shorthand",
            connector: "reddit",
            tool: "get_subreddit",
            pattern: Regex::new(r"^r/(?P<subreddit>[a-zA-Z0-9_]+)$").unwrap(),
            captures: &["subreddit"],
            arg_mapping: &[("subreddit", "subreddit")],
            priority: 80,
            description: "Reddit subreddit shorthand (r/name)",
        },

        // === X (Twitter) ===
        InputPattern {
            id: "twitter_tweet_url",
            connector: "x",
            tool: "get_tweet",
            pattern: Regex::new(r"(?:https?://)?(?:www\.)?(?:twitter\.com|x\.com)/(?P<username>[a-zA-Z0-9_]+)/status/(?P<tweet_id>\d+)").unwrap(),
            captures: &["username", "tweet_id"],
            arg_mapping: &[("tweet_id", "tweet_id")],
            priority: 100,
            description: "X/Twitter tweet URL",
        },
        InputPattern {
            id: "twitter_profile_url",
            connector: "x",
            tool: "get_profile",
            pattern: Regex::new(r"(?:https?://)?(?:www\.)?(?:twitter\.com|x\.com)/(?P<username>[a-zA-Z0-9_]+)/?$").unwrap(),
            captures: &["username"],
            arg_mapping: &[("username", "username")],
            priority: 90,
            description: "X/Twitter profile URL",
        },
        InputPattern {
            id: "twitter_handle",
            connector: "x",
            tool: "get_profile",
            pattern: Regex::new(r"^@(?P<username>[a-zA-Z0-9_]+)$").unwrap(),
            captures: &["username"],
            arg_mapping: &[("username", "username")],
            priority: 80,
            description: "X/Twitter handle (@username)",
        },

        // === Generic Web URLs ===
        InputPattern {
            id: "web_url",
            connector: "web",
            tool: "fetch",
            pattern: Regex::new(r"^(?P<url>https?://[^\s]+)$").unwrap(),
            captures: &["url"],
            arg_mapping: &[("url", "url")],
            priority: 1, // Lowest priority - catch-all for URLs
            description: "Generic web URL",
        },
    ];

    // Sort by priority (highest first)
    patterns.sort_by(|a, b| b.priority.cmp(&a.priority));
    patterns
}

/// Get an example input for a pattern
fn get_pattern_example(pattern_id: &str) -> String {
    match pattern_id {
        "youtube_url_watch" => "https://www.youtube.com/watch?v=dQw4w9WgXcQ",
        "youtube_url_short" => "https://youtu.be/dQw4w9WgXcQ",
        "youtube_url_embed" => "https://www.youtube.com/embed/dQw4w9WgXcQ",
        "youtube_video_id" => "dQw4w9WgXcQ",
        "youtube_playlist" => {
            "https://www.youtube.com/playlist?list=PLrAXtmErZgOeiKm4sgNOknGvNjby9efdf"
        }
        "youtube_channel" => "https://www.youtube.com/@veritasium",
        "hackernews_url" => "https://news.ycombinator.com/item?id=38500000",
        "hackernews_id" => "38500000",
        "arxiv_url" => "https://arxiv.org/abs/2301.07041",
        "arxiv_id" => "arXiv:2301.07041",
        "arxiv_old_id" => "hep-th/9901001",
        "pubmed_url" => "https://pubmed.ncbi.nlm.nih.gov/12345678",
        "pubmed_id" => "PMID:12345678",
        "doi_url" => "https://doi.org/10.1038/nature12373",
        "doi_bare" => "10.1038/nature12373",
        "semantic_scholar_url" => {
            "https://www.semanticscholar.org/paper/Attention-Is-All-You-Need/abc123..."
        }
        "wikipedia_url" => "https://en.wikipedia.org/wiki/Rust_(programming_language)",
        "github_repo_url" => "https://github.com/rust-lang/rust",
        "github_issue_url" => "https://github.com/rust-lang/rust/issues/12345",
        "github_pr_url" => "https://github.com/rust-lang/rust/pull/12345",
        "github_repo_shorthand" => "rust-lang/rust",
        "reddit_post_url" => "https://www.reddit.com/r/rust/comments/abc123",
        "reddit_subreddit_url" => "https://www.reddit.com/r/rust",
        "reddit_subreddit_shorthand" => "r/rust",
        "twitter_tweet_url" => "https://x.com/elonmusk/status/1234567890",
        "twitter_profile_url" => "https://x.com/elonmusk",
        "twitter_handle" => "@elonmusk",
        "web_url" => "https://example.com/page",
        _ => "",
    }
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_youtube_urls() {
        let resolver = SmartResolver::new();

        // Standard watch URL
        let action = resolver
            .resolve("https://www.youtube.com/watch?v=dQw4w9WgXcQ")
            .unwrap();
        assert_eq!(action.connector, "youtube");
        assert_eq!(action.tool, "get_video_details");
        assert_eq!(action.arguments.get("video_id").unwrap(), "dQw4w9WgXcQ");

        // Short URL
        let action = resolver.resolve("https://youtu.be/dQw4w9WgXcQ").unwrap();
        assert_eq!(action.connector, "youtube");
        assert_eq!(action.arguments.get("video_id").unwrap(), "dQw4w9WgXcQ");

        // Bare video ID
        let action = resolver.resolve("dQw4w9WgXcQ").unwrap();
        assert_eq!(action.connector, "youtube");
    }

    #[test]
    fn test_arxiv() {
        let resolver = SmartResolver::new();

        // ArXiv URL
        let action = resolver
            .resolve("https://arxiv.org/abs/2301.07041")
            .unwrap();
        assert_eq!(action.connector, "arxiv");
        assert_eq!(action.arguments.get("id").unwrap(), "2301.07041");

        // ArXiv ID with prefix
        let action = resolver.resolve("arXiv:2301.07041").unwrap();
        assert_eq!(action.connector, "arxiv");
        assert_eq!(action.arguments.get("id").unwrap(), "2301.07041");
    }

    #[test]
    fn test_pubmed() {
        let resolver = SmartResolver::new();

        // PubMed URL
        let action = resolver
            .resolve("https://pubmed.ncbi.nlm.nih.gov/12345678")
            .unwrap();
        assert_eq!(action.connector, "pubmed");
        assert_eq!(action.arguments.get("pmid").unwrap(), "12345678");

        // PMID prefix
        let action = resolver.resolve("PMID:12345678").unwrap();
        assert_eq!(action.connector, "pubmed");
    }

    #[test]
    fn test_github() {
        let resolver = SmartResolver::new();

        // Repo URL
        let action = resolver
            .resolve("https://github.com/rust-lang/rust")
            .unwrap();
        assert_eq!(action.connector, "github");
        assert_eq!(action.tool, "get_repository");

        // Shorthand
        let action = resolver.resolve("rust-lang/rust").unwrap();
        assert_eq!(action.connector, "github");
    }

    #[test]
    fn test_hackernews() {
        let resolver = SmartResolver::new();

        let action = resolver
            .resolve("https://news.ycombinator.com/item?id=38500000")
            .unwrap();
        assert_eq!(action.connector, "hackernews");
        assert_eq!(action.arguments.get("id").unwrap(), "38500000");
    }

    #[test]
    fn test_priority() {
        let resolver = SmartResolver::new();

        // GitHub URL should match github, not generic web
        let action = resolver
            .resolve("https://github.com/rust-lang/rust")
            .unwrap();
        assert_eq!(action.connector, "github");

        // Random URL should fall back to web
        let action = resolver.resolve("https://example.com/page").unwrap();
        assert_eq!(action.connector, "web");
    }
}
