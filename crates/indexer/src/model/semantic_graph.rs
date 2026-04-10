use crate::model::{code_annotation::CodeAnnotation, diagnostic::Diagnostic, document::Document};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SemanticGraph {
    pub documents: Vec<Document>,
    pub annotations: Vec<CodeAnnotation>,
    pub diagnostics: Vec<Diagnostic>,
}
