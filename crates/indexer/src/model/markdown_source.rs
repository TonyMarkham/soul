use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkdownSource {
    pub source_id: String,
    pub source_path: PathBuf,
}
