use crate::{
    IndexerError, IndexerResult,
    model::{AnnotationSyntax, CodeAnnotation, Diagnostic, DiagnosticSeverity, ParseReport},
};

use soul_attributes::soul;
use std::path::Path;

#[soul(id = "indexer.markdown-annotations")]
pub(crate) fn extract_annotations(
    path: &Path,
    input: &str,
) -> IndexerResult<ParseReport<Vec<CodeAnnotation>>> {
    let mut annotations = Vec::new();
    let mut diagnostics = Vec::new();
    let syntax = AnnotationSyntax("markdown-comment".to_string());
    for (index, line) in input.lines().enumerate() {
        let line_number = index + 1;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        // Must start with <!-- and contain soul
        if !trimmed.starts_with("<!--") || !trimmed.contains("soul") {
            continue;
        }
        // Must end with -->
        if !trimmed.ends_with("-->") {
            diagnostics.push(Diagnostic {
                severity: DiagnosticSeverity::Error,
                path: path.to_path_buf(),
                line: Some(line_number),
                message: "malformed soul annotation: HTML comment not closed".to_string(),
            });
            continue;
        }
        // Extract content between <!-- and -->
        let inner = &trimmed["<!--".len()..trimmed.len() - "-->".len()];
        let inner = inner.trim();
        // Must start with the keyword "soul" (as a word)
        let Some(payload) = inner.strip_prefix("soul") else {
            continue;
        };
        // require word boundary after keyword
        if !payload.is_empty() && !payload.starts_with(|c: char| c.is_ascii_whitespace()) {
            continue;
        }
        let payload = payload.trim();
        // Parse space-separated key="value" segments
        let fields = match parse_html_attr_fields(payload) {
            Ok(fields) => fields,
            Err(err) => {
                diagnostics.push(Diagnostic {
                    severity: DiagnosticSeverity::Error,
                    path: path.to_path_buf(),
                    line: Some(line_number),
                    message: format!("{err}"),
                });
                continue;
            }
        };
        // Require `id` to be present and non-empty
        let id = match fields.get("id").and_then(|v| v.as_str()) {
            Some(s) if !s.trim().is_empty() => s.trim().to_string(),
            _ => {
                diagnostics.push(Diagnostic {
                    severity: DiagnosticSeverity::Error,
                    path: path.to_path_buf(),
                    line: Some(line_number),
                    message: "soul annotation missing required `id` field".to_string(),
                });
                continue;
            }
        };
        // Remaining key-value pairs become metadata
        let metadata: serde_json::Map<String, serde_json::Value> =
            fields.into_iter().filter(|(k, _)| k != "id").collect();
        annotations.push(CodeAnnotation {
            id,
            metadata,
            path: path.to_path_buf(),
            line: line_number,
            syntax: syntax.clone(),
            raw: trimmed.to_string(),
        });
    }
    Ok(ParseReport {
        value: annotations,
        diagnostics,
    })
}
/// Parse space-separated `key="value"` segments from an HTML attribute payload.
fn parse_html_attr_fields(
    mut remaining: &str,
) -> IndexerResult<serde_json::Map<String, serde_json::Value>> {
    let mut fields = serde_json::Map::new();
    loop {
        remaining = remaining.trim_start();
        if remaining.is_empty() {
            break;
        }
        // Read key up to '='
        let eq_pos = remaining
            .find('=')
            .ok_or_else(|| IndexerError::annotation_parse("expected key=value pair"))?;
        let key = remaining[..eq_pos].trim_end().to_string();
        if key.is_empty() {
            return Err(IndexerError::annotation_parse("empty key in annotation"));
        }
        remaining = remaining[(eq_pos + 1)..].trim_start();
        // Value must be a quoted string
        if !remaining.starts_with('"') {
            return Err(IndexerError::annotation_parse(format!(
                "expected quoted value for key `{key}`"
            )));
        }
        let value = extract_quoted_string(&mut remaining)?;
        if fields.contains_key(&key) {
            return Err(IndexerError::annotation_parse(format!(
                "duplicate key `{key}`"
            )));
        }
        fields.insert(key, serde_json::Value::String(value));
    }
    Ok(fields)
}

/// Extract a backslash-escaped double-quoted string from the front of `source`.
/// On success, `source` is advanced past the closing quote.
/// Handles `\"`, `\\`, `\n`, `\r`, `\t`.
fn extract_quoted_string(source: &mut &str) -> IndexerResult<String> {
    // Copy the reference locally to avoid borrow conflicts when advancing source below.
    let s = *source;
    if !s.starts_with('"') {
        return Err(IndexerError::annotation_parse("expected opening quote"));
    }
    let mut value = String::new();
    let mut escaped = false;
    for (byte_offset, c) in s[1..].char_indices() {
        if escaped {
            match c {
                '"' => value.push('"'),
                '\\' => value.push('\\'),
                'n' => value.push('\n'),
                'r' => value.push('\r'),
                't' => value.push('\t'),
                other => value.push(other),
            }
            escaped = false;
        } else if c == '\\' {
            escaped = true;
        } else if c == '"' {
            // byte_offset is relative to s[1..]; add 1 + c.len_utf8() for absolute position
            let abs = 1 + byte_offset + c.len_utf8();
            *source = &s[abs..];
            return Ok(value);
        } else {
            value.push(c);
        }
    }
    Err(IndexerError::annotation_parse("unterminated quoted string"))
}
