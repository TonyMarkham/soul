use crate::{
    graph::ExplainResult,
    mcp::format::explain_result,
    model::{Diagnostic, DiagnosticSeverity, Reference},
};

use std::path::PathBuf;

fn reference(display_text: Option<&str>) -> Reference {
    Reference {
        source_id: "source.id".to_string(),
        target_id: "target.id".to_string(),
        source_path: PathBuf::from("docs/source.md"),
        source_start_line: 5,
        source_start_col: 4,
        source_end_line: 5,
        source_end_col: 17,
        display_text: display_text.map(str::to_string),
    }
}

#[test]
fn referenced_by_section_renders_when_references_exist() {
    let result = ExplainResult {
        id: "target.id".to_string(),
        documents: vec![],
        annotations: vec![],
        references: vec![reference(Some("Display"))],
        scan_diagnostics: vec![],
    };

    let out = explain_result(&result);
    assert!(out.contains("## Referenced by"));
    assert!(out.contains("docs/source.md:5:4-17 -> source.id"));
    assert!(out.contains("(Display)"));
}

#[test]
fn references_prevent_empty_message() {
    let result = ExplainResult {
        id: "target.id".to_string(),
        documents: vec![],
        annotations: vec![],
        references: vec![reference(None)],
        scan_diagnostics: vec![],
    };

    let out = explain_result(&result);
    assert!(!out.contains("No documents, annotations, or references found for this ID."));
}

#[test]
fn diagnostics_render_when_other_sections_are_empty() {
    let result = ExplainResult {
        id: "target.id".to_string(),
        documents: vec![],
        annotations: vec![],
        references: vec![],
        scan_diagnostics: vec![Diagnostic {
            severity: DiagnosticSeverity::Error,
            path: PathBuf::from("docs/bad.md"),
            line: Some(1),
            message: "bad markdown".to_string(),
        }],
    };

    let out = explain_result(&result);
    assert!(out.contains("## Diagnostics"));
    assert!(out.contains("docs/bad.md"));
    assert!(out.contains("bad markdown"));
}

#[test]
fn formatter_sanitizes_control_and_markdown_metacharacters() {
    let result = ExplainResult {
        id: "target.id".to_string(),
        documents: vec![],
        annotations: vec![],
        references: vec![Reference {
            source_id: "src\n\tid".to_string(),
            target_id: "target.id".to_string(),
            source_path: PathBuf::from("docs/so`urce.md"),
            source_start_line: 5,
            source_start_col: 4,
            source_end_line: 5,
            source_end_col: 17,
            display_text: Some("x\n# heading\n- bullet\t`code`".to_string()),
        }],
        scan_diagnostics: vec![Diagnostic {
            severity: DiagnosticSeverity::Error,
            path: PathBuf::from("docs/\u{0007}bad.md"),
            line: Some(2),
            message: "oops\n## forged".to_string(),
        }],
    };

    let out = explain_result(&result);

    assert!(!out.contains("\n## forged"));
    assert!(out.contains("\\n\\# heading"));
    assert!(out.contains("\\- bullet\\\\t\\`code\\`"));
    assert!(out.contains("\\u{7}"));
}
