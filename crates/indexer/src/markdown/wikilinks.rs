use crate::{
    IndexerError, IndexerResult,
    markdown::{
        line_scanner::LineScanner,
        wikilink_validation::{
            is_valid_reference_display_text, is_valid_reference_target_id,
            normalize_reference_display_text,
        },
    },
    model::{Diagnostic, DiagnosticSeverity, ParseReport, WikiLinkToken},
};

use std::path::Path;

pub(crate) fn extract_wikilinks(
    path: &Path,
    input: &str,
    start_line: usize,
) -> ParseReport<Vec<WikiLinkToken>> {
    let mut scanner = LineScanner::new();
    let mut tokens = Vec::new();
    let mut diagnostics = Vec::new();

    for (offset, line) in input.lines().enumerate() {
        let line_number = start_line + offset;
        let scanned = scanner.scan_line(line_number, line);

        if scanned.visible_ranges.is_empty() {
            continue;
        }

        for range in scanned.visible_ranges {
            let mut index = range.start;
            let mut stack: Vec<(usize, bool)> = Vec::new();

            while index < range.end {
                if index + 1 < range.end
                    && line.as_bytes()[index] == b'['
                    && line.as_bytes()[index + 1] == b'['
                {
                    if is_escaped_open(line, index) {
                        index += 2;
                        continue;
                    }

                    if let Some(last) = stack.last_mut() {
                        last.1 = true;
                    }
                    stack.push((index, false));
                    index += 2;
                    continue;
                }

                if index + 1 < range.end
                    && line.as_bytes()[index] == b']'
                    && line.as_bytes()[index + 1] == b']'
                {
                    if let Some((open_index, nested)) = stack.pop()
                        && !nested
                    {
                        let content = &line[open_index + 2..index];
                        match parse_wikilink_content(content) {
                            Ok((target_id, display_text)) => tokens.push(WikiLinkToken {
                                target_id,
                                start_line: line_number,
                                start_col: utf16_col_at_byte_index(line, open_index),
                                end_line: line_number,
                                end_col: utf16_col_at_byte_index(line, index + 2),
                                display_text,
                            }),
                            Err(error) => diagnostics.push(Diagnostic {
                                severity: DiagnosticSeverity::Error,
                                path: path.to_path_buf(),
                                line: Some(line_number),
                                message: match error {
                                    IndexerError::WikiLinkParse { message, .. } => message,
                                    _ => unreachable!("wikilink parser only returns WikiLinkParse"),
                                },
                            }),
                        }
                    }
                    index += 2;
                    continue;
                }

                let ch = line[index..]
                    .chars()
                    .next()
                    .expect("index on char boundary");
                index += ch.len_utf8();
            }

            for _ in stack {
                diagnostics.push(Diagnostic {
                    severity: DiagnosticSeverity::Error,
                    path: path.to_path_buf(),
                    line: Some(line_number),
                    message: "wiki link crosses a line boundary".to_string(),
                });
            }
        }
    }

    tokens.sort_by(|a, b| {
        a.start_line
            .cmp(&b.start_line)
            .then(a.start_col.cmp(&b.start_col))
            .then(a.target_id.cmp(&b.target_id))
    });

    ParseReport {
        value: tokens,
        diagnostics,
    }
}

fn parse_wikilink_content(content: &str) -> IndexerResult<(String, Option<String>)> {
    let (target_raw, display_raw) = match content.split_once('|') {
        Some((target, display)) => (target, Some(display)),
        None => (content, None),
    };

    let target = target_raw.trim();
    if !is_valid_reference_target_id(target) {
        return Err(IndexerError::wikilink_parse("invalid wiki link target"));
    }

    let display_text = if let Some(display_raw) = display_raw {
        if !is_valid_reference_display_text(display_raw) {
            return Err(IndexerError::wikilink_parse(
                "wiki link display text is too long",
            ));
        }
        normalize_reference_display_text(display_raw)
    } else {
        None
    };

    Ok((target.to_string(), display_text))
}

fn is_escaped_open(line: &str, index: usize) -> bool {
    let bytes = line.as_bytes();
    let mut count = 0;
    let mut cursor = index;

    while cursor > 0 && bytes[cursor - 1] == b'\\' {
        count += 1;
        cursor -= 1;
    }

    count % 2 == 1
}

fn utf16_col_at_byte_index(line: &str, byte_index: usize) -> usize {
    line[..byte_index].encode_utf16().count()
}
