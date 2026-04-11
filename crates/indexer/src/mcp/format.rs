use crate::{
    graph::ExplainResult,
    model::{CodeAnnotation, Document, SemanticGraph},
};

use std::collections::BTreeSet;

pub fn explain_result(result: &ExplainResult) -> String {
    let mut out = String::new();
    out.push_str(&format!("# Soul ID: {}\n\n", result.id));
    if result.documents.is_empty() && result.annotations.is_empty() {
        out.push_str("No documents or annotations found for this ID.\n");
        return out;
    }
    if !result.documents.is_empty() {
        out.push_str("## Documents\n\n");
        for doc in &result.documents {
            out.push_str(&document(doc));
            out.push('\n');
        }
    }
    if !result.annotations.is_empty() {
        out.push_str("## Annotations\n\n");
        for ann in &result.annotations {
            out.push_str(&annotation(ann));
            out.push('\n');
        }
    }
    out
}

pub fn document(doc: &Document) -> String {
    format!(
        "- [{}] {} — {}\n  Title: {}\n  -> Read {} for the full specification",
        doc.kind,
        doc.id,
        doc.path.display(),
        doc.title.as_deref().unwrap_or("(untitled)"),
        doc.path.display(),
    )
}

pub fn annotation(ann: &CodeAnnotation) -> String {
    let meta: Vec<String> = ann
        .metadata
        .iter()
        .map(|(k, v)| format!("{k}={}", v.as_str().unwrap_or(&v.to_string())))
        .collect();
    if meta.is_empty() {
        format!("- {} @ {}:{}", ann.id, ann.path.display(), ann.line)
    } else {
        format!(
            "- {} @ {}:{} [{}]",
            ann.id,
            ann.path.display(),
            ann.line,
            meta.join(", ")
        )
    }
}

pub fn gaps(graph: &SemanticGraph) -> String {
    let doc_ids: BTreeSet<&str> = graph.documents.iter().map(|d| d.id.as_str()).collect();
    let ann_ids: BTreeSet<&str> = graph.annotations.iter().map(|a| a.id.as_str()).collect();

    let unlinked: BTreeSet<&str> = ann_ids
        .iter()
        .copied()
        .filter(|id| !doc_ids.contains(id))
        .collect();
    let undocumented: BTreeSet<&str> = doc_ids
        .iter()
        .copied()
        .filter(|id| !ann_ids.contains(id))
        .collect();

    let mut out = String::new();

    out.push_str(&format!("## Unlinked Annotations ({})\n", unlinked.len()));
    out.push_str("Code is annotated with these IDs but no document exists. These need documentation created.\n\n");
    for id in &unlinked {
        out.push_str(&format!("  {id}\n"));
        for ann in graph.annotations.iter().filter(|a| a.id.as_str() == *id) {
            out.push_str(&format!("    @ {}:{}\n", ann.path.display(), ann.line));
        }
    }

    out.push_str(&format!("\n## Undocumented IDs ({})\n", undocumented.len()));
    out.push_str("A document exists for these IDs but no code annotation links to them.\n\n");
    for id in &undocumented {
        if let Some(doc) = graph.documents.iter().find(|d| d.id.as_str() == *id) {
            out.push_str(&format!(
                "  {} — {} ({})\n",
                id,
                doc.path.display(),
                doc.title.as_deref().unwrap_or("untitled"),
            ));
        } else {
            out.push_str(&format!("  {id}\n"));
        }
    }

    out
}
