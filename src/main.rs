mod cli;
mod db;
mod mcp;
mod models;
mod tui;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "blueprint", about = "AI-native project management system")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the MCP server
    Serve,
    /// Launch the terminal UI
    Tui,
    /// Show project status
    Status {
        /// Filter by project name
        #[arg(long)]
        project: Option<String>,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Serve => {
            println!("Starting MCP server... (not yet implemented)");
        }
        Commands::Tui => {
            println!("Launching TUI... (not yet implemented)");
        }
        Commands::Status { project } => {
            if let Some(name) = project {
                println!("Status for project: {name} (not yet implemented)");
            } else {
                println!("Overall status (not yet implemented)");
            }
        }
    }

    Ok(())
}
