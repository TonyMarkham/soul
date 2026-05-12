use crate::{
    graph::ExplainResult,
    model::{CodeAnnotation, Document, Reference, SemanticGraph},
};

use std::collections::BTreeSet;

pub fn explain_result(result: &ExplainResult) -> String {
    let mut out = String::new();
    out.push_str(&format!("# Soul ID: {}\n\n", markdown_text(&result.id)));

    let has_documents = !result.documents.is_empty();
    let has_annotations = !result.annotations.is_empty();
    let has_references = !result.references.is_empty();

    if has_documents {
        out.push_str("## Documents\n\n");
        for doc in &result.documents {
            out.push_str(&document(doc));
            out.push('\n');
        }
    }

    if has_annotations {
        out.push_str("## Annotations\n\n");
        for ann in &result.annotations {
            out.push_str(&annotation(ann));
            out.push('\n');
        }
    }

    if has_references {
        out.push_str("## Referenced by\n\n");
        for reference in &result.references {
            out.push_str(&reference_row(reference));
            out.push('\n');
        }
    }

    if !has_documents && !has_annotations && !has_references {
        out.push_str("No documents, annotations, or references found for this ID.\n");
    }

    if !result.scan_diagnostics.is_empty() {
        out.push_str("\n## Diagnostics\n\n");
        for diag in &result.scan_diagnostics {
            match diag.line {
                Some(line) => out.push_str(&format!(
                    "- {}:{} {}\n",
                    markdown_code_span(&visible_path(&diag.path)),
                    line,
                    markdown_text(&diag.message)
                )),
                None => out.push_str(&format!(
                    "- {} {}\n",
                    markdown_code_span(&visible_path(&diag.path)),
                    markdown_text(&diag.message)
                )),
            }
        }
    }

    out
}

pub fn document(doc: &Document) -> String {
    format!(
        "- [{}] {} — {}\n  Title: {}\n  -> Read {} for the full specification",
        markdown_text(&doc.kind),
        markdown_code_span(&visible_text(&doc.id)),
        markdown_code_span(&visible_path(&doc.path)),
        markdown_text(doc.title.as_deref().unwrap_or("(untitled)")),
        markdown_code_span(&visible_path(&doc.path)),
    )
}

pub fn annotation(ann: &CodeAnnotation) -> String {
    let meta: Vec<String> = ann
        .metadata
        .iter()
        .map(|(k, v)| {
            format!(
                "{}={}",
                visible_text(k),
                visible_text(v.as_str().unwrap_or(&v.to_string()))
            )
        })
        .collect();

    if meta.is_empty() {
        format!(
            "- {} @ {}:{}",
            markdown_code_span(&visible_text(&ann.id)),
            markdown_code_span(&visible_path(&ann.path)),
            ann.line
        )
    } else {
        format!(
            "- {} @ {}:{} [{}]",
            markdown_code_span(&visible_text(&ann.id)),
            markdown_code_span(&visible_path(&ann.path)),
            ann.line,
            meta.into_iter()
                .map(|item| markdown_code_span(&item))
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}

pub fn reference_row(reference: &Reference) -> String {
    let base = format!(
        "{}:{}:{}-{} -> {}",
        visible_path(&reference.source_path),
        reference.source_start_line,
        reference.source_start_col,
        reference.source_end_col,
        visible_text(&reference.source_id)
    );

    match &reference.display_text {
        Some(display_text) => format!(
            "- {} ({})",
            markdown_code_span(&base),
            markdown_text(&visible_text(display_text))
        ),
        None => format!("- {}", markdown_code_span(&base)),
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
    out.push_str(
        "Code is annotated with these IDs but no document exists. These need documentation created.\n\n",
    );
    for id in &unlinked {
        out.push_str(&format!("  {}\n", markdown_code_span(&visible_text(id))));
        for ann in graph.annotations.iter().filter(|a| a.id.as_str() == *id) {
            out.push_str(&format!(
                "    @ {}:{}\n",
                markdown_code_span(&visible_path(&ann.path)),
                ann.line
            ));
        }
    }

    out.push_str(&format!("\n## Undocumented IDs ({})\n", undocumented.len()));
    out.push_str("A document exists for these IDs but no code annotation links to them.\n\n");
    for id in &undocumented {
        if let Some(doc) = graph.documents.iter().find(|d| d.id.as_str() == *id) {
            out.push_str(&format!(
                "  {} — {} ({})\n",
                markdown_code_span(&visible_text(id)),
                markdown_code_span(&visible_path(&doc.path)),
                markdown_text(doc.title.as_deref().unwrap_or("untitled")),
            ));
        } else {
            out.push_str(&format!("  {}\n", markdown_code_span(&visible_text(id))));
        }
    }

    out
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

fn visible_path(path: &std::path::Path) -> String {
    visible_text(&path.display().to_string())
}

fn markdown_text(input: &str) -> String {
    let mut out = String::new();
    for ch in input.chars() {
        match ch {
            '\\' | '*' | '_' | '`' | '[' | ']' | '(' | ')' | '#' | '+' | '-' | '!' | '>' | '|' => {
                out.push('\\');
                out.push(ch);
            }
            _ => out.push(ch),
        }
    }
    out
}

fn markdown_code_span(input: &str) -> String {
    if input.contains('`') {
        markdown_text(input)
    } else {
        format!("`{input}`")
    }
}
