use std::ops::Range;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ScannedLine<'a> {
    pub(crate) line_number: usize,
    pub(crate) raw: &'a str,
    pub(crate) is_fenced_code: bool,
    pub(crate) is_indented_code: bool,
    pub(crate) visible_ranges: Vec<Range<usize>>,
}

impl<'a> ScannedLine<'a> {
    pub(crate) fn is_annotation_candidate(&self) -> bool {
        !self.is_fenced_code && !self.is_indented_code
    }
}
