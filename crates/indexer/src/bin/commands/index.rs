use indexer::{
    IndexerResult, annotation::PluginRegistry, config::load_config, index::write_index,
    scan_repository,
};

use std::path::Path;

pub async fn run(root: &Path) -> IndexerResult<()> {
    let config = load_config(root)?;
    let registry = PluginRegistry::load(&config.plugins, root)?;
    let graph = scan_repository(root, &config, &registry)?;
    let index_path = write_index(root, &graph).await?;

    println!(
        "Indexed {} documents, {} annotations, {} diagnostics → {}",
        graph.documents.len(),
        graph.annotations.len(),
        graph.diagnostics.len(),
        index_path.display(),
    );
    Ok(())
}
