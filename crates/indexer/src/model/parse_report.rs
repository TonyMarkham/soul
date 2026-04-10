use crate::model::diagnostic::Diagnostic;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseReport<T> {
    pub value: T,
    pub diagnostics: Vec<Diagnostic>,
}
