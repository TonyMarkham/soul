pub mod explain_result;

// ---------------------------------------------------------------------------------------------- //

pub use explain_result::ExplainResult;

// ---------------------------------------------------------------------------------------------- //

use crate::model::SemanticGraph;

use soul_attributes::soul;

#[soul(id = "indexer.explain", role = "implementation")]
pub fn explain(graph: &SemanticGraph, id: &str) -> ExplainResult {
    let documents = graph
        .documents
        .iter()
        .filter(|document| document.id == id)
        .cloned()
        .collect();

    let annotations = graph
        .annotations
        .iter()
        .filter(|annotation| annotation.id == id)
        .cloned()
        .collect();

    ExplainResult {
        id: id.to_string(),
        documents,
        annotations,
        scan_diagnostics: graph.diagnostics.clone(),
    }
}
