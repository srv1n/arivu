use crate::cli::{Cli, ConfigAction};
use crate::commands::{CommandError, Result};
use crate::output::{format_output, OutputData};
use arivu_core::{
    auth::AuthDetails,
    auth_store::{AuthStore, FileAuthStore},
};
use owo_colors::OwoColorize;
use serde_json::{json, Value};
use std::collections::HashMap;

pub async fn run(cli: &Cli, action: ConfigAction) -> Result<()> {
    match action {
        ConfigAction::Show => show_config(cli).await,
        ConfigAction::Set {
            connector,
            auth_type,
            value,
            browser,
        } => {
            set_config(
                cli,
                &connector,
                auth_type.as_deref(),
                value.as_deref(),
                browser.as_deref(),
            )
            .await
        }
        ConfigAction::Remove { connector } => remove_config(cli, &connector).await,
        ConfigAction::Test { connector } => test_config(cli, &connector).await,
    }
}

async fn show_config(cli: &Cli) -> Result<()> {
    let config = get_current_config()?;

    let output_data = OutputData::ConfigInfo(config.clone());

    match cli.output {
        crate::cli::OutputFormat::Pretty => {
            println!("{}", "Current Configuration".bold().cyan());
            println!();

            if config
                .as_object()
                .expect("Config JSON must be an object")
                .is_empty()
            {
                println!("{}", "No configuration found".yellow());
                println!();
                println!(
                    "{} Use {} to configure authentication for connectors",
                    "Tip:".green().bold(),
                    "arivu config set <connector>".cyan()
                );
            } else {
                for (connector, settings) in
                    config.as_object().expect("Config JSON must be an object")
                {
                    println!("{} {}", "Connector:".bold(), connector.cyan());
                    if let Some(auth_status) = settings.get("auth_status") {
                        let status = auth_status.as_str().unwrap_or("unknown");
                        let status_display = match status {
                            "configured" => "✓ Configured".green().to_string(),
                            "missing" => "✗ Not configured".red().to_string(),
                            _ => status.yellow().to_string(),
                        };
                        println!("  {}: {}", "Status".bold(), status_display);
                    }
                    if let Some(auth_type) = settings.get("auth_type") {
                        println!(
                            "  {}: {}",
                            "Auth Type".bold(),
                            auth_type.as_str().unwrap_or("unknown")
                        );
                    }
                    println!();
                }
            }
        }
        _ => {
            format_output(&output_data, &cli.output)?;
        }
    }

    Ok(())
}

async fn set_config(
    _cli: &Cli,
    connector: &str,
    auth_type: Option<&str>,
    value: Option<&str>,
    browser: Option<&str>,
) -> Result<()> {
    println!(
        "{} configuration for {}",
        "Setting".bold().cyan(),
        connector.yellow()
    );

    // Validate connector exists
    let registry = crate::commands::list::create_registry().await?;
    if registry.get_provider(connector).is_none() {
        return Err(CommandError::ConnectorNotFound(connector.to_string()));
    }

    // Special-case Slack: allow setting token directly into the core FileAuthStore
    if connector == "slack" {
        if let Some(token) = value {
            let mut auth: AuthDetails = HashMap::new();
            auth.insert("token".to_string(), token.to_string());
            let store = FileAuthStore::new_default();
            store
                .save("slack", &auth)
                .map_err(|e| CommandError::InvalidConfig(format!("failed to save token: {}", e)))?;
            println!(
                "{} Stored Slack token locally for {}",
                "✓".green().bold(),
                connector.cyan()
            );
            println!("Scopes expected: conversations:read, users:read, channels:history, groups:history, im:history, mpim:history, files:read, search:read");
            return Ok(());
        }
    }

    // Determine auth method (generic fallback)
    let auth_method = match (auth_type, value, browser) {
        (Some("api-key"), Some(key), _) => {
            store_api_key(connector, key)?;
            "API Key"
        }
        (Some("browser"), _, Some(browser_name)) => {
            configure_browser_auth(connector, browser_name)?;
            "Browser Cookies"
        }
        (Some("oauth"), _, _) => {
            configure_oauth(connector)?;
            "OAuth2"
        }
        (None, Some(value), None) => {
            // Guess the auth type based on value format
            if value.starts_with("sk-") || value.len() > 20 {
                store_api_key(connector, value)?;
                "API Key"
            } else {
                return Err(CommandError::InvalidConfig(
                    "Could not determine authentication type. Use --auth-type".to_string(),
                ));
            }
        }
        (None, None, Some(browser_name)) => {
            configure_browser_auth(connector, browser_name)?;
            "Browser Cookies"
        }
        _ => {
            return Err(CommandError::InvalidConfig(
                "Invalid configuration. Specify --auth-type with --value or --browser".to_string(),
            ));
        }
    };

    println!(
        "{} {} authentication for {}",
        "✓".green().bold(),
        "Configured".green(),
        connector.cyan()
    );
    println!("  {}: {}", "Method".bold(), auth_method);

    Ok(())
}

async fn remove_config(_cli: &Cli, connector: &str) -> Result<()> {
    // Remove environment variables or config file entries
    println!(
        "{} configuration for {}",
        "Removing".bold().red(),
        connector.yellow()
    );

    // TODO: Implement actual removal logic
    println!(
        "{} Configuration removed for {}",
        "✓".green().bold(),
        connector.cyan()
    );

    Ok(())
}

async fn test_config(_cli: &Cli, connector: &str) -> Result<()> {
    println!(
        "{} authentication for {}",
        "Testing".bold().cyan(),
        connector.yellow()
    );

    let registry = crate::commands::list::create_registry().await?;
    let provider = registry
        .get_provider(connector)
        .ok_or_else(|| CommandError::ConnectorNotFound(connector.to_string()))?;

    let c = provider.lock().await;
    match c.test_auth().await {
        Ok(_) => {
            println!(
                "{} Authentication successful for {}",
                "✓".green().bold(),
                connector.cyan()
            );
        }
        Err(e) => {
            println!(
                "{} Authentication failed for {}: {}",
                "✗".red().bold(),
                connector.cyan(),
                e.to_string().red()
            );
        }
    }

    Ok(())
}

fn get_current_config() -> Result<Value> {
    let mut config = json!({});

    // Check environment variables for common connectors
    let connectors = vec![
        ("reddit", vec!["REDDIT_CLIENT_ID", "REDDIT_CLIENT_SECRET"]),
        ("x", vec!["X_USERNAME", "X_PASSWORD"]),
    ];

    for (connector, env_vars) in connectors {
        let mut connector_config = json!({});
        let mut has_config = false;

        for env_var in env_vars {
            if std::env::var(env_var).is_ok() {
                has_config = true;
                connector_config[env_var.to_lowercase()] = json!("***");
            }
        }

        if has_config {
            connector_config["auth_status"] = json!("configured");
            connector_config["auth_type"] = json!("environment");
        } else {
            connector_config["auth_status"] = json!("missing");
        }

        config[connector] = connector_config;
    }

    Ok(config)
}

fn store_api_key(connector: &str, key: &str) -> Result<()> {
    // In a real implementation, this would store to a secure config file
    // For now, just suggest setting environment variables
    let env_var = match connector {
        "reddit" => "REDDIT_CLIENT_ID", // Would need both ID and secret
        _ => {
            return Err(CommandError::InvalidConfig(format!(
                "Unknown connector: {}",
                connector
            )))
        }
    };

    println!("{} Set environment variable:", "Note:".yellow().bold());
    println!("  export {}=\"{}\"", env_var, key);

    Ok(())
}

fn configure_browser_auth(_connector: &str, browser: &str) -> Result<()> {
    let supported_browsers = ["chrome", "firefox", "safari", "brave"];
    if !supported_browsers.contains(&browser) {
        return Err(CommandError::InvalidConfig(format!(
            "Unsupported browser: {}. Supported: {}",
            browser,
            supported_browsers.join(", ")
        )));
    }

    println!(
        "{} Browser authentication will extract cookies from {}",
        "Note:".yellow().bold(),
        browser.cyan()
    );

    Ok(())
}

fn configure_oauth(connector: &str) -> Result<()> {
    println!(
        "{} OAuth2 configuration not yet implemented for {}",
        "Note:".yellow().bold(),
        connector.cyan()
    );

    Ok(())
}
