use crate::markdown::{fence_state::FenceState, scanned_line::ScannedLine};

use std::ops::Range;

#[derive(Debug, Default)]
pub(crate) struct LineScanner {
    fence: Option<FenceState>,
    in_html_comment: bool,
}

impl LineScanner {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn scan_line<'a>(&mut self, line_number: usize, line: &'a str) -> ScannedLine<'a> {
        if let Some(fence) = self.fence.clone() {
            if closes_fence(line, fence.marker, fence.len) {
                self.fence = None;
            }
            return ScannedLine {
                line_number,
                raw: line,
                is_fenced_code: true,
                is_indented_code: false,
                visible_ranges: Vec::new(),
            };
        }

        if let Some(fence) = opens_fence(line) {
            self.fence = Some(fence);
            return ScannedLine {
                line_number,
                raw: line,
                is_fenced_code: true,
                is_indented_code: false,
                visible_ranges: Vec::new(),
            };
        }

        let is_indented_code = line.starts_with('\t') || line.starts_with("    ");
        let visible_ranges = if is_indented_code {
            Vec::new()
        } else {
            visible_non_comment_non_inline_code_ranges(line, &mut self.in_html_comment)
        };

        ScannedLine {
            line_number,
            raw: line,
            is_fenced_code: false,
            is_indented_code,
            visible_ranges,
        }
    }
}

fn opens_fence(line: &str) -> Option<FenceState> {
    let trimmed = line.trim_start();
    let marker = trimmed.chars().next()?;
    if marker != '`' && marker != '~' {
        return None;
    }
    let len = trimmed.chars().take_while(|ch| *ch == marker).count();
    (len >= 3).then_some(FenceState { marker, len })
}

fn closes_fence(line: &str, marker: char, opening_len: usize) -> bool {
    let trimmed = line.trim_start();
    let len = trimmed.chars().take_while(|ch| *ch == marker).count();
    len >= opening_len && trimmed.chars().skip(len).all(|ch| ch.is_whitespace())
}

fn visible_non_comment_non_inline_code_ranges(
    line: &str,
    in_html_comment: &mut bool,
) -> Vec<Range<usize>> {
    let mut ranges = Vec::new();
    let mut visible_start: Option<usize> = None;
    let mut index = 0;

    while index < line.len() {
        if *in_html_comment {
            if let Some(end) = line[index..].find("-->") {
                index += end + "-->".len();
                *in_html_comment = false;
            } else {
                return ranges;
            }
            continue;
        }

        if line[index..].starts_with("<!--") {
            if let Some(start) = visible_start.take() {
                ranges.push(start..index);
            }
            if let Some(end) = line[index + "<!--".len()..].find("-->") {
                index += "<!--".len() + end + "-->".len();
            } else {
                *in_html_comment = true;
                return ranges;
            }
            continue;
        }

        if line[index..].starts_with('`') {
            let tick_len = line[index..].chars().take_while(|ch| *ch == '`').count();
            let close = find_matching_backticks(line, index + tick_len, tick_len);
            if let Some(close) = close {
                if let Some(start) = visible_start.take() {
                    ranges.push(start..index);
                }
                index = close;
            } else {
                if visible_start.is_none() {
                    visible_start = Some(index);
                }
                index += tick_len;
            }
            continue;
        }

        if visible_start.is_none() {
            visible_start = Some(index);
        }
        let ch = line[index..]
            .chars()
            .next()
            .expect("index on char boundary");
        index += ch.len_utf8();
    }

    if let Some(start) = visible_start {
        ranges.push(start..line.len());
    }
    ranges
}

fn find_matching_backticks(line: &str, mut index: usize, tick_len: usize) -> Option<usize> {
    while index < line.len() {
        if line[index..].starts_with('`') {
            let len = line[index..].chars().take_while(|ch| *ch == '`').count();
            if len == tick_len {
                return Some(index + len);
            }
            index += len;
        } else {
            let ch = line[index..].chars().next()?;
            index += ch.len_utf8();
        }
    }
    None
}
