#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WikiLinkToken {
    pub target_id: String,
    pub start_line: usize,
    pub start_col: usize,
    pub end_line: usize,
    pub end_col: usize,
    pub display_text: Option<String>,
}
