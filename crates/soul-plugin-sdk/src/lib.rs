pub mod error;
pub mod helpers;
pub mod normalized_annotation;

// abi_stable_derive 0.11.3 generates `impl` blocks inside a named const — upstream macro issue.
#[allow(non_local_definitions)]
pub mod parser;
pub mod root_module;

// ---------------------------------------------------------------------------------------------- //

pub use error::{AnnotationError, AnnotationResult};
pub use normalized_annotation::NormalizedAnnotation;
pub use parser::{AnnotationParser, AnnotationParser_TO};
pub use root_module::{SoulPlugin, SoulPluginRef};

// ---------------------------------------------------------------------------------------------- //
