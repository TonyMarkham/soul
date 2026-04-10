pub mod annotation;
pub mod error;
pub mod graph;
pub mod markdown;
pub mod model;

pub mod config;
pub mod constants;
pub mod index;
pub mod mcp;
pub mod scan;

#[cfg(test)]
pub mod tests;

pub use error::{
    IndexerError, IndexerResult,
    annotation::{AnnotationError, AnnotationResult},
};
pub use graph::{ExplainResult, explain};
pub use mcp::SoulServer;
pub use model::{CodeAnnotation, Diagnostic, DiagnosticSeverity, Document, SemanticGraph};
pub use scan::scan_repository;
