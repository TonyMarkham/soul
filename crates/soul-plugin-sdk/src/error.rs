use abi_stable::{StableAbi, std_types::RString};
use std::result::Result as StdResult;

#[repr(u8)]
#[derive(StableAbi, Debug, Clone)]
pub enum AnnotationError {
    Malformed,
    MissingId,
}

impl AnnotationError {
    pub fn malformed() -> Self {
        Self::Malformed
    }

    pub fn missing_id() -> Self {
        Self::MissingId
    }

    pub fn message(&self) -> RString {
        match self {
            Self::Malformed => "malformed soul attribute".into(),
            Self::MissingId => "missing required field `id`".into(),
        }
    }
}

pub type AnnotationResult<T> = StdResult<T, AnnotationError>;
