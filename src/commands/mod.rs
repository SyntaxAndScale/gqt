pub mod add;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "gqt")]
#[command(about = "A terminal-based user interface (TUI) for managing GQueues tasks", long_about = None)]
pub struct Cli {
    #[arg(short, long, help = "Quick add a task using GQueues syntax")]
    pub input: Option<String>,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Add a new task
    Add {
        /// The task title (supports Quick Add Syntax)
        #[arg(help = "The task title (supports Quick Add Syntax)")]
        title: String,
    },
}
