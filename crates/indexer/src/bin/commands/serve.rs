use indexer::{IndexerError, IndexerResult, SoulServer};

use rmcp::{ServiceExt, transport::stdio};
use std::path::PathBuf;

pub async fn run(root: PathBuf) -> IndexerResult<()> {
    SoulServer::new(root)?
        .serve(stdio())
        .await
        .map_err(IndexerError::mcp)?
        .waiting()
        .await
        .map_err(IndexerError::mcp)?;
    Ok(())
}
