use crate::model::{CodeAnnotation, Diagnostic, Document, MarkdownSource, Reference};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SemanticGraph {
    pub documents: Vec<Document>,
    pub annotations: Vec<CodeAnnotation>,
    pub references: Vec<Reference>,
    pub markdown_sources: Vec<MarkdownSource>,
    pub diagnostics: Vec<Diagnostic>,
}
