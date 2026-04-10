use crate::model::diagnostic_severity::DiagnosticSeverity;

use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub severity: DiagnosticSeverity,
    pub path: PathBuf,
    pub line: Option<usize>,
    pub message: String,
}
