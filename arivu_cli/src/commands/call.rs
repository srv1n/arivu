use crate::cli::Cli;
use crate::commands::usage_helpers::print_cost_summary;
use crate::commands::{copy_to_clipboard, CommandError, Result};
use crate::output::{format_output, format_pretty, OutputData};
use arivu_core::{CallToolRequestParam, PaginatedRequestParam};
use owo_colors::OwoColorize;
use serde_json::{json, Map, Value};

pub async fn run(
    cli: &Cli,
    connector: &str,
    tool: &str,
    args_json: Option<&str>,
    params: &[String],
) -> Result<()> {
    let registry = crate::commands::list::create_registry().await?;
    let provider = registry
        .get_provider(connector)
        .ok_or_else(|| CommandError::ConnectorNotFound(connector.to_string()))?;

    // Lock the provider once
    let c = provider.lock().await;

    let mut args_map: Map<String, Value> = Map::new();

    // 1. Handle JSON args if present
    if let Some(s) = args_json {
        if !s.trim().is_empty() {
            let v: Value = serde_json::from_str(s)?;
            match v {
                Value::Object(m) => args_map = m,
                _ => {
                    return Err(CommandError::InvalidConfig(
                        "--args must be a JSON object".to_string(),
                    ))
                }
            }
        }
    }

    // 2. Handle positional params if present (smart mapping)
    if !params.is_empty() {
        // We need to know the parameter names to map positional args.
        // Fetch the tool definition.
        let tools_response = c
            .list_tools(Some(PaginatedRequestParam { cursor: None }))
            .await?;

        let tool_def = tools_response
            .tools
            .iter()
            .find(|t| t.name == tool)
            .ok_or_else(|| CommandError::ToolNotFound(tool.to_string(), connector.to_string()))?;

        // Extract property names from the JSON schema
        let mut param_names: Vec<String> = Vec::new();

        {
            let schema = &tool_def.input_schema;
            if let Some(properties) = schema.get("properties").and_then(|v| v.as_object()) {
                // Heuristic: Order by 'required' array first, then others?
                // Or just use the order they appear? JSON objects are unordered in standard,
                // but usually preserved in serde_json::Map if using "preserve_order" feature,
                // which isn't guaranteed here.
                // BETTER STRATEGY: Use the 'required' list as the priority order.
                let mut required: Vec<String> = schema
                    .get("required")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                            .collect()
                    })
                    .unwrap_or_default();

                // Add any other properties that aren't in required
                for key in properties.keys() {
                    if !required.contains(key) {
                        required.push(key.clone());
                    }
                }
                param_names = required;
            }
        }

        // Parse arguments - support both styles:
        // 1. Named args: --name value or -n value (anywhere in the args)
        // 2. Positional args: values without flags
        // The -- separator can be used to force remaining args as positional
        let mut positional_args: Vec<&String> = Vec::new();
        let mut named_args: Vec<(String, String)> = Vec::new();
        let mut force_positional = false;
        let mut i = 0;

        while i < params.len() {
            let param = &params[i];

            // The -- separator forces remaining args to be positional
            if param == "--" {
                force_positional = true;
                i += 1;
                continue;
            }

            if !force_positional {
                // Check for --name value or --name=value style
                if let Some(flag_part) = param.strip_prefix("--") {
                    // Handle --name=value style
                    if let Some(eq_pos) = flag_part.find('=') {
                        let name = flag_part[..eq_pos].replace('-', "_");
                        let value = flag_part[eq_pos + 1..].to_string();
                        named_args.push((name, value));
                        i += 1;
                        continue;
                    }

                    // Handle --name value style
                    if i + 1 < params.len() && !params[i + 1].starts_with('-') {
                        let name = flag_part.replace('-', "_");
                        named_args.push((name, params[i + 1].clone()));
                        i += 2;
                        continue;
                    }

                    // Boolean flag (--flag without value)
                    let name = flag_part.replace('-', "_");
                    named_args.push((name, "true".to_string()));
                    i += 1;
                    continue;
                }

                // Check for -n value style (single char flags)
                if param.starts_with('-') && param.len() == 2 && i + 1 < params.len() {
                    let name = param[1..].to_string();
                    named_args.push((name, params[i + 1].clone()));
                    i += 2;
                    continue;
                }
            }

            // Everything else is a positional argument
            positional_args.push(param);
            i += 1;
        }

        // Check positional arg count
        if positional_args.len() > param_names.len() {
            return Err(CommandError::InvalidConfig(format!(
                "Too many arguments provided. Tool '{}' accepts at most {} positional arguments ({}), but got {}.",
                tool,
                param_names.len(),
                param_names.join(", "),
                positional_args.len()
            )));
        }

        // Map positional args to names
        for (i, param_value) in positional_args.iter().enumerate() {
            let param_name = &param_names[i];
            // Try to guess type? For now, treat everything as string unless it looks like a number/bool
            let value = if let Ok(num) = param_value.parse::<i64>() {
                json!(num)
            } else if let Ok(b) = param_value.parse::<bool>() {
                json!(b)
            } else {
                json!(param_value)
            };
            args_map.insert(param_name.clone(), value);
        }

        // Map named args
        for (name, value) in named_args {
            // name is already normalized (kebab to snake_case done during parsing)
            let typed_value = if let Ok(num) = value.parse::<i64>() {
                json!(num)
            } else if let Ok(b) = value.parse::<bool>() {
                json!(b)
            } else {
                json!(&value)
            };
            args_map.insert(name, typed_value);
        }
    }

    let request = CallToolRequestParam {
        name: tool.to_string().into(),
        arguments: Some(args_map.into_iter().collect()),
    };

    let result = match c.call_tool(request).await {
        Ok(r) => r,
        Err(e) => {
            // On tool not found, show available tools
            if matches!(&e, arivu_core::error::ConnectorError::ToolNotFound) {
                eprintln!(
                    "{} Tool '{}' not found for connector '{}'.",
                    "Error:".red().bold(),
                    tool,
                    connector
                );
                eprintln!();

                // Try to list available tools (we already have the list from above check or need to fetch)
                // If we fetched above, great, if not (because no params), fetch now.
                // Simplification: just re-fetch or use if available.
                // For error display logic, it's fine to re-fetch or just return error.
            }
            return Err(e.into());
        }
    };

    let meta_value = result
        .meta
        .as_ref()
        .and_then(|m| serde_json::to_value(m).ok());

    // Prefer structured_content if present
    let payload = if let Some(sc) = result.structured_content {
        sc
    } else {
        serde_json::to_value(&result).unwrap_or_else(|_| json!({"ok": true}))
    };

    match cli.output {
        crate::cli::OutputFormat::Pretty => {
            println!(
                "{} {}.{}",
                "Call".bold().cyan(),
                connector.yellow(),
                tool.cyan()
            );
            println!();
            println!("{}", format_pretty(&payload));
        }
        _ => {
            let data = OutputData::CallResult {
                connector: connector.to_string(),
                tool: tool.to_string(),
                result: payload.clone(),
                meta: meta_value.clone(),
            };
            format_output(&data, &cli.output)?;
        }
    }

    // Copy to clipboard if requested
    if cli.copy {
        let text = serde_json::to_string_pretty(&payload)?;
        copy_to_clipboard(&text)?;
    }

    print_cost_summary(&cli.output, meta_value.as_ref());

    Ok(())
}
