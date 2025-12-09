use arivu_core::connectors::hackernews::{
    AlgoliaSearchResponse, HackerNewsConnector, HackerNewsItem, ItemType, SimpleItem,
};
use arivu_core::Connector;
use async_mcp::types::{CallToolRequest, ToolResponseContent};
use serde_json::{json, Value};
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Initializing Hacker News connector...");

    // Create a new HackerNews connector
    let connector = HackerNewsConnector::new();

    // Test authentication (connectivity)
    connector.test_auth().await?;
    println!("✅ Connection test successful");

    // PART 1: Search for posts by term
    let search_term = "nootropics magnesium";
    println!("\n🔍 Searching for posts about: '{}'", search_term);

    let mut search_args = HashMap::new();
    search_args.insert("query".to_string(), json!(search_term));
    search_args.insert("hitsPerPage".to_string(), json!(25));

    let search_request = CallToolRequest {
        name: "search_stories".to_string(),
        arguments: Some(search_args),
        meta: None,
    };

    let search_response = connector.call_tool(search_request).await?;

    // Parse the search results
    let search_results: AlgoliaSearchResponse = match &search_response.content[0] {
        ToolResponseContent::Text { text } => serde_json::from_str::<AlgoliaSearchResponse>(text)?,
        _ => panic!("Unexpected response type"),
    };

    // Display search results
    if let Some(hits) = &search_results.hits {
        println!("Found {} results", hits.len());

        for (i, hit) in hits.iter().enumerate() {
            if hit.is_comment() {
                // println!("Comment: {}", hit.text.as_ref().unwrap_or(&"".to_string()));
                let comment = match hit.clone().into_simple() {
                    Some(SimpleItem::Comment(comment)) => comment,
                    _ => continue,
                };
                println!("Comment: {:#?}\n\n\n", comment);
                continue;
            }

            // Get the object ID and fetch full details using Algolia items endpoint
            if let Some(object_id) = &hit.object_id {
                let id = object_id.parse::<i64>()?;
                let flatten = true;

                let mut post_args = HashMap::new();
                post_args.insert("id".to_string(), json!(id));
                post_args.insert("flatten".to_string(), json!(flatten));

                let post_request = CallToolRequest {
                    name: "get_post".to_string(),
                    arguments: Some(post_args),
                    meta: None,
                };
                println!("Post request: {:#?}\n\n\n", post_request);

                let post_response = connector.call_tool(post_request).await?;

                if flatten {
                    match &post_response.content[0] {
                        ToolResponseContent::Text { text } => {
                            let comments: Vec<SimpleItem> = serde_json::from_str(text)?;

                            println!("Comments: {:#?}\n\n\n", comments);
                        }
                        _ => panic!("Unexpected response type"),
                    }
                } else {
                    match &post_response.content[0] {
                        ToolResponseContent::Text { text } => {
                            let item: HackerNewsItem = serde_json::from_str(text)?;

                            let simple_item = match item.into_simple() {
                                Some(SimpleItem::Story(story)) => story,
                                _ => continue,
                            };

                            println!("Story: {:#?}\n\n\n", simple_item);
                        }
                        _ => panic!("Unexpected response type"),
                    }
                }
            }
        }
    } else {
        println!("No results found");
    }

    // // PART 2: Search with tags and numeric filters
    // println!("\n🏷️  Searching for 'rust' posts with at least 100 points");

    // let mut filtered_search_args = HashMap::new();
    // filtered_search_args.insert("query".to_string(), json!("rust"));
    // filtered_search_args.insert("tags".to_string(), json!("story"));
    // filtered_search_args.insert("numericFilters".to_string(), json!("points>100"));
    // filtered_search_args.insert("hitsPerPage".to_string(), json!(5));

    // let filtered_search_request = CallToolRequest {
    //     name: "search_stories".to_string(),
    //     arguments: Some(filtered_search_args),
    //     meta: None,
    // };

    // let filtered_search_response = connector.call_tool(filtered_search_request).await?;

    // // Parse the filtered search results
    // let filtered_results = match &filtered_search_response.content[0] {
    //     ToolResponseContent::Text { text } => {
    //         serde_json::from_str::<Value>(text)?
    //     },
    //     _ => panic!("Unexpected response type"),
    // };

    // // Display filtered search results
    // let filtered_hits = filtered_results["hits"].as_array().unwrap();
    // println!("Found {} high-scoring Rust stories", filtered_hits.len());

    // for (i, hit) in filtered_hits.iter().enumerate() {
    //     let title = hit["title"].as_str().unwrap_or("No title");
    //     let author = hit["author"].as_str().unwrap_or("Unknown author");
    //     let points = hit["points"].as_i64().unwrap_or(0);
    //     let comments = hit["num_comments"].as_i64().unwrap_or(0);
    //     let id = hit["objectID"].as_str().unwrap_or("0");

    //     println!("{}. \"{}\" by {} ({} points, {} comments) [ID: {}]",
    //         i+1, title, author, points, comments, id);
    // }

    // // PART 3: Search for Ask HN posts
    // println!("\n❓ Searching for recent Ask HN posts");

    // let mut ask_hn_args = HashMap::new();
    // ask_hn_args.insert("query".to_string(), json!(""));
    // ask_hn_args.insert("tags".to_string(), json!("ask_hn"));
    // ask_hn_args.insert("hitsPerPage".to_string(), json!(5));

    // let ask_hn_request = CallToolRequest {
    //     name: "search_by_date".to_string(),
    //     arguments: Some(ask_hn_args),
    //     meta: None,
    // };

    // let ask_hn_response = connector.call_tool(ask_hn_request).await?;

    // // Parse the Ask HN results
    // let ask_hn_results = match &ask_hn_response.content[0] {
    //     ToolResponseContent::Text { text } => {
    //         serde_json::from_str::<Value>(text)?
    //     },
    //     _ => panic!("Unexpected response type"),
    // };

    // // Display Ask HN results
    // let ask_hn_hits = ask_hn_results["hits"].as_array().unwrap();
    // println!("Found {} recent Ask HN posts", ask_hn_hits.len());

    // for (i, hit) in ask_hn_hits.iter().enumerate() {
    //     let title = hit["title"].as_str().unwrap_or("No title");
    //     let author = hit["author"].as_str().unwrap_or("Unknown author");
    //     let points = hit["points"].as_i64().unwrap_or(0);
    //     let comments = hit["num_comments"].as_i64().unwrap_or(0);
    //     let created_at = hit["created_at"].as_str().unwrap_or("");

    //     println!("{}. \"{}\" by {} ({} points, {} comments) - {}",
    //         i+1, title, author, points, comments, created_at);
    // }

    // // PART 4: Get a specific post with full comment tree
    // if !hits.is_empty() {
    //     let first_hit = &hits[0];
    //     let post_id = first_hit["objectID"].as_str().unwrap_or("0").parse::<i64>()?;

    //     println!("\n📝 Getting full details for post ID: {}", post_id);

    //     let mut post_args = HashMap::new();
    //     post_args.insert("id".to_string(), json!(post_id));

    //     let post_request = CallToolRequest {
    //         name: "get_post".to_string(),
    //         arguments: Some(post_args),
    //         meta: None,
    //     };

    //     let post_response = connector.call_tool(post_request).await?;

    //     // Parse the post details
    //     let post_details = match &post_response.content[0] {
    //         ToolResponseContent::Text { text } => {
    //             serde_json::from_str::<Value>(text)?
    //         },
    //         _ => panic!("Unexpected response type"),
    //     };

    //     // Display post details
    //     println!("\n=== POST DETAILS ===");
    //     println!("Title: {}", post_details["title"].as_str().unwrap_or("No title"));
    //     println!("By: {}", post_details["by"].as_str().unwrap_or("Unknown author"));
    //     println!("Score: {}", post_details["score"].as_i64().unwrap_or(0));
    //     println!("Time: {}", format_time(post_details["time"].as_i64().unwrap_or(0)));

    //     if let Some(url) = post_details["url"].as_str() {
    //         println!("URL: {}", url);
    //     }

    //     if let Some(text) = post_details["text"].as_str() {
    //         println!("\nContent:\n{}", text);
    //     }

    //     // Display comments
    //     if let Some(comments) = post_details["comments"].as_array() {
    //         println!("\n=== COMMENTS ({}) ===", comments.len());
    //         print_comments(comments, 0);
    //     } else {
    //         println!("\nNo comments");
    //     }
    // }

    // // PART 5: Get top 10 trending posts
    // println!("\n🔥 Getting top 10 trending posts");

    // let mut top_args = HashMap::new();
    // top_args.insert("limit".to_string(), json!(10));

    // let top_request = CallToolRequest {
    //     name: "get_top_stories".to_string(),
    //     arguments: Some(top_args),
    //     meta: None,
    // };

    // let top_response = connector.call_tool(top_request).await?;

    // // Parse the top stories
    // let top_stories = match &top_response.content[0] {
    //     ToolResponseContent::Text { text } => {
    //         serde_json::from_str::<Value>(text)?
    //     },
    //     _ => panic!("Unexpected response type"),
    // };

    // // Display top stories
    // println!("\n=== TOP 10 TRENDING POSTS ===");

    // let top_stories_array = top_stories.as_array().unwrap();
    // for (i, story) in top_stories_array.iter().enumerate() {
    //     let title = story["title"].as_str().unwrap_or("No title");
    //     let author = story["by"].as_str().unwrap_or("Unknown author");
    //     let score = story["score"].as_i64().unwrap_or(0);
    //     let id = story["id"].as_i64().unwrap_or(0);

    //     println!("{}. \"{}\" by {} ({} points) [ID: {}]",
    //         i+1, title, author, score, id);
    // }

    Ok(())
}

// Helper function to format Unix timestamp
fn format_time(timestamp: i64) -> String {
    let datetime = chrono::DateTime::from_timestamp(timestamp, 0)
        .unwrap_or_else(|| chrono::DateTime::from_timestamp(0, 0).unwrap());
    datetime.format("%Y-%m-%d %H:%M:%S UTC").to_string()
}

// Helper function to recursively print comments with indentation
fn print_comments(comments: &[Value], depth: usize) {
    let indent = "  ".repeat(depth);

    for (i, comment) in comments.iter().enumerate() {
        if comment["deleted"].as_bool().unwrap_or(false)
            || comment["dead"].as_bool().unwrap_or(false)
        {
            continue;
        }

        let author = comment["by"].as_str().unwrap_or("[deleted]");
        let text = comment["text"].as_str().unwrap_or("[no content]");
        let time = format_time(comment["time"].as_i64().unwrap_or(0));

        println!("\n{}Comment #{} by {} at {}:", indent, i + 1, author, time);
        println!(
            "{}{}",
            indent,
            text.replace("<p>", "\n").replace("</p>", "")
        );

        if let Some(replies) = comment["comments"].as_array() {
            if !replies.is_empty() {
                println!("{}--- {} replies ---", indent, replies.len());
                print_comments(replies, depth + 1);
            }
        }
    }
}
