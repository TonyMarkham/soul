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
        "Indexed {} documents, {} annotations, {} references, {} diagnostics → {}",
        graph.documents.len(),
        graph.annotations.len(),
        graph.references.len(),
        graph.diagnostics.len(),
        visible_path(&index_path),
    );

    Ok(())
}

fn visible_text(input: &str) -> String {
    let mut out = String::new();
    for ch in input.chars() {
        match ch {
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ch if ch.is_control() => out.push_str(&format!("\\u{{{:X}}}", ch as u32)),
            ch => out.push(ch),
        }
    }
    out
}

fn visible_path(path: &Path) -> String {
    visible_text(&path.display().to_string())
}
