mod candidate_kind;
mod scan_candidate;

use crate::{
    IndexerError, IndexerResult,
    annotation::parse_annotations,
    markdown::parse_markdown,
    model::{Diagnostic, DiagnosticSeverity, SemanticGraph},
    scan::{candidate_kind::CandidateKind, scan_candidate::ScanCandidate},
};

use std::{
    fs,
    io::ErrorKind,
    path::{Component, Path, PathBuf},
};
use walkdir::{DirEntry, WalkDir};

pub fn scan_repository(root: &Path) -> IndexerResult<SemanticGraph> {
    if !root.exists() || !root.is_dir() {
        return Err(IndexerError::invalid_root(root.to_path_buf()));
    }

    let mut graph = SemanticGraph::default();

    for entry in WalkDir::new(root)
        .into_iter()
        .filter_entry(|entry| !is_excluded_dir(root, entry))
    {
        let entry = entry.map_err(|error| {
            IndexerError::walk_entry(
                error
                    .path()
                    .map(|path| path.to_path_buf())
                    .unwrap_or_else(|| root.to_path_buf()),
                error
                    .into_io_error()
                    .unwrap_or_else(|| std::io::Error::other("walkdir error")),
            )
        })?;

        if !entry.file_type().is_file() {
            continue;
        }

        let path = entry.path();
        let Some(candidate) = classify_path(root, path) else {
            continue;
        };

        let contents = match fs::read_to_string(path) {
            Ok(contents) => contents,
            Err(source) => {
                graph.diagnostics.push(read_failure_diagnostic(
                    &candidate.display_path,
                    source.kind(),
                    source,
                ));
                continue;
            }
        };

        match candidate.kind {
            CandidateKind::Document => {
                let report = parse_markdown(&candidate.display_path, &contents)?;
                if let Some(document) = report.value {
                    graph.documents.push(document);
                }
                graph.diagnostics.extend(report.diagnostics);
            }
            CandidateKind::AnnotationSource => {
                let report = parse_annotations(&candidate.display_path, &contents)?;
                graph.annotations.extend(report.value);
                graph.diagnostics.extend(report.diagnostics);
            }
        }
    }

    graph
        .documents
        .sort_by(|left, right| left.path.cmp(&right.path));

    let documents = std::mem::take(&mut graph.documents);
    let mut deduped_documents = Vec::new();
    let mut seen_document_ids = std::collections::BTreeMap::<String, PathBuf>::new();

    for document in documents {
        match seen_document_ids.entry(document.id.clone()) {
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(document.path.clone());
                deduped_documents.push(document);
            }
            std::collections::btree_map::Entry::Occupied(first_path) => {
                graph.diagnostics.push(Diagnostic {
                    severity: DiagnosticSeverity::Error,
                    path: document.path.clone(),
                    line: None,
                    message: format!(
                        "duplicate markdown id `{}`; first path `{}` wins",
                        document.id,
                        first_path.get().display()
                    ),
                });
            }
        }
    }

    graph.documents = deduped_documents;
    graph
        .annotations
        .sort_by(|left, right| left.path.cmp(&right.path).then(left.line.cmp(&right.line)));
    graph
        .diagnostics
        .sort_by(|left, right| left.path.cmp(&right.path).then(left.line.cmp(&right.line)));

    Ok(graph)
}

fn is_excluded_dir(root: &Path, entry: &DirEntry) -> bool {
    if !entry.file_type().is_dir() {
        return false;
    }

    if entry.path() == root {
        return false;
    }

    let Some(relative_path) = entry.path().strip_prefix(root).ok() else {
        return false;
    };

    let under_docs = matches!(
        relative_path.components().next(),
        Some(Component::Normal(name)) if name == ".docs"
    );

    let parent_name = entry
        .path()
        .parent()
        .and_then(|path| path.file_name())
        .and_then(|name| name.to_str());

    let file_name = entry.file_name().to_str();

    matches!(
        file_name,
        Some(".git" | "target" | ".idea" | ".vscode" | ".vs" | ".codex" | "node_modules" | "obj")
    ) || matches!(file_name, Some(name) if (name == "tests" || name == "Tests" || name.ends_with(".Tests")))
        && !under_docs
        || matches!(file_name, Some("bin")) && parent_name.is_some_and(|name| name != "src")
}

fn classify_path(root: &Path, path: &Path) -> Option<ScanCandidate> {
    let display_path = path.strip_prefix(root).ok()?.to_path_buf();

    match path.extension().and_then(|ext| ext.to_str()) {
        Some("md") if is_indexable_doc(&display_path) => Some(ScanCandidate {
            display_path,
            kind: CandidateKind::Document,
        }),
        Some("rs" | "cs") => Some(ScanCandidate {
            display_path,
            kind: CandidateKind::AnnotationSource,
        }),
        _ => None,
    }
}

fn is_indexable_doc(path: &Path) -> bool {
    matches!(
        path.components().next(),
        Some(Component::Normal(name)) if name == ".docs"
    )
}

fn read_failure_diagnostic(path: &Path, kind: ErrorKind, source: std::io::Error) -> Diagnostic {
    let message = match kind {
        ErrorKind::InvalidData => "file is not valid UTF-8".to_string(),
        _ => format!("failed to read file: {source}"),
    };

    Diagnostic {
        severity: DiagnosticSeverity::Error,
        path: path.to_path_buf(),
        line: None,
        message,
    }
}
