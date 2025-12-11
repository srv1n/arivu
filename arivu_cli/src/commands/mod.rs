pub mod call;
pub mod config;
pub mod connectors;
pub mod fetch;
pub mod get;
pub mod list;
pub mod search;
pub mod setup;
pub mod tools;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum CommandError {
    #[error("Connector '{0}' not found")]
    ConnectorNotFound(String),

    #[error("Tool '{0}' not found for connector '{1}'")]
    ToolNotFound(String, String),

    #[error("Authentication required for connector '{0}'")]
    #[allow(dead_code)]
    AuthenticationRequired(String),

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("Tool error: {0}")]
    ToolError(String),

    #[error("Core library error: {0}")]
    Core(#[from] arivu_core::error::ConnectorError),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("YAML serialization error: {0}")]
    Yaml(#[from] serde_yaml::Error),
}

pub type Result<T> = std::result::Result<T, CommandError>;
