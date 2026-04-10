use rmcp::schemars;
use serde::Deserialize;

#[derive(Deserialize, schemars::JsonSchema)]
pub struct ExplainParams {
    /// The Soul ID to explain. A dot-separated semantic identifier, e.g. `interaction.checkout.create-order`.
    /// Use soul_list_documents to discover valid IDs.
    pub id: String,
}
