use error_location::ErrorLocation;
use std::{panic::Location, path::PathBuf, result::Result as StdResult};

#[derive(Debug, thiserror::Error)]
pub enum IndexerError {
    #[error("root does not exist or is not a directory: {path} {location}")]
    InvalidRoot {
        path: PathBuf,
        location: ErrorLocation,
    },

    #[error("failed to walk path {path}: {source} {location}")]
    WalkEntry {
        path: PathBuf,
        location: ErrorLocation,
        #[source]
        source: std::io::Error,
    },

    #[error("{message} {location}")]
    Cli {
        message: String,
        location: ErrorLocation,
    },

    #[error("failed to read config `{path}`: {source} {location}")]
    ConfigRead {
        path: PathBuf,
        location: ErrorLocation,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to parse config `{path}`: {source} {location}")]
    ConfigParse {
        path: PathBuf,
        location: ErrorLocation,
        #[source]
        source: Box<toml::de::Error>,
    },

    #[error("index db error at `{path}`: {source} {location}")]
    IndexDb {
        path: PathBuf,
        location: ErrorLocation,
        #[source]
        source: sqlx::Error,
    },

    #[error("mcp error: {source} {location}")]
    Mcp {
        location: ErrorLocation,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    #[error("failed to load plugin `{language}` from `{path}`: {message} {location}")]
    PluginLoadFailed {
        language: String,
        path: PathBuf,
        message: String,
        location: ErrorLocation,
    },

    #[error("duplicate plugin language `{language}` {location}")]
    DuplicatePluginLanguage {
        language: String,
        location: ErrorLocation,
    },

    #[error("two plugins claim extension `{extension}`: `{first}` and `{second}` {location}")]
    DuplicatePluginExtension {
        extension: String,
        first: String,
        second: String,
        location: ErrorLocation,
    },

    #[error("malformed soul annotation: {message} {location}")]
    AnnotationParse {
        message: String,
        location: ErrorLocation,
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

    #[track_caller]
    pub fn plugin_load(language: String, path: PathBuf, source: impl std::fmt::Display) -> Self {
        Self::PluginLoadFailed {
            language,
            path,
            message: source.to_string(),
            location: ErrorLocation::from(Location::caller()),
        }
    }

    #[track_caller]
    pub fn duplicate_plugin_language(language: String) -> Self {
        Self::DuplicatePluginLanguage {
            language,
            location: ErrorLocation::from(Location::caller()),
        }
    }

    #[track_caller]
    pub fn duplicate_plugin_extension(extension: String, first: String, second: String) -> Self {
        Self::DuplicatePluginExtension {
            extension,
            first,
            second,
            location: ErrorLocation::from(Location::caller()),
        }
    }

    #[track_caller]
    pub fn annotation_parse(message: impl Into<String>) -> Self {
        Self::AnnotationParse {
            message: message.into(),
            location: ErrorLocation::from(Location::caller()),
        }
    }
}

pub type IndexerResult<T> = StdResult<T, IndexerError>;
