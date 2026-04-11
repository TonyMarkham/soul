use rmcp::schemars;
use serde::Deserialize;

#[derive(Deserialize, schemars::JsonSchema)]
pub struct AnnotationSyntaxParams {
    /// A file path (e.g. `crates/foo/src/bar.rs`) or a bare extension (e.g. `.rs` or `rs`).
    pub target: String,
}
