use crate::cli::OutputFormat;
use crate::commands::{CommandError, Result};
use arivu_core::ServerInfo;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum OutputData {
    ConnectorList(Vec<ServerInfo>),
    SearchResults {
        connector: String,
        query: String,
        results: Value,
    },
    ResourceData {
        connector: String,
        id: String,
        data: Value,
    },
    ToolsList {
        connector: Option<String>,
        tools: Value,
    },
    CallResult {
        connector: String,
        tool: String,
        result: Value,
    },
    ConfigInfo(Value),
    ErrorMessage(String),
}

pub fn format_output(data: &OutputData, format: &OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(data)?);
        }
        OutputFormat::Yaml => {
            println!("{}", serde_yaml::to_string(data)?);
        }
        OutputFormat::Text => {
            format_text_output(data)?;
        }
        OutputFormat::Markdown => {
            format_markdown_output(data)?;
        }
        OutputFormat::Pretty => {
            // Pretty formatting is handled in individual commands
            format_text_output(data)?;
        }
    }
    Ok(())
}

fn format_text_output(data: &OutputData) -> Result<()> {
    match data {
        OutputData::ConnectorList(connectors) => {
            for connector in connectors {
                println!("{}: {}", connector.name, connector.description);
            }
        }
        OutputData::SearchResults {
            connector,
            query,
            results,
        } => {
            println!("Search results for '{}' using {}:", query, connector);
            println!("{}", serde_json::to_string_pretty(results)?);
        }
        OutputData::ResourceData {
            connector,
            id,
            data,
        } => {
            println!("Resource '{}' from {}:", id, connector);
            println!("{}", serde_json::to_string_pretty(data)?);
        }
        OutputData::ToolsList { connector, tools } => {
            if let Some(connector) = connector {
                println!("Tools for {}:", connector);
            } else {
                println!("Available tools:");
            }
            println!("{}", serde_json::to_string_pretty(tools)?);
        }
        OutputData::CallResult {
            connector,
            tool,
            result,
        } => {
            println!("Call {}.{}", connector, tool);
            println!("{}", serde_json::to_string_pretty(result)?);
        }
        OutputData::ConfigInfo(config) => {
            println!("Configuration:");
            println!("{}", serde_json::to_string_pretty(config)?);
        }
        OutputData::ErrorMessage(msg) => {
            eprintln!("Error: {}", msg);
        }
    }
    Ok(())
}

fn format_markdown_output(data: &OutputData) -> Result<()> {
    match data {
        OutputData::ConnectorList(connectors) => {
            println!("# Available Connectors\n");
            for connector in connectors {
                println!("## {}\n", connector.name);
                println!("{}\n", connector.description);
            }
        }
        OutputData::SearchResults {
            connector,
            query,
            results,
        } => {
            println!("# Search Results\n");
            println!("**Connector:** {}\n", connector);
            println!("**Query:** {}\n", query);
            println!("```json");
            println!("{}", serde_json::to_string_pretty(results)?);
            println!("```\n");
        }
        OutputData::ResourceData {
            connector,
            id,
            data,
        } => {
            println!("# Resource Data\n");
            println!("**Connector:** {}\n", connector);
            println!("**ID:** {}\n", id);
            println!("```json");
            println!("{}", serde_json::to_string_pretty(data)?);
            println!("```\n");
        }
        OutputData::ToolsList { connector, tools } => {
            if let Some(connector) = connector {
                println!("# Tools for {}\n", connector);
            } else {
                println!("# Available Tools\n");
            }
            println!("```json");
            println!("{}", serde_json::to_string_pretty(tools)?);
            println!("```\n");
        }
        OutputData::CallResult {
            connector,
            tool,
            result,
        } => {
            println!("# Call Result\n");
            println!("**Connector:** {}\n", connector);
            println!("**Tool:** {}\n", tool);
            println!("```json");
            println!("{}", serde_json::to_string_pretty(result)?);
            println!("```\n");
        }
        OutputData::ConfigInfo(config) => {
            println!("# Configuration\n");
            println!("```json");
            println!("{}", serde_json::to_string_pretty(config)?);
            println!("```\n");
        }
        OutputData::ErrorMessage(msg) => {
            println!("# Error\n");
            println!("{}\n", msg);
        }
    }
    Ok(())
}

#[allow(dead_code)]
pub trait FormatError {
    fn format_error(&self) -> String;
}

impl FormatError for CommandError {
    fn format_error(&self) -> String {
        match self {
            CommandError::ConnectorNotFound(name) => {
                format!(
                    "Connector '{}' not found. Use 'arivu list' to see available connectors.",
                    name
                )
            }
            CommandError::ToolNotFound(tool, connector) => {
                format!("Tool '{}' not found for connector '{}'. Use 'arivu tools {}' to see available tools.", tool, connector, connector)
            }
            CommandError::AuthenticationRequired(connector) => {
                format!("Authentication required for connector '{}'. Use 'arivu config set {}' to configure.", connector, connector)
            }
            _ => self.to_string(),
        }
    }
}
