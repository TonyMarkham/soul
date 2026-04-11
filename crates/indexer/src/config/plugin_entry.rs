use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct PluginEntry {
    pub language: String,
    pub path: PathBuf,
}
