pub mod loader;

// ---------------------------------------------------------------------------------------------- //

pub use loader::PluginRegistry;

// ---------------------------------------------------------------------------------------------- //

use crate::{
    IndexerResult,
    model::{AnnotationSyntax, CodeAnnotation, Diagnostic, DiagnosticSeverity, ParseReport},
};

use soul_plugin_sdk::NormalizedAnnotation;

use abi_stable::std_types::{ROption, RResult};
use std::path::Path;

pub fn parse_annotations(
    path: &Path,
    input: &str,
    registry: &PluginRegistry,
) -> IndexerResult<ParseReport<Vec<CodeAnnotation>>> {
    let Some(parser) = path
        .extension()
        .and_then(|e| e.to_str())
        .and_then(|ext| registry.parser_for_extension(ext))
    else {
        return Ok(ParseReport {
            value: Vec::new(),
            diagnostics: Vec::new(),
        });
    };

    let mut annotations = Vec::new();
    let mut diagnostics = Vec::new();
    let syntax = AnnotationSyntax(parser.syntax().into());

    for (index, line) in input.lines().enumerate() {
        match parser.parse_line(line.into()) {
            ROption::RNone => continue,
            ROption::RSome(RResult::ROk(ann)) => {
                annotations.push(normalized_to_code_annotation(
                    ann,
                    path.to_path_buf(),
                    index + 1,
                    syntax.clone(),
                ));
            }
            ROption::RSome(RResult::RErr(e)) => {
                diagnostics.push(Diagnostic {
                    severity: DiagnosticSeverity::Error,
                    path: path.to_path_buf(),
                    line: Some(index + 1),
                    message: e.message().into(),
                });
            }
        }
    }

    Ok(ParseReport {
        value: annotations,
        diagnostics,
    })
}

fn normalized_to_code_annotation(
    annotation: NormalizedAnnotation,
    path: std::path::PathBuf,
    line: usize,
    syntax: AnnotationSyntax,
) -> CodeAnnotation {
    let metadata: serde_json::Map<String, serde_json::Value> = annotation
        .metadata
        .into_iter()
        .map(|entry| {
            let key: String = entry.0.into();
            let val_str: String = entry.1.into();
            let value = serde_json::from_str::<serde_json::Value>(&val_str)
                .unwrap_or(serde_json::Value::String(val_str));
            (key, value)
        })
        .collect();
    CodeAnnotation {
        id: annotation.id.into(),
        metadata,
        path,
        line,
        syntax,
        raw: annotation.raw.into(),
    }
}
