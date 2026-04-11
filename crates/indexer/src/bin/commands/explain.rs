use indexer::{
    ExplainResult, IndexerError, IndexerResult,
    annotation::PluginRegistry,
    config::load_config,
    explain,
    index::{explain_from_index, open_index},
    scan_repository,
};

use std::path::Path;

pub async fn run(root: &Path, id: &str) -> IndexerResult<()> {
    if !root.exists() || !root.is_dir() {
        return Err(IndexerError::invalid_root(root.to_path_buf()));
    }

    let config = load_config(root)?;
    let registry = PluginRegistry::load(&config.plugins, root)?;
    let result = match open_index(root).await? {
        Some(pool) => explain_from_index(&pool, id).await?,
        None => {
            let graph = scan_repository(root, &config, &registry)?;
            explain(&graph, id)
        }
    };
    print_result(&result);
    Ok(())
}

fn print_result(result: &ExplainResult) {
    println!("ID: {}", result.id);
    println!();
    println!("Documents:");
    if result.documents.is_empty() {
        println!("none");
    } else {
        for doc in &result.documents {
            match &doc.title {
                Some(title) => println!(
                    "- {} [kind={}, title={}]",
                    doc.path.display(),
                    doc.kind,
                    title
                ),
                None => println!("- {} [kind={}]", doc.path.display(), doc.kind),
            }
        }
    }
    println!();
    println!("Annotations:");
    if result.annotations.is_empty() {
        println!("none");
    } else {
        for ann in &result.annotations {
            println!("- {}:{}", ann.path.display(), ann.line);
        }
    }
    if !result.scan_diagnostics.is_empty() {
        println!();
        println!("Diagnostics:");
        for diag in &result.scan_diagnostics {
            match diag.line {
                Some(line) => println!("- {}:{} {}", diag.path.display(), line, diag.message),
                None => println!("- {} {}", diag.path.display(), diag.message),
            }
        }
    }
}
