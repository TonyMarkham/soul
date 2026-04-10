use error_location::ErrorLocation;
use std::{panic::Location, result::Result as StdResult};

#[derive(Debug, thiserror::Error)]
pub enum AnnotationError {
    #[error("malformed soul attribute")]
    Malformed { location: ErrorLocation },

    #[error("missing required field `id`")]
    MissingId { location: ErrorLocation },
}

impl AnnotationError {
    #[track_caller]
    pub fn malformed() -> Self {
        Self::Malformed {
            location: ErrorLocation::from(Location::caller()),
        }
    }

    #[track_caller]
    pub fn missing_id() -> Self {
        Self::MissingId {
            location: ErrorLocation::from(Location::caller()),
        }
    }
}

pub type AnnotationResult<T> = StdResult<T, AnnotationError>;
