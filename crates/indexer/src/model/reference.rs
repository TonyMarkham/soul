use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reference {
    pub source_id: String,
    pub target_id: String,
    pub source_path: PathBuf,
    pub source_start_line: usize,
    pub source_start_col: usize,
    pub source_end_line: usize,
    pub source_end_col: usize,
    pub display_text: Option<String>,
}
