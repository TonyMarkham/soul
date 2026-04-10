use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Document {
    pub id: String,
    pub kind: String,
    pub title: Option<String>,
    pub path: PathBuf,
}
