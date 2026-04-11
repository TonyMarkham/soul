use crate::model::annotation_syntax::AnnotationSyntax;

use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeAnnotation {
    pub id: String,
    pub metadata: serde_json::Map<String, serde_json::Value>,
    pub path: PathBuf,
    pub line: usize,
    pub syntax: AnnotationSyntax,
    pub raw: String,
}
