use crate::config::ScanConfig;

use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct SoulConfig {
    pub scan: ScanConfig,
}
