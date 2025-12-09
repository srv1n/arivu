#[cfg(feature = "tui")]
use crate::commands::Result;

#[cfg(feature = "tui")]
pub async fn run() -> Result<()> {
    println!("🚧 TUI mode is not yet implemented");
    println!("This will launch an interactive dashboard with:");
    println!("  • Live connector status");
    println!("  • Search interface");
    println!("  • Configuration management");
    println!("  • Real-time data streaming");
    println!();
    println!("For now, use the CLI commands:");
    println!("  arivu list");
    println!("  arivu search <connector> <query>");
    println!("  arivu get <connector> <id>");
    println!("  arivu config show");

    Ok(())
}

#[cfg(not(feature = "tui"))]
pub async fn run() -> Result<()> {
    Err(CommandError::InvalidConfig(
        "TUI feature not enabled. Compile with --features tui".to_string(),
    ))
}
