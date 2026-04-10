use crate::{AnnotationResult, annotation::NormalizedAnnotation, model::AnnotationSyntax};

pub(crate) trait Parser {
    fn extension(&self) -> &'static str;
    fn syntax(&self) -> AnnotationSyntax;
    fn parse_line(&self, line: &str) -> Option<AnnotationResult<NormalizedAnnotation>>;
}
