mod cli;
mod commands;

// ---------------------------------------------------------------------------------------------- //

use cli::Cli;
use commands::{Command, explain, index, init, serve};

use indexer::IndexerResult;

use clap::Parser;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

async fn run() -> IndexerResult<()> {
    match Cli::parse().command {
        Command::Init { root } => init::run(&root).await,
        Command::Index { root } => index::run(&root).await,
        Command::Explain { id, root } => explain::run(&root, &id).await,
        Command::Serve { root } => serve::run(root).await,
    }
}
