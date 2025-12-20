use crate::cli::Cli;
use crate::commands::Result;
use crate::output::{format_output, OutputData};
use arivu_core::auth_store::{AuthStore, FileAuthStore};
use arivu_core::{ProviderRegistry, UsageManager};
use comfy_table::{modifiers::UTF8_ROUND_CORNERS, presets::UTF8_FULL, ContentArrangement, Table};
use owo_colors::OwoColorize;

/// Get the terminal width, defaulting to 80 if detection fails
fn get_terminal_width() -> u16 {
    terminal_size::terminal_size()
        .map(|(w, _)| w.0)
        .unwrap_or(80)
}

/// Truncate text to fit within a given width, adding "..." if truncated
fn truncate_text(text: &str, max_width: usize) -> String {
    if text.len() <= max_width {
        text.to_string()
    } else if max_width > 3 {
        format!("{}...", &text[..max_width - 3])
    } else {
        text.chars().take(max_width).collect()
    }
}

pub async fn run(cli: &Cli) -> Result<()> {
    let registry = create_registry().await?;
    let providers = registry.list_providers();

    if providers.is_empty() {
        println!("{}", "No connectors available".yellow());
        return Ok(());
    }

    let output_data = OutputData::ConnectorList(providers.clone());

    match cli.output {
        crate::cli::OutputFormat::Pretty => {
            let term_width = get_terminal_width() as usize;
            let desc_width = term_width.saturating_sub(30);

            println!("{}", "Available Data Sources".bold().cyan());
            println!();

            let mut table = Table::new();
            table
                .load_preset(UTF8_FULL)
                .apply_modifier(UTF8_ROUND_CORNERS)
                .set_content_arrangement(ContentArrangement::Dynamic)
                .set_width(term_width as u16)
                .set_header(vec!["Name", "Description"]);

            for provider in &providers {
                table.add_row(vec![
                    provider.name.clone(),
                    truncate_text(&provider.description, desc_width.max(30)),
                ]);
            }

            println!("{}", table);
            println!();
            println!(
                "{} Use {} to see available tools for a connector",
                "Tip:".green().bold(),
                "arivu tools <connector>".cyan()
            );
        }
        _ => {
            format_output(&output_data, &cli.output)?;
        }
    }

    Ok(())
}

pub async fn create_registry() -> Result<ProviderRegistry> {
    // Use the core helper to build a registry with only feature-enabled connectors.
    let registry = match UsageManager::new_default() {
        Ok(usage) => {
            arivu_core::build_registry_enabled_only_with_usage(std::sync::Arc::new(usage)).await
        }
        Err(err) => {
            tracing::debug!(
                "Usage manager init failed, continuing without metering: {}",
                err
            );
            arivu_core::build_registry_enabled_only().await
        }
    };

    // Load saved credentials from auth store and set them on each connector
    let auth_store = FileAuthStore::new_default();
    for provider_info in registry.list_providers() {
        // Try to load credentials for this provider (by name and any aliases)
        let names_to_try = [provider_info.name.as_str()];
        for name in names_to_try {
            if let Some(auth) = auth_store.load(name) {
                if let Some(provider) = registry.get_provider(&provider_info.name) {
                    let mut connector = provider.lock().await;
                    // Silently set auth - errors are ok (connector might not need this auth)
                    let _ = connector.set_auth_details(auth).await;
                    break;
                }
            }
        }
    }

    Ok(registry)
}
