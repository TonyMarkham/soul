mod candidate_kind;
mod scan_candidate;

// ---------------------------------------------------------------------------------------------- //

use crate::{
    IndexerError, IndexerResult,
    annotation::{PluginRegistry, parse_annotations},
    config::SoulConfig,
    markdown::{annotations::extract_annotations, parse_markdown},
    model::{Diagnostic, DiagnosticSeverity, MarkdownSource, Reference, SemanticGraph},
    scan::{candidate_kind::CandidateKind, scan_candidate::ScanCandidate},
};

use soul_attributes::soul;

use std::{
    collections::BTreeSet,
    fs,
    io::ErrorKind,
    path::{Path, PathBuf},
};

use walkdir::{DirEntry, WalkDir};

#[soul(id = "indexer.scan-repository")]
pub fn scan_repository(
    root: &Path,
    config: &SoulConfig,
    registry: &PluginRegistry,
) -> IndexerResult<SemanticGraph> {
    if !root.exists() || !root.is_dir() {
        return Err(IndexerError::invalid_root(root.to_path_buf()));
    }

    let mut graph = SemanticGraph::default();

    for entry in WalkDir::new(root)
        .into_iter()
        .filter_entry(|entry| !is_excluded_dir(root, entry, config))
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
        let Some(candidate) = classify_path(root, path, registry) else {
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
                let ann_report = extract_annotations(&candidate.display_path, &contents)?;

                let parse_value = report.value;
                let document = parse_value.document;
                let document_id = document.as_ref().map(|document| document.id.clone());
                let wiki_links = parse_value.wiki_links;
                let wiki_link_diagnostics = parse_value.wiki_link_diagnostics;

                let annotations = ann_report.value;
                let annotation_ids: BTreeSet<String> = annotations
                    .iter()
                    .map(|annotation| annotation.id.clone())
                    .collect();
                let source_path = candidate.display_path.clone();

                if let Some(document) = document {
                    graph.documents.push(document);
                }
                graph.annotations.extend(annotations);
                graph.diagnostics.extend(report.diagnostics);
                graph.diagnostics.extend(ann_report.diagnostics);

                let source_id = resolve_wikilink_source_id(
                    document_id.as_deref(),
                    &annotation_ids,
                    &source_path,
                    &mut graph.diagnostics,
                );

                let emit_wikilink_diagnostics = document_id.is_some() || !annotation_ids.is_empty();
                if emit_wikilink_diagnostics {
                    graph.diagnostics.extend(wiki_link_diagnostics);
                }

                if let Some(source_id) = source_id {
                    if document_id.is_none() {
                        graph.markdown_sources.push(MarkdownSource {
                            source_id: source_id.clone(),
                            source_path: source_path.clone(),
                        });
                    }

                    graph
                        .references
                        .extend(wiki_links.into_iter().map(|token| Reference {
                            source_id: source_id.clone(),
                            target_id: token.target_id,
                            source_path: source_path.clone(),
                            source_start_line: token.start_line,
                            source_start_col: token.start_col,
                            source_end_line: token.end_line,
                            source_end_col: token.end_col,
                            display_text: token.display_text,
                        }));
                }
            }
            CandidateKind::AnnotationSource => {
                let report = parse_annotations(&candidate.display_path, &contents, registry)?;
                graph.annotations.extend(report.value);
                graph.diagnostics.extend(report.diagnostics);
            }
        }
    }

    graph
        .documents
        .sort_by(|left, right| left.path.cmp(&right.path).then(left.id.cmp(&right.id)));

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

    let surviving_document_pairs: BTreeSet<(String, PathBuf)> = graph
        .documents
        .iter()
        .map(|document| (document.id.clone(), document.path.clone()))
        .collect();

    let surviving_markdown_source_pairs: BTreeSet<(String, PathBuf)> = graph
        .markdown_sources
        .iter()
        .map(|source| (source.source_id.clone(), source.source_path.clone()))
        .collect();

    graph.references.retain(|reference| {
        let source_pair = (reference.source_id.clone(), reference.source_path.clone());
        surviving_document_pairs.contains(&source_pair)
            || surviving_markdown_source_pairs.contains(&source_pair)
    });

    graph.annotations.sort_by(|left, right| {
        left.path
            .cmp(&right.path)
            .then(left.line.cmp(&right.line))
            .then(left.id.cmp(&right.id))
    });
    graph.references.sort_by(|left, right| {
        left.source_path
            .cmp(&right.source_path)
            .then(left.source_start_line.cmp(&right.source_start_line))
            .then(left.source_start_col.cmp(&right.source_start_col))
            .then(left.target_id.cmp(&right.target_id))
    });
    graph.markdown_sources.sort_by(|left, right| {
        left.source_path
            .cmp(&right.source_path)
            .then(left.source_id.cmp(&right.source_id))
    });
    graph.diagnostics.sort_by(|left, right| {
        left.path
            .cmp(&right.path)
            .then(left.line.cmp(&right.line))
            .then(left.message.cmp(&right.message))
    });

    Ok(graph)
}

fn resolve_wikilink_source_id(
    document_id: Option<&str>,
    annotation_ids: &BTreeSet<String>,
    path: &Path,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<String> {
    if let Some(document_id) = document_id {
        if annotation_ids.is_empty()
            || (annotation_ids.len() == 1 && annotation_ids.contains(document_id))
        {
            return Some(document_id.to_string());
        }

        diagnostics.push(Diagnostic {
            severity: DiagnosticSeverity::Error,
            path: path.to_path_buf(),
            line: None,
            message: "wiki links skipped: frontmatter id does not match markdown annotation ids"
                .to_string(),
        });
        return None;
    }

    if annotation_ids.len() == 1 {
        return annotation_ids.iter().next().cloned();
    }

    if annotation_ids.len() > 1 {
        diagnostics.push(Diagnostic {
            severity: DiagnosticSeverity::Error,
            path: path.to_path_buf(),
            line: None,
            message: "wiki links skipped: multiple markdown annotation ids".to_string(),
        });
    }

    None
}

fn is_excluded_dir(root: &Path, entry: &DirEntry, config: &SoulConfig) -> bool {
    if !entry.file_type().is_dir() || entry.path() == root {
        return false;
    }

    let Some(name) = entry.file_name().to_str() else {
        return false;
    };

    if config.scan.excluded_dirs.iter().any(|d| d == name) {
        return true;
    }

    if config
        .scan
        .excluded_dir_suffixes
        .iter()
        .any(|s| name.ends_with(s.as_str()))
    {
        return true;
    }

    if name == "bin" {
        let parent_name = entry
            .path()
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str());
        return !parent_name
            .is_some_and(|p| config.scan.excluded_bin_except_under.iter().any(|s| s == p));
    }

    false
}

fn classify_path(root: &Path, path: &Path, registry: &PluginRegistry) -> Option<ScanCandidate> {
    let display_path = path.strip_prefix(root).ok()?.to_path_buf();

    match path.extension().and_then(|ext| ext.to_str()) {
        Some("md") => Some(ScanCandidate {
            display_path,
            kind: CandidateKind::Document,
        }),
        Some(ext) if registry.parser_for_extension(ext).is_some() => Some(ScanCandidate {
            display_path,
            kind: CandidateKind::AnnotationSource,
        }),
        _ => None,
    }
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
