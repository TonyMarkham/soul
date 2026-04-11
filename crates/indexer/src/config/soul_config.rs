use crate::config::{PluginEntry, ScanConfig};

use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct SoulConfig {
    pub scan: ScanConfig,
    /// `[[plugins]]` entries from `soul.toml`. `#[serde(default)]` ensures existing
    /// configs without a `[[plugins]]` section deserialise to an empty Vec rather than failing.
    #[serde(default)]
    pub plugins: Vec<PluginEntry>,
}
