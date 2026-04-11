use crate::commands::Command;

use clap::Parser;

#[derive(Parser)]
#[command(name = "indexer", about = "Soul semantic indexer")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}
