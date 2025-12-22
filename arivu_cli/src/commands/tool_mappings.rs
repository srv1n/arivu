use crate::commands::{CommandError, Result};
use serde_json::{json, Map, Value};

pub fn generic_get_tool_and_args(
    connector: &str,
    id: &str,
) -> Result<(&'static str, Map<String, Value>)> {
    match connector {
        "youtube" => Ok((
            "get",
            json!({ "video_id": id, "response_format": "detailed" })
                .as_object()
                .expect("json object")
                .clone(),
        )),
        "reddit" => {
            let post_url = if id.starts_with("http://") || id.starts_with("https://") {
                id.to_string()
            } else {
                format!("https://www.reddit.com/comments/{}", id)
            };
            Ok((
                "get",
                json!({ "post_url": post_url })
                    .as_object()
                    .expect("json object")
                    .clone(),
            ))
        }
        "hackernews" => {
            let parsed = id.parse::<u64>().map_err(|_| {
                CommandError::InvalidInput("Hacker News IDs must be numeric.".to_string())
            })?;
            Ok((
                "get_post",
                json!({ "id": parsed }).as_object().expect("json object").clone(),
            ))
        }
        "wikipedia" => Ok((
            "get_article",
            json!({ "title": id, "response_format": "detailed" })
                .as_object()
                .expect("json object")
                .clone(),
        )),
        "arxiv" => Ok((
            "get",
            json!({ "paper_id": id, "response_format": "detailed" })
                .as_object()
                .expect("json object")
                .clone(),
        )),
        "pubmed" => Ok((
            "get",
            json!({ "pmid": id, "response_format": "detailed" })
                .as_object()
                .expect("json object")
                .clone(),
        )),
        "semantic-scholar" | "semantic_scholar" => Ok((
            "get_paper_details",
            json!({ "paper_id": id, "response_format": "detailed" })
                .as_object()
                .expect("json object")
                .clone(),
        )),
        "github" => {
            let (owner, repo) = id.split_once('/').ok_or_else(|| {
                CommandError::InvalidInput("GitHub IDs must be in owner/repo form.".to_string())
            })?;
            Ok((
                "get_repository",
                json!({ "owner": owner, "repo": repo })
                    .as_object()
                    .expect("json object")
                    .clone(),
            ))
        }
        _ => Err(CommandError::InvalidInput(format!(
            "Connector '{}' is not supported by the generic `get` command. Use `arivu {0} --help` or `arivu tools {0}`.",
            connector
        ))),
    }
}

pub fn generic_search_tool_and_args(
    connector: &str,
    query: &str,
    limit: u32,
) -> Result<(&'static str, Map<String, Value>)> {
    match connector {
        "youtube" => Ok((
            "search",
            json!({ "query": query, "limit": limit })
                .as_object()
                .expect("json object")
                .clone(),
        )),
        "reddit" => Ok((
            "search",
            json!({ "query": query, "limit": limit })
                .as_object()
                .expect("json object")
                .clone(),
        )),
        "hackernews" => Ok((
            "search_stories",
            json!({ "query": query, "hitsPerPage": limit })
                .as_object()
                .expect("json object")
                .clone(),
        )),
        "wikipedia" => Ok((
            "search",
            json!({ "query": query, "limit": limit })
                .as_object()
                .expect("json object")
                .clone(),
        )),
        "arxiv" => Ok((
            "search",
            json!({ "query": query, "limit": limit, "response_format": "concise" })
                .as_object()
                .expect("json object")
                .clone(),
        )),
        "pubmed" => Ok((
            "search",
            json!({ "query": query, "limit": limit, "response_format": "concise" })
                .as_object()
                .expect("json object")
                .clone(),
        )),
        "semantic-scholar" | "semantic_scholar" => Ok((
            "search_papers",
            json!({ "query": query, "limit": limit, "response_format": "concise" })
                .as_object()
                .expect("json object")
                .clone(),
        )),
        "github" => Ok((
            "search_repositories",
            json!({ "query": query, "per_page": limit, "response_format": "concise" })
                .as_object()
                .expect("json object")
                .clone(),
        )),
        _ => Err(CommandError::InvalidInput(format!(
            "Connector '{}' is not supported by the generic `search` command. Use `arivu {0} --help` or `arivu tools {0}`.",
            connector
        ))),
    }
}
