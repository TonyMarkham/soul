use crate::model::{CodeAnnotation, Diagnostic, Document, Reference};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExplainResult {
    pub id: String,
    pub documents: Vec<Document>,
    pub annotations: Vec<CodeAnnotation>,
    pub references: Vec<Reference>,
    pub scan_diagnostics: Vec<Diagnostic>,
}
