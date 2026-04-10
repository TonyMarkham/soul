use serde_json::{Map, Value};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NormalizedAnnotation {
    pub(crate) id: String,
    pub(crate) role: Option<String>,
    pub(crate) metadata: Map<String, Value>,
    pub(crate) raw: String,
}
