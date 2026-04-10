use error_location::ErrorLocation;
use std::{panic::Location, result::Result as StdResult};

#[derive(Debug, thiserror::Error)]
pub enum SoulError {
    #[error("soul attribute keys must be identifiers")]
    NonIdentifierKey { location: ErrorLocation },

    #[error("duplicate soul attribute field `{field}`")]
    DuplicateField {
        field: String,
        location: ErrorLocation,
    },

    #[error("soul attribute values must be string literals")]
    NonStringValue { location: ErrorLocation },

    #[error("soul attribute value for `{field}` must not be empty")]
    EmptyValue {
        field: String,
        location: ErrorLocation,
    },

    #[error("soul attribute requires `id = \"...\"`")]
    MissingId { location: ErrorLocation },
}

impl SoulError {
    #[track_caller]
    pub fn non_identifier_key() -> Self {
        Self::NonIdentifierKey {
            location: ErrorLocation::from(Location::caller()),
        }
    }

    #[track_caller]
    pub fn duplicate_field(field: impl Into<String>) -> Self {
        Self::DuplicateField {
            field: field.into(),
            location: ErrorLocation::from(Location::caller()),
        }
    }

    #[track_caller]
    pub fn non_string_value() -> Self {
        Self::NonStringValue {
            location: ErrorLocation::from(Location::caller()),
        }
    }

    #[track_caller]
    pub fn empty_value(field: impl Into<String>) -> Self {
        Self::EmptyValue {
            field: field.into(),
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

pub type SoulResult<T> = StdResult<T, SoulError>;
