pub mod scan_config;
pub mod soul_config;

// ---------------------------------------------------------------------------------------------- //

pub use scan_config::ScanConfig;
pub use soul_config::SoulConfig;

// ---------------------------------------------------------------------------------------------- //

use crate::{IndexerError, IndexerResult, constants};

use std::path::Path;

pub fn load_config(root: &Path) -> IndexerResult<SoulConfig> {
    let path = root.join(constants::SOUL_DIR).join(constants::CONFIG_FILE);
    let contents =
        std::fs::read_to_string(&path).map_err(|e| IndexerError::config_read(path.clone(), e))?;
    toml::from_str(&contents).map_err(|e| IndexerError::config_parse(path.clone(), e))
}
