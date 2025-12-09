use crate::cli::Cli;
use crate::commands::{CommandError, Result};
use arivu_core::{
    auth::AuthDetails,
    auth_store::{AuthStore, FileAuthStore},
    PaginatedRequestParam,
};
use owo_colors::OwoColorize;
use std::collections::HashMap;
use std::io::{self, Write};

/// Connector configuration metadata
struct ConnectorSetupInfo {
    name: &'static str,
    display_name: &'static str,
    description: &'static str,
    auth_type: AuthType,
    env_vars: &'static [(&'static str, &'static str)], // (env_var, description)
    required_fields: &'static [(&'static str, &'static str, bool)], // (field, description, is_secret)
}

enum AuthType {
    None,
    ApiKey,
    #[allow(dead_code)]
    OAuth, // Reserved for future OAuth implementations
    BrowserCookies,
    MultipleFields,
}

const CONNECTORS: &[ConnectorSetupInfo] = &[
    ConnectorSetupInfo {
        name: "youtube",
        display_name: "YouTube",
        description: "Video details, transcripts, and search",
        auth_type: AuthType::None,
        env_vars: &[],
        required_fields: &[],
    },
    ConnectorSetupInfo {
        name: "hackernews",
        display_name: "Hacker News",
        description: "Tech news and discussions",
        auth_type: AuthType::None,
        env_vars: &[],
        required_fields: &[],
    },
    ConnectorSetupInfo {
        name: "arxiv",
        display_name: "ArXiv",
        description: "Academic preprints and papers",
        auth_type: AuthType::None,
        env_vars: &[],
        required_fields: &[],
    },
    ConnectorSetupInfo {
        name: "wikipedia",
        display_name: "Wikipedia",
        description: "Encyclopedia articles",
        auth_type: AuthType::None,
        env_vars: &[],
        required_fields: &[],
    },
    ConnectorSetupInfo {
        name: "pubmed",
        display_name: "PubMed",
        description: "Medical and life science literature",
        auth_type: AuthType::None,
        env_vars: &[],
        required_fields: &[],
    },
    ConnectorSetupInfo {
        name: "slack",
        display_name: "Slack",
        description: "Workspace messages and channels",
        auth_type: AuthType::ApiKey,
        env_vars: &[("SLACK_TOKEN", "Slack Bot Token (xoxb-...)")],
        required_fields: &[("token", "Bot Token (xoxb-...)", true)],
    },
    ConnectorSetupInfo {
        name: "github",
        display_name: "GitHub",
        description: "Repositories, issues, and PRs",
        auth_type: AuthType::ApiKey,
        env_vars: &[("GITHUB_TOKEN", "Personal Access Token")],
        required_fields: &[("token", "Personal Access Token", true)],
    },
    ConnectorSetupInfo {
        name: "reddit",
        display_name: "Reddit",
        description: "Posts, comments, and subreddits",
        auth_type: AuthType::MultipleFields,
        env_vars: &[
            ("REDDIT_CLIENT_ID", "Reddit App Client ID"),
            ("REDDIT_CLIENT_SECRET", "Reddit App Client Secret"),
        ],
        required_fields: &[
            ("client_id", "Client ID", false),
            ("client_secret", "Client Secret", true),
        ],
    },
    ConnectorSetupInfo {
        name: "x",
        display_name: "X (Twitter)",
        description: "Tweets, profiles, and trends",
        auth_type: AuthType::BrowserCookies,
        env_vars: &[],
        required_fields: &[(
            "browser",
            "Browser to extract cookies from (chrome/firefox/safari/brave)",
            false,
        )],
    },
    ConnectorSetupInfo {
        name: "google_search",
        display_name: "Google Search",
        description: "Web search results",
        auth_type: AuthType::MultipleFields,
        env_vars: &[
            ("GOOGLE_API_KEY", "Google API Key"),
            ("GOOGLE_CSE_ID", "Custom Search Engine ID"),
        ],
        required_fields: &[
            ("api_key", "API Key", true),
            ("cse_id", "Custom Search Engine ID", false),
        ],
    },
    ConnectorSetupInfo {
        name: "brave_search",
        display_name: "Brave Search",
        description: "Privacy-focused web search",
        auth_type: AuthType::ApiKey,
        env_vars: &[("BRAVE_API_KEY", "Brave Search API Key")],
        required_fields: &[("api_key", "API Key", true)],
    },
    ConnectorSetupInfo {
        name: "openai-search",
        display_name: "OpenAI Web Search",
        description: "Web search via OpenAI Responses API",
        auth_type: AuthType::ApiKey,
        env_vars: &[("OPENAI_API_KEY", "OpenAI API Key")],
        required_fields: &[("api_key", "API Key", true)],
    },
    ConnectorSetupInfo {
        name: "anthropic-search",
        display_name: "Claude Web Search",
        description: "Web search via Claude",
        auth_type: AuthType::ApiKey,
        env_vars: &[("ANTHROPIC_API_KEY", "Anthropic API Key")],
        required_fields: &[("api_key", "API Key", true)],
    },
    ConnectorSetupInfo {
        name: "perplexity-search",
        display_name: "Perplexity Search",
        description: "AI-powered web search",
        auth_type: AuthType::ApiKey,
        env_vars: &[("PERPLEXITY_API_KEY", "Perplexity API Key")],
        required_fields: &[("api_key", "API Key", true)],
    },
];

pub async fn run(cli: &Cli, connector: Option<&str>) -> Result<()> {
    if let Some(connector_name) = connector {
        // Setup specific connector
        setup_connector(cli, connector_name).await
    } else {
        // Interactive setup wizard
        run_setup_wizard(cli).await
    }
}

async fn run_setup_wizard(_cli: &Cli) -> Result<()> {
    println!();
    println!("{}", "Welcome to RZN DataSourcer Setup".bold().cyan());
    println!("{}", "================================".cyan());
    println!();
    println!("This wizard will help you configure connectors for accessing various data sources.");
    println!();

    // Show available connectors grouped by auth requirement
    println!("{}", "Available Connectors:".bold().green());
    println!();

    // No auth required
    println!(
        "  {} (no authentication required):",
        "Ready to use".green().bold()
    );
    for info in CONNECTORS
        .iter()
        .filter(|c| matches!(c.auth_type, AuthType::None))
    {
        println!(
            "    {} - {}",
            info.display_name.cyan(),
            info.description.dimmed()
        );
    }
    println!();

    // Auth required
    println!(
        "  {} (authentication required):",
        "Needs setup".yellow().bold()
    );
    for info in CONNECTORS
        .iter()
        .filter(|c| !matches!(c.auth_type, AuthType::None))
    {
        let auth_hint = match info.auth_type {
            AuthType::ApiKey => "[API Key]",
            AuthType::OAuth => "[OAuth]",
            AuthType::BrowserCookies => "[Browser Cookies]",
            AuthType::MultipleFields => "[Multiple Fields]",
            AuthType::None => "",
        };
        println!(
            "    {} {} - {}",
            info.display_name.cyan(),
            auth_hint.dimmed(),
            info.description.dimmed()
        );
    }
    println!();

    // Ask which connector to configure
    println!("{}", "Which connector would you like to configure?".bold());
    println!("Enter connector name (e.g., 'slack', 'github') or 'q' to quit:");
    print!("{} ", ">".green().bold());
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let connector_name = input.trim().to_lowercase();

    if connector_name == "q" || connector_name == "quit" || connector_name.is_empty() {
        println!();
        println!("{}", "Setup complete! You can run 'arivu setup <connector>' anytime to configure a connector.".green());
        println!();
        println!("{}", "Quick start:".bold());
        println!("  {} - List all connectors", "arivu list".cyan());
        println!(
            "  {} - Search YouTube videos",
            "arivu search youtube \"rust tutorial\"".cyan()
        );
        println!(
            "  {} - Get video with transcript",
            "arivu get youtube dQw4w9WgXcQ".cyan()
        );
        return Ok(());
    }

    // Find the connector
    let info = CONNECTORS.iter().find(|c| c.name == connector_name);

    if let Some(info) = info {
        configure_connector(info).await?;
    } else {
        println!();
        println!(
            "{} Unknown connector '{}'. Available connectors:",
            "Error:".red().bold(),
            connector_name
        );
        for info in CONNECTORS {
            println!("  - {}", info.name);
        }
    }

    Ok(())
}

async fn setup_connector(_cli: &Cli, connector_name: &str) -> Result<()> {
    let info = CONNECTORS.iter().find(|c| c.name == connector_name);

    if let Some(info) = info {
        println!();
        println!(
            "{} {}",
            "Setting up".bold().cyan(),
            info.display_name.yellow()
        );
        println!("{}", info.description.dimmed());
        println!();

        configure_connector(info).await?;
    } else {
        // Try to show tools for the connector even if not in our predefined list
        let registry = crate::commands::list::create_registry().await?;

        if let Some(provider) = registry.get_provider(connector_name) {
            let c = provider.lock().await;
            let tools_response = c
                .list_tools(Some(PaginatedRequestParam { cursor: None }))
                .await?;

            println!();
            println!("{} {}", "Connector:".bold().cyan(), connector_name.yellow());
            println!();

            if tools_response.tools.is_empty() {
                println!("{}", "No tools available for this connector.".yellow());
            } else {
                println!("{}", "Available tools:".bold().green());
                for tool in &tools_response.tools {
                    println!(
                        "  {} - {}",
                        tool.name.cyan().bold(),
                        tool.description
                            .as_deref()
                            .unwrap_or("No description")
                            .dimmed()
                    );
                }
            }

            // Show config schema if available
            let schema = c.config_schema();
            if !schema.fields.is_empty() {
                println!();
                println!("{}", "Required configuration:".bold().yellow());
                for field in &schema.fields {
                    let req = if field.required {
                        "(required)"
                    } else {
                        "(optional)"
                    };
                    println!(
                        "  {} {} - {}",
                        field.name.cyan(),
                        req.dimmed(),
                        field.description.as_deref().unwrap_or("").dimmed()
                    );
                }
                println!();
                println!("To configure, run:");
                println!(
                    "  {}",
                    format!("arivu config set {} --value <your-value>", connector_name).cyan()
                );
            } else {
                println!();
                println!("{}", "This connector requires no authentication.".green());
            }

            println!();
            println!("{}", "Example usage:".bold().green());
            println!("  {}", format!("arivu tools {}", connector_name).cyan());
            println!(
                "  {}",
                format!("arivu search {} \"query\"", connector_name).cyan()
            );
        } else {
            return Err(CommandError::ConnectorNotFound(connector_name.to_string()));
        }
    }

    Ok(())
}

async fn configure_connector(info: &ConnectorSetupInfo) -> Result<()> {
    match info.auth_type {
        AuthType::None => {
            println!(
                "{} {} requires no authentication!",
                "Great!".green().bold(),
                info.display_name
            );
            println!();
            println!("{}", "You can start using it right away:".bold());
            println!(
                "  {}",
                format!("arivu search {} \"your query\"", info.name).cyan()
            );
            println!("  {}", format!("arivu tools {}", info.name).cyan());
        }
        AuthType::ApiKey | AuthType::MultipleFields => {
            println!("{}", "Configuration options:".bold());
            println!();

            // Show environment variable option
            if !info.env_vars.is_empty() {
                println!(
                    "  {} Set environment variables:",
                    "Option 1:".yellow().bold()
                );
                for (env_var, desc) in info.env_vars {
                    println!("    export {}=\"<{}>\"", env_var, desc);
                }
                println!();
            }

            // Show config file option
            println!(
                "  {} Enter credentials now (stored in ~/.config/arivu/auth.json):",
                "Option 2:".yellow().bold()
            );
            println!();

            print!("Enter credentials now? [y/N] ");
            io::stdout().flush()?;

            let mut input = String::new();
            io::stdin().read_line(&mut input)?;

            if input.trim().to_lowercase() == "y" {
                let mut auth: AuthDetails = HashMap::new();

                for (field, desc, is_secret) in info.required_fields {
                    print!("  {} ", format!("{}:", desc).bold());
                    io::stdout().flush()?;

                    let value = if *is_secret {
                        read_password()?
                    } else {
                        let mut v = String::new();
                        io::stdin().read_line(&mut v)?;
                        v.trim().to_string()
                    };

                    if !value.is_empty() {
                        auth.insert(field.to_string(), value);
                    }
                }

                if !auth.is_empty() {
                    let store = FileAuthStore::new_default();
                    store.save(info.name, &auth).map_err(|e| {
                        CommandError::InvalidConfig(format!("Failed to save credentials: {}", e))
                    })?;

                    println!();
                    println!(
                        "{} Credentials saved for {}",
                        "Success!".green().bold(),
                        info.display_name
                    );
                    println!();
                    println!(
                        "Test with: {}",
                        format!("arivu config test {}", info.name).cyan()
                    );
                }
            } else {
                println!();
                println!("You can configure later with:");
                println!("  {}", format!("arivu setup {}", info.name).cyan());
                println!(
                    "  {}",
                    format!("arivu config set {} --value <value>", info.name).cyan()
                );
            }
        }
        AuthType::BrowserCookies => {
            println!(
                "{} extracts authentication from your browser cookies.",
                info.display_name
            );
            println!();
            println!("{}", "Supported browsers:".bold());
            println!("  - Chrome");
            println!("  - Firefox");
            println!("  - Safari");
            println!("  - Brave");
            println!();
            println!("{}", "Requirements:".yellow().bold());
            println!(
                "  1. You must be logged in to {} in your browser",
                info.display_name
            );
            println!("  2. Close the browser before extracting cookies");
            println!();

            print!("Which browser do you use? [chrome/firefox/safari/brave]: ");
            io::stdout().flush()?;

            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            let browser = input.trim().to_lowercase();

            if ["chrome", "firefox", "safari", "brave"].contains(&browser.as_str()) {
                let mut auth: AuthDetails = HashMap::new();
                auth.insert("browser".to_string(), browser.clone());

                let store = FileAuthStore::new_default();
                store.save(info.name, &auth).map_err(|e| {
                    CommandError::InvalidConfig(format!("Failed to save config: {}", e))
                })?;

                println!();
                println!("{} Browser set to {}", "Success!".green().bold(), browser);
                println!();
                println!("Cookies will be extracted from {} when needed.", browser);
                println!(
                    "Test with: {}",
                    format!("arivu config test {}", info.name).cyan()
                );
            } else {
                println!();
                println!(
                    "{} Invalid browser. Supported: chrome, firefox, safari, brave",
                    "Error:".red().bold()
                );
            }
        }
        AuthType::OAuth => {
            println!(
                "{} OAuth setup is not yet fully implemented.",
                "Note:".yellow().bold()
            );
            println!();
            println!("For now, you can:");
            println!("  1. Obtain an access token manually");
            println!(
                "  2. Set it via: {}",
                format!("arivu config set {} --value <token>", info.name).cyan()
            );
        }
    }

    Ok(())
}

fn read_password() -> Result<String> {
    // Simple password input (not hidden - would need rpassword crate for that)
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(input.trim().to_string())
}
