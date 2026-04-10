pub mod annotation;

use error_location::ErrorLocation;
use std::{panic::Location, path::PathBuf, result::Result as StdResult};

#[derive(Debug, thiserror::Error)]
pub enum IndexerError {
    #[error("root does not exist or is not a directory: {path}")]
    InvalidRoot {
        path: PathBuf,
        location: ErrorLocation,
    },

    #[error("failed to walk path {path}: {source}")]
    WalkEntry {
        path: PathBuf,
        location: ErrorLocation,
        #[source]
        source: std::io::Error,
    },

    #[error("{message}")]
    Cli {
        message: String,
        location: ErrorLocation,
    },

    #[error("{location} failed to read config `{path}`: {source}")]
    ConfigRead {
        path: PathBuf,
        location: ErrorLocation,
        #[source]
        source: std::io::Error,
    },

    #[error("{location} failed to parse config `{path}`: {source}")]
    ConfigParse {
        path: PathBuf,
        location: ErrorLocation,
        #[source]
        source: Box<toml::de::Error>,
    },

    #[error("{location} index db error at `{path}`: {source}")]
    IndexDb {
        path: PathBuf,
        location: ErrorLocation,
        #[source]
        source: sqlx::Error,
    },

    #[error("{location} mcp error: {source}")]
    Mcp {
        location: ErrorLocation,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },
}

impl IndexerError {
    #[track_caller]
    pub fn invalid_root(path: PathBuf) -> Self {
        Self::InvalidRoot {
            path,
            location: ErrorLocation::from(Location::caller()),
        }
    }

    #[track_caller]
    pub fn walk_entry(path: PathBuf, source: std::io::Error) -> Self {
        Self::WalkEntry {
            path,
            location: ErrorLocation::from(Location::caller()),
            source,
        }
    }

    #[track_caller]
    pub fn cli(message: impl Into<String>) -> Self {
        Self::Cli {
            message: message.into(),
            location: ErrorLocation::from(Location::caller()),
        }
    }

    #[track_caller]
    pub fn config_read(path: PathBuf, source: std::io::Error) -> Self {
        Self::ConfigRead {
            path,
            location: ErrorLocation::from(Location::caller()),
            source,
        }
    }

    #[track_caller]
    pub fn config_parse(path: PathBuf, source: toml::de::Error) -> Self {
        Self::ConfigParse {
            path,
            location: ErrorLocation::from(Location::caller()),
            source: Box::new(source),
        }
    }

    #[track_caller]
    pub fn index_db(path: PathBuf, source: sqlx::Error) -> Self {
        Self::IndexDb {
            path,
            location: ErrorLocation::from(Location::caller()),
            source,
        }
    }

    #[track_caller]
    pub fn mcp(source: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self::Mcp {
            location: ErrorLocation::from(Location::caller()),
            source: Box::new(source),
        }
    }
}

pub type IndexerResult<T> = StdResult<T, IndexerError>;
