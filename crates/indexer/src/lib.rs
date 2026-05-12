pub mod annotation;
pub mod config;
pub mod constants;
pub mod error;
pub mod graph;
pub mod index;
pub mod markdown;
pub mod mcp;
pub mod model;
pub mod scan;

// ---------------------------------------------------------------------------------------------- //

#[cfg(test)]
pub mod tests;

// ---------------------------------------------------------------------------------------------- //

pub use annotation::loader::PluginRegistry;
pub use error::{IndexerError, IndexerResult};
pub use graph::{ExplainResult, explain};
pub use mcp::SoulServer;
pub use model::{
    CodeAnnotation, Diagnostic, DiagnosticSeverity, Document, MarkdownSource, Reference,
    SemanticGraph, WikiLinkToken,
};
pub use scan::scan_repository;
