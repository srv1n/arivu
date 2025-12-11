use crate::cli::Cli;
use crate::commands::{CommandError, Result};
use arivu_core::{
    auth::AuthDetails,
    auth_store::{AuthStore, FileAuthStore},
    oauth::{google_device_authorize, google_device_poll, ms_device_authorize, ms_device_poll},
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
    required_fields: &'static [FieldInfo],
    instructions: Option<SetupInstructions>,
}

struct FieldInfo {
    name: &'static str,
    label: &'static str,
    is_secret: bool,
    hint: Option<&'static str>, // e.g., "starts with xoxb-"
}

struct SetupInstructions {
    obtain_url: &'static str,
    steps: &'static [&'static str],
}

#[derive(Clone, Copy)]
enum AuthType {
    None,
    ApiKey,
    OAuth { provider: OAuthProvider },
    BrowserCookies,
    MultipleFields,
}

#[derive(Clone, Copy)]
enum OAuthProvider {
    Google { scopes: &'static str },
    Microsoft { scopes: &'static str },
}

const CONNECTORS: &[ConnectorSetupInfo] = &[
    // === No Auth Required ===
    ConnectorSetupInfo {
        name: "youtube",
        display_name: "YouTube",
        description: "Video details, transcripts, and search",
        auth_type: AuthType::None,
        env_vars: &[],
        required_fields: &[],
        instructions: None,
    },
    ConnectorSetupInfo {
        name: "hackernews",
        display_name: "Hacker News",
        description: "Tech news and discussions",
        auth_type: AuthType::None,
        env_vars: &[],
        required_fields: &[],
        instructions: None,
    },
    ConnectorSetupInfo {
        name: "arxiv",
        display_name: "ArXiv",
        description: "Academic preprints and papers",
        auth_type: AuthType::None,
        env_vars: &[],
        required_fields: &[],
        instructions: None,
    },
    ConnectorSetupInfo {
        name: "wikipedia",
        display_name: "Wikipedia",
        description: "Encyclopedia articles",
        auth_type: AuthType::None,
        env_vars: &[],
        required_fields: &[],
        instructions: None,
    },
    ConnectorSetupInfo {
        name: "pubmed",
        display_name: "PubMed",
        description: "Medical and life science literature",
        auth_type: AuthType::None,
        env_vars: &[],
        required_fields: &[],
        instructions: None,
    },
    ConnectorSetupInfo {
        name: "biorxiv",
        display_name: "bioRxiv/medRxiv",
        description: "Biology and Health Sciences Preprints",
        auth_type: AuthType::None,
        env_vars: &[],
        required_fields: &[],
        instructions: None,
    },
    ConnectorSetupInfo {
        name: "rss",
        display_name: "RSS",
        description: "RSS/Atom feed reader",
        auth_type: AuthType::None,
        env_vars: &[],
        required_fields: &[],
        instructions: None,
    },
    ConnectorSetupInfo {
        name: "google-scholar",
        display_name: "Google Scholar",
        description: "Academic papers via Google Scholar (scraping)",
        auth_type: AuthType::None,
        env_vars: &[],
        required_fields: &[],
        instructions: None,
    },
    // === API Key Auth ===
    ConnectorSetupInfo {
        name: "discord",
        display_name: "Discord",
        description: "Discord server messages and channels",
        auth_type: AuthType::ApiKey,
        env_vars: &[("DISCORD_TOKEN", "Bot Token")],
        required_fields: &[FieldInfo {
            name: "token",
            label: "Bot Token",
            is_secret: true,
            hint: None,
        }],
        instructions: Some(SetupInstructions {
            obtain_url: "https://discord.com/developers/applications",
            steps: &[
                "Create a New Application",
                "Go to the 'Bot' tab",
                "Click 'Reset Token' to get your token",
                "Ensure 'Message Content Intent' is enabled",
            ],
        }),
    },
    ConnectorSetupInfo {
        name: "slack",
        display_name: "Slack",
        description: "Workspace messages and channels",
        auth_type: AuthType::ApiKey,
        env_vars: &[("SLACK_BOT_TOKEN", "Bot Token")],
        required_fields: &[FieldInfo {
            name: "token",
            label: "Bot Token",
            is_secret: true,
            hint: Some("starts with xoxb-"),
        }],
        instructions: Some(SetupInstructions {
            obtain_url: "https://api.slack.com/apps",
            steps: &[
                "Create a new app or select an existing one",
                "Go to 'OAuth & Permissions' in the sidebar",
                "Add required scopes: channels:read, channels:history, users:read",
                "Install the app to your workspace",
                "Copy the 'Bot User OAuth Token' (starts with xoxb-)",
            ],
        }),
    },
    ConnectorSetupInfo {
        name: "github",
        display_name: "GitHub",
        description: "Repositories, issues, and PRs",
        auth_type: AuthType::ApiKey,
        env_vars: &[("GITHUB_TOKEN", "Personal Access Token")],
        required_fields: &[FieldInfo {
            name: "token",
            label: "Personal Access Token",
            is_secret: true,
            hint: Some("starts with ghp_ or github_pat_"),
        }],
        instructions: Some(SetupInstructions {
            obtain_url: "https://github.com/settings/tokens",
            steps: &[
                "Click 'Generate new token' (classic or fine-grained)",
                "Select scopes: repo, read:org (for private repos)",
                "Generate and copy the token",
            ],
        }),
    },
    ConnectorSetupInfo {
        name: "brave_search",
        display_name: "Brave Search",
        description: "Privacy-focused web search",
        auth_type: AuthType::ApiKey,
        env_vars: &[("BRAVE_API_KEY", "API Key")],
        required_fields: &[FieldInfo {
            name: "api_key",
            label: "API Key",
            is_secret: true,
            hint: None,
        }],
        instructions: Some(SetupInstructions {
            obtain_url: "https://brave.com/search/api/",
            steps: &[
                "Sign up for a Brave Search API account",
                "Navigate to the API dashboard",
                "Create a new API key",
            ],
        }),
    },
    ConnectorSetupInfo {
        name: "openai-search",
        display_name: "OpenAI Web Search",
        description: "Web search via OpenAI",
        auth_type: AuthType::ApiKey,
        env_vars: &[("OPENAI_API_KEY", "API Key")],
        required_fields: &[FieldInfo {
            name: "api_key",
            label: "API Key",
            is_secret: true,
            hint: Some("starts with sk-"),
        }],
        instructions: Some(SetupInstructions {
            obtain_url: "https://platform.openai.com/api-keys",
            steps: &[
                "Log in to your OpenAI account",
                "Navigate to API Keys section",
                "Create a new secret key",
            ],
        }),
    },
    ConnectorSetupInfo {
        name: "anthropic-search",
        display_name: "Claude Web Search",
        description: "Web search via Claude",
        auth_type: AuthType::ApiKey,
        env_vars: &[("ANTHROPIC_API_KEY", "API Key")],
        required_fields: &[FieldInfo {
            name: "api_key",
            label: "API Key",
            is_secret: true,
            hint: Some("starts with sk-ant-"),
        }],
        instructions: Some(SetupInstructions {
            obtain_url: "https://console.anthropic.com/settings/keys",
            steps: &[
                "Log in to your Anthropic Console",
                "Navigate to API Keys",
                "Create a new key",
            ],
        }),
    },
    ConnectorSetupInfo {
        name: "perplexity-search",
        display_name: "Perplexity Search",
        description: "AI-powered web search",
        auth_type: AuthType::ApiKey,
        env_vars: &[("PPLX_API_KEY", "API Key")],
        required_fields: &[FieldInfo {
            name: "api_key",
            label: "API Key",
            is_secret: true,
            hint: Some("starts with pplx-"),
        }],
        instructions: Some(SetupInstructions {
            obtain_url: "https://www.perplexity.ai/settings/api",
            steps: &[
                "Log in to Perplexity",
                "Go to Settings > API",
                "Generate a new API key",
            ],
        }),
    },
    ConnectorSetupInfo {
        name: "exa-search",
        display_name: "Exa Search",
        description: "Neural web search",
        auth_type: AuthType::ApiKey,
        env_vars: &[("EXA_API_KEY", "API Key")],
        required_fields: &[FieldInfo {
            name: "api_key",
            label: "API Key",
            is_secret: true,
            hint: None,
        }],
        instructions: Some(SetupInstructions {
            obtain_url: "https://dashboard.exa.ai/api-keys",
            steps: &[
                "Sign up at exa.ai",
                "Navigate to the API Keys dashboard",
                "Create a new key",
            ],
        }),
    },
    ConnectorSetupInfo {
        name: "tavily-search",
        display_name: "Tavily Search",
        description: "AI search optimized for LLMs",
        auth_type: AuthType::ApiKey,
        env_vars: &[("TAVILY_API_KEY", "API Key")],
        required_fields: &[FieldInfo {
            name: "api_key",
            label: "API Key",
            is_secret: true,
            hint: Some("starts with tvly-"),
        }],
        instructions: Some(SetupInstructions {
            obtain_url: "https://tavily.com/#api",
            steps: &[
                "Sign up at tavily.com",
                "Go to your dashboard",
                "Copy your API key",
            ],
        }),
    },
    // === Multiple Fields ===
    ConnectorSetupInfo {
        name: "reddit",
        display_name: "Reddit",
        description: "Posts, comments, and subreddits (works without auth for public content)",
        auth_type: AuthType::MultipleFields,
        env_vars: &[
            ("REDDIT_CLIENT_ID", "Client ID"),
            ("REDDIT_CLIENT_SECRET", "Client Secret"),
        ],
        required_fields: &[
            FieldInfo {
                name: "client_id",
                label: "Client ID",
                is_secret: false,
                hint: Some("found under your app name"),
            },
            FieldInfo {
                name: "client_secret",
                label: "Client Secret",
                is_secret: true,
                hint: None,
            },
        ],
        instructions: Some(SetupInstructions {
            obtain_url: "https://www.reddit.com/prefs/apps",
            steps: &[
                "Scroll to 'Developed Applications' and click 'create app'",
                "Select 'script' as the app type",
                "Set redirect URI to http://localhost:8080",
                "Note the client ID (under app name) and secret",
            ],
        }),
    },
    ConnectorSetupInfo {
        name: "google_search",
        display_name: "Google Custom Search",
        description: "Web search via Google CSE",
        auth_type: AuthType::MultipleFields,
        env_vars: &[
            ("GOOGLE_API_KEY", "API Key"),
            ("GOOGLE_CSE_ID", "Custom Search Engine ID"),
        ],
        required_fields: &[
            FieldInfo {
                name: "api_key",
                label: "API Key",
                is_secret: true,
                hint: None,
            },
            FieldInfo {
                name: "cse_id",
                label: "Search Engine ID",
                is_secret: false,
                hint: Some("looks like: 017576662512468239146:omuauf_lfve"),
            },
        ],
        instructions: Some(SetupInstructions {
            obtain_url: "https://programmablesearchengine.google.com/",
            steps: &[
                "Create a Custom Search Engine at the URL above",
                "Get your Search Engine ID from the control panel",
                "Enable the Custom Search API in Google Cloud Console",
                "Create an API key in Google Cloud Console > Credentials",
            ],
        }),
    },
    // === Browser Cookies ===
    ConnectorSetupInfo {
        name: "x",
        display_name: "X (Twitter)",
        description: "Tweets, profiles, and trends",
        auth_type: AuthType::BrowserCookies,
        env_vars: &[],
        required_fields: &[FieldInfo {
            name: "browser",
            label: "Browser",
            is_secret: false,
            hint: Some("chrome, firefox, safari, or brave"),
        }],
        instructions: Some(SetupInstructions {
            obtain_url: "https://x.com",
            steps: &[
                "Log in to X (Twitter) in your browser",
                "Make sure you're logged in and can see your timeline",
                "Close the browser completely before running setup",
                "Arivu will extract your session cookies automatically",
            ],
        }),
    },
    // === OAuth ===
    ConnectorSetupInfo {
        name: "google-drive",
        display_name: "Google Drive",
        description: "Files and folders",
        auth_type: AuthType::OAuth {
            provider: OAuthProvider::Google {
                scopes: "https://www.googleapis.com/auth/drive.readonly",
            },
        },
        env_vars: &[],
        required_fields: &[],
        instructions: Some(SetupInstructions {
            obtain_url: "https://console.cloud.google.com/apis/credentials",
            steps: &[
                "Create OAuth 2.0 credentials in Google Cloud Console",
                "Or use the device authorization flow below (recommended)",
            ],
        }),
    },
    ConnectorSetupInfo {
        name: "google-gmail",
        display_name: "Gmail",
        description: "Email access",
        auth_type: AuthType::OAuth {
            provider: OAuthProvider::Google {
                scopes: "https://www.googleapis.com/auth/gmail.readonly",
            },
        },
        env_vars: &[],
        required_fields: &[],
        instructions: None,
    },
    ConnectorSetupInfo {
        name: "google-calendar",
        display_name: "Google Calendar",
        description: "Calendar events",
        auth_type: AuthType::OAuth {
            provider: OAuthProvider::Google {
                scopes: "https://www.googleapis.com/auth/calendar.readonly",
            },
        },
        env_vars: &[],
        required_fields: &[],
        instructions: None,
    },
    ConnectorSetupInfo {
        name: "microsoft-graph",
        display_name: "Microsoft Graph",
        description: "OneDrive, Outlook, Calendar",
        auth_type: AuthType::OAuth {
            provider: OAuthProvider::Microsoft {
                scopes: "Files.Read Mail.Read Calendars.Read User.Read offline_access",
            },
        },
        env_vars: &[],
        required_fields: &[],
        instructions: Some(SetupInstructions {
            obtain_url:
                "https://portal.azure.com/#blade/Microsoft_AAD_RegisteredApps/ApplicationsListBlade",
            steps: &[
                "Register an app in Azure AD",
                "Or use the device authorization flow below (recommended)",
            ],
        }),
    },
];

pub async fn run(cli: &Cli, connector: Option<&str>) -> Result<()> {
    if let Some(connector_name) = connector {
        setup_connector(cli, connector_name).await
    } else {
        run_setup_wizard(cli).await
    }
}

async fn run_setup_wizard(_cli: &Cli) -> Result<()> {
    println!();
    println!("{}", "Arivu Setup".bold().cyan());
    println!("{}", "===========".cyan());
    println!();
    println!("Configure connectors for accessing external data sources.");
    println!();

    // Show available connectors grouped by auth requirement
    println!("{}", "Available Connectors:".bold());
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
            AuthType::OAuth { .. } => "[OAuth]",
            AuthType::BrowserCookies => "[Browser Cookies]",
            AuthType::MultipleFields => "[Credentials]",
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
        println!(
            "{}",
            "Run 'arivu setup <connector>' anytime to configure a connector.".green()
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
            info.display_name.bold()
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
                println!("  {}", format!("arivu setup {}", connector_name).cyan());
            } else {
                println!();
                println!("{}", "This connector requires no authentication.".green());
            }
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
                "{} {} requires no authentication.",
                "Ready!".green().bold(),
                info.display_name
            );
            println!();
            println!("{}", "Try it now:".bold());
            println!(
                "  {}",
                format!("arivu search {} \"your query\"", info.name).cyan()
            );
        }
        AuthType::ApiKey | AuthType::MultipleFields => {
            // Show instructions if available
            if let Some(instructions) = &info.instructions {
                println!("{}", "How to get credentials:".bold());
                println!("  {}", instructions.obtain_url.cyan().underline());
                println!();
                for (i, step) in instructions.steps.iter().enumerate() {
                    println!("  {}. {}", i + 1, step);
                }
                println!();
            }

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

            // Show interactive option
            let store = FileAuthStore::new_default();
            let config_path = store.config_path();
            println!(
                "  {} Enter credentials now (stored in {}):",
                "Option 2:".yellow().bold(),
                config_path.dimmed()
            );
            println!();

            print!("Enter credentials now? [y/N] ");
            io::stdout().flush()?;

            let mut input = String::new();
            io::stdin().read_line(&mut input)?;

            if input.trim().to_lowercase() == "y" {
                let mut auth: AuthDetails = HashMap::new();

                for field in info.required_fields {
                    let hint = field.hint.map(|h| format!(" ({})", h)).unwrap_or_default();
                    print!("  {}{}: ", field.label.bold(), hint.dimmed());
                    io::stdout().flush()?;

                    let value = if field.is_secret {
                        read_secret()?
                    } else {
                        let mut v = String::new();
                        io::stdin().read_line(&mut v)?;
                        v.trim().to_string()
                    };

                    if !value.is_empty() {
                        auth.insert(field.name.to_string(), value);
                    }
                }

                if !auth.is_empty() {
                    // Save credentials
                    store.save(info.name, &auth).map_err(|e| {
                        CommandError::InvalidConfig(format!("Failed to save credentials: {}", e))
                    })?;

                    println!();
                    println!(
                        "{} Credentials saved for {}",
                        "Saved!".green().bold(),
                        info.display_name
                    );

                    // Test the connection
                    println!();
                    print!("{}", "Testing connection... ".dimmed());
                    io::stdout().flush()?;

                    match test_connector_auth(info.name).await {
                        Ok(_) => {
                            println!("{}", "Success!".green().bold());
                            println!();
                            println!("{}", "You're all set! Try:".bold());
                            println!(
                                "  {}",
                                format!("arivu search {} \"test query\"", info.name).cyan()
                            );
                        }
                        Err(e) => {
                            println!("{}", "Failed".red().bold());
                            println!();
                            println!("{} {}", "Error:".red().bold(), e.to_string().red());
                            println!();
                            println!("Your credentials were saved. You can:");
                            println!("  - Check if the credentials are correct");
                            println!(
                                "  - Re-run {} to try again",
                                format!("arivu setup {}", info.name).cyan()
                            );
                            println!(
                                "  - Test manually with {}",
                                format!("arivu config test {}", info.name).cyan()
                            );
                        }
                    }
                }
            } else {
                show_later_instructions(info);
            }
        }
        AuthType::BrowserCookies => {
            // Show instructions
            if let Some(instructions) = &info.instructions {
                println!("{}", "Prerequisites:".bold());
                for (i, step) in instructions.steps.iter().enumerate() {
                    println!("  {}. {}", i + 1, step);
                }
                println!();
            }

            println!("{}", "Supported browsers:".bold());
            println!("  - Chrome");
            println!("  - Firefox");
            println!("  - Safari (macOS only)");
            println!("  - Brave");
            println!();

            print!("Which browser are you logged into? [chrome/firefox/safari/brave]: ");
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
                println!(
                    "{} Browser set to {}",
                    "Saved!".green().bold(),
                    browser.cyan()
                );

                // Test the connection
                println!();
                print!("{}", "Extracting cookies and testing... ".dimmed());
                io::stdout().flush()?;

                match test_connector_auth(info.name).await {
                    Ok(_) => {
                        println!("{}", "Success!".green().bold());
                        println!();
                        println!("{}", "You're all set! Try:".bold());
                        println!(
                            "  {}",
                            format!("arivu {} search_tweets \"rust\"", info.name).cyan()
                        );
                    }
                    Err(e) => {
                        println!("{}", "Failed".red().bold());
                        println!();
                        println!("{} {}", "Error:".red().bold(), e.to_string().red());
                        println!();
                        println!("Make sure you:");
                        println!("  - Are logged into {} in {}", info.display_name, browser);
                        println!("  - Have closed the browser completely");
                        println!("  - Have granted Arivu permission to access cookies (macOS)");
                    }
                }
            } else if !browser.is_empty() {
                println!();
                println!(
                    "{} '{}' is not supported. Use: chrome, firefox, safari, or brave",
                    "Error:".red().bold(),
                    browser
                );
            }
        }
        AuthType::OAuth { provider } => {
            configure_oauth(info, provider).await?;
        }
    }

    Ok(())
}

async fn configure_oauth(info: &ConnectorSetupInfo, provider: OAuthProvider) -> Result<()> {
    println!("{}", "OAuth Authorization".bold());
    println!();
    println!(
        "This will open a device authorization flow. You'll get a code to enter in your browser."
    );
    println!();

    print!("Start authorization? [Y/n] ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    if input.trim().to_lowercase() == "n" {
        show_later_instructions(info);
        return Ok(());
    }

    // Check if user has custom client credentials
    println!();
    print!("Do you have your own OAuth client credentials? [y/N] ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    let (client_id, client_secret) = if input.trim().to_lowercase() == "y" {
        print!("  Client ID: ");
        io::stdout().flush()?;
        let mut cid = String::new();
        io::stdin().read_line(&mut cid)?;

        print!("  Client Secret (optional, press Enter to skip): ");
        io::stdout().flush()?;
        let cs = read_secret()?;

        (
            cid.trim().to_string(),
            if cs.is_empty() { None } else { Some(cs) },
        )
    } else {
        // Use default public client (would need to be configured per-app)
        match provider {
            OAuthProvider::Google { .. } => {
                println!();
                println!(
                    "{} You need to provide OAuth client credentials for Google.",
                    "Note:".yellow().bold()
                );
                println!(
                    "Get them from: {}",
                    "https://console.cloud.google.com/apis/credentials"
                        .cyan()
                        .underline()
                );
                return Ok(());
            }
            OAuthProvider::Microsoft { .. } => {
                println!();
                println!(
                    "{} You need to provide OAuth client credentials for Microsoft.",
                    "Note:".yellow().bold()
                );
                println!(
                    "Get them from: {}",
                    "https://portal.azure.com/#blade/Microsoft_AAD_RegisteredApps"
                        .cyan()
                        .underline()
                );
                return Ok(());
            }
        }
    };

    println!();
    print!("{}", "Starting device authorization... ".dimmed());
    io::stdout().flush()?;

    let device_auth = match provider {
        OAuthProvider::Google { scopes } => google_device_authorize(&client_id, scopes).await?,
        OAuthProvider::Microsoft { scopes } => {
            ms_device_authorize("common", &client_id, scopes).await?
        }
    };

    println!("{}", "Done!".green());
    println!();
    println!("{}", "=".repeat(50).dimmed());
    println!();
    println!(
        "  Go to: {}",
        device_auth.verification_uri.cyan().bold().underline()
    );
    println!("  Enter code: {}", device_auth.user_code.yellow().bold());
    println!();
    println!("{}", "=".repeat(50).dimmed());
    println!();
    println!("{}", "Waiting for authorization...".dimmed());
    println!("(Press Ctrl+C to cancel)");
    println!();

    // Poll for token
    let interval = device_auth.interval.unwrap_or(5) as u64;
    let max_attempts = (device_auth.expires_in as u64) / interval;

    for attempt in 0..max_attempts {
        tokio::time::sleep(tokio::time::Duration::from_secs(interval)).await;

        let result = match provider {
            OAuthProvider::Google { .. } => {
                google_device_poll(
                    &client_id,
                    client_secret.as_deref(),
                    &device_auth.device_code,
                )
                .await
            }
            OAuthProvider::Microsoft { .. } => {
                ms_device_poll("common", &client_id, &device_auth.device_code).await
            }
        };

        match result {
            Ok(tokens) => {
                // Save tokens
                let mut auth: AuthDetails = HashMap::new();
                auth.insert("access_token".to_string(), tokens.access_token);
                if let Some(rt) = tokens.refresh_token {
                    auth.insert("refresh_token".to_string(), rt);
                }
                if let Some(exp) = tokens.expires_in {
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs() as i64;
                    let expires_at = now + exp - 60;
                    auth.insert("expires_at".to_string(), expires_at.to_string());
                }
                auth.insert("client_id".to_string(), client_id.clone());
                if let Some(ref cs) = client_secret {
                    auth.insert("client_secret".to_string(), cs.clone());
                }

                let store = FileAuthStore::new_default();
                store.save(info.name, &auth).map_err(|e| {
                    CommandError::InvalidConfig(format!("Failed to save tokens: {}", e))
                })?;

                println!("{}", "Authorization successful!".green().bold());
                println!();
                println!("Credentials saved to: {}", store.config_path().dimmed());
                println!();
                println!("{}", "You're all set! Try:".bold());
                println!("  {}", format!("arivu {} list_files", info.name).cyan());

                return Ok(());
            }
            Err(e) => {
                let err_str = e.to_string();
                if err_str.contains("authorization_pending") || err_str.contains("slow_down") {
                    // Still waiting, show progress
                    print!(
                        "\r{} ",
                        format!("Waiting... ({}/{})", attempt + 1, max_attempts).dimmed()
                    );
                    io::stdout().flush()?;
                    continue;
                } else if err_str.contains("access_denied") || err_str.contains("expired") {
                    println!();
                    println!(
                        "{} Authorization was denied or expired.",
                        "Error:".red().bold()
                    );
                    println!(
                        "Run {} to try again.",
                        format!("arivu setup {}", info.name).cyan()
                    );
                    return Ok(());
                } else {
                    // Other error, might still be pending
                    continue;
                }
            }
        }
    }

    println!();
    println!(
        "{} Authorization timed out. Please try again.",
        "Error:".red().bold()
    );

    Ok(())
}

async fn test_connector_auth(connector_name: &str) -> Result<()> {
    let registry = crate::commands::list::create_registry().await?;
    let provider = registry
        .get_provider(connector_name)
        .ok_or_else(|| CommandError::ConnectorNotFound(connector_name.to_string()))?;

    let c = provider.lock().await;
    c.test_auth()
        .await
        .map_err(|e| CommandError::InvalidConfig(format!("Authentication test failed: {}", e)))?;

    Ok(())
}

fn show_later_instructions(info: &ConnectorSetupInfo) {
    println!();
    println!("You can configure later with:");
    println!("  {}", format!("arivu setup {}", info.name).cyan());
}

fn read_secret() -> Result<String> {
    // Use rpassword for hidden input
    match rpassword::read_password() {
        Ok(password) => Ok(password.trim().to_string()),
        Err(_) => {
            // Fallback to regular input if rpassword fails (e.g., in non-TTY)
            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            Ok(input.trim().to_string())
        }
    }
}
