use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ScanConfig {
    pub excluded_dirs: Vec<String>,
    pub excluded_dir_suffixes: Vec<String>,
    pub excluded_bin_except_under: Vec<String>,
    pub annotation_extensions: Vec<String>,
}
