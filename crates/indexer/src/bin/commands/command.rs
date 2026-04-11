use clap::Subcommand;
use std::path::PathBuf;

#[derive(Subcommand)]
pub enum Command {
    /// Initialize a new Soul repository
    Init {
        #[arg(long, default_value = ".")]
        root: PathBuf,
    },
    /// Scan and write the index
    Index {
        #[arg(long, default_value = ".")]
        root: PathBuf,
    },
    /// Explain a semantic ID
    Explain {
        id: String,
        #[arg(long, default_value = ".")]
        root: PathBuf,
    },
    /// Start the MCP stdio server
    Serve {
        #[arg(long, default_value = ".")]
        root: PathBuf,
    },
}
