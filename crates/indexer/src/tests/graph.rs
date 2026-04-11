use std::path::PathBuf;

use crate::{
    graph::explain,
    model::{
        AnnotationSyntax, CodeAnnotation, Diagnostic, DiagnosticSeverity, Document, SemanticGraph,
    },
};

#[test]
fn returns_matches_and_preserves_global_scan_diagnostics() {
    let graph = SemanticGraph {
        documents: vec![Document {
            id: "interaction.checkout.create-order".to_string(),
            kind: "interaction".to_string(),
            title: Some("Create order".to_string()),
            path: PathBuf::from(".docs/interactions/checkout.md"),
        }],
        annotations: vec![CodeAnnotation {
            id: "interaction.checkout.create-order".to_string(),
            metadata: serde_json::Map::new(),
            path: PathBuf::from("fixtures/backend.rs"),
            line: 2,
            syntax: AnnotationSyntax("rust-attribute".to_string()),
            raw: r#"#[soul(id = "interaction.checkout.create-order")]"#.to_string(),
        }],
        diagnostics: vec![Diagnostic {
            severity: DiagnosticSeverity::Error,
            path: PathBuf::from("fixtures/bad.rs"),
            line: Some(1),
            message: "malformed soul attribute for interaction.checkout.create-order".to_string(),
        }],
    };

    let result = explain(&graph, "interaction.checkout.create-order");

    assert_eq!(result.id, "interaction.checkout.create-order");
    assert_eq!(result.documents.len(), 1);
    assert_eq!(result.annotations.len(), 1);
    assert_eq!(result.scan_diagnostics.len(), 1);
}

#[test]
fn no_match_still_returns_global_scan_diagnostics() {
    let graph = SemanticGraph {
        diagnostics: vec![Diagnostic {
            severity: DiagnosticSeverity::Error,
            path: PathBuf::from("fixtures/bad.rs"),
            line: Some(1),
            message: "malformed soul attribute".to_string(),
        }],
        ..SemanticGraph::default()
    };

    let result = explain(&graph, "interaction.checkout.missing");

    assert!(result.documents.is_empty());
    assert!(result.annotations.is_empty());
    assert_eq!(result.scan_diagnostics.len(), 1);
}
