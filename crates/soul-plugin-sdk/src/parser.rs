use crate::{AnnotationError, NormalizedAnnotation};

use abi_stable::{
    sabi_trait,
    std_types::{ROption, RResult, RStr, RString},
};

#[sabi_trait]
pub trait AnnotationParser: Send + Sync {
    /// File extension this plugin handles (e.g. "rs", "cs"). No dot.
    fn extension(&self) -> RString;

    /// Short stable identifier for the annotation syntax (e.g. "rust-attribute").
    /// Stored in the SQLite `syntax` column.
    fn syntax(&self) -> RString;

    /// Parse a single source line. Returns `RNone` if the line isn't an annotation,
    /// `RSome(RErr)` on malformed input, `RSome(ROk)` on success.
    fn parse_line(&self, line: RStr<'_>)
    -> ROption<RResult<NormalizedAnnotation, AnnotationError>>;

    /// Human-readable guidance returned by the `soul_annotation_syntax` MCP tool.
    fn syntax_guidance(&self) -> RString;
}
