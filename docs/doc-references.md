<!-- soul id="indexer.markdown-annotations" -->

# Markdown HTML-Comment Annotations

Let soul annotations appear in regular Markdown files (not soul frontmatter documents) so that soul documents can reference content in arbitrary `.md` files.

## Format

```html
<!-- soul id="dot.separated.id" key="value" key2="value2" -->
```

Space-separated `key="value"` pairs, HTML-attribute style. The annotation must occupy the entire trimmed line (consistent with existing plugins). Single-line only. Backslash escapes supported in string values (`\"`, `\\`, `\n`, `\r`, `\t`).

## Why not a plugin

`.md` files are already handled specially in the scanner (`classify_path` → `CandidateKind::Document`). A plugin claiming `.md` would conflict with YAML-frontmatter document parsing. Instead, annotation extraction is built directly into the existing markdown processing path.

## Implementation plan

### 1. New file: `crates/indexer/src/markdown/annotations.rs`

Also add a new variant to `IndexerError` in `crates/indexer/src/error/mod.rs`:

```rust
// Add after DuplicatePluginExtension, before the closing brace of the enum:

    #[error("malformed soul annotation: {message}")]
    AnnotationParse {
        message: String,
        location: ErrorLocation,
    },
```

And add its constructor in the `impl IndexerError` block:

```rust
    #[track_caller]
    pub fn annotation_parse(message: impl Into<String>) -> Self {
        Self::AnnotationParse {
            message: message.into(),
            location: ErrorLocation::from(Location::caller()),
        }
    }
```

Now the new file:

```rust
use crate::{
    IndexerError, IndexerResult,
    model::{AnnotationSyntax, CodeAnnotation, Diagnostic, DiagnosticSeverity, ParseReport},
};

use std::path::Path;

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
                    message:
                        "soul annotation missing required `id` field".to_string(),
                });
                continue;
            }
        };

        // Remaining key-value pairs become metadata
        let metadata: serde_json::Map<String, serde_json::Value> = fields
            .into_iter()
            .filter(|(k, _)| k != "id")
            .collect();

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
) -> Result<serde_json::Map<String, serde_json::Value>, IndexerError> {
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
            return Err(IndexerError::annotation_parse(
                format!("expected quoted value for key `{key}`"),
            ));
        }
        let value = extract_quoted_string(&mut remaining)?;

        if fields.contains_key(&key) {
            return Err(IndexerError::annotation_parse(
                format!("duplicate key `{key}`"),
            ));
        }

        fields.insert(key, serde_json::Value::String(value));
    }

    Ok(fields)
}

/// Extract a backslash-escaped double-quoted string from the front of `source`.
/// On success, `source` is advanced past the closing quote.
/// Handles `\"`, `\\`, `\n`, `\r`, `\t`.
fn extract_quoted_string(source: &mut &str) -> Result<String, IndexerError> {
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
```

### 2. Modify `crates/indexer/src/markdown/mod.rs`

Add the module declaration and a re-export of the extractor. Place them after the existing module declarations but before the separator comment:

```rust
pub(crate) mod annotations;
pub(crate) use annotations::extract_annotations;
```

### 3. Modify `crates/indexer/src/scan/mod.rs`

Add `extract_annotations` to the `markdown` import in the `use crate::{}` block. Change `markdown::parse_markdown` to `markdown::{parse_markdown, extract_annotations}`. Then, inside the `CandidateKind::Document` match arm, call `extract_annotations` after `parse_markdown`:

```rust
CandidateKind::Document => {
    let doc_report = parse_markdown(&candidate.display_path, &contents)?;
    if let Some(document) = doc_report.value {
        graph.documents.push(document);
    }
    graph.diagnostics.extend(doc_report.diagnostics);

    // Also scan for HTML-comment annotations
    let ann_report = extract_annotations(&candidate.display_path, &contents)?;
    graph.annotations.extend(ann_report.value);
    graph.diagnostics.extend(ann_report.diagnostics);
}
```

### 4. Modify `crates/indexer/src/mcp/mod.rs`

In `soul_annotation_syntax_impl`, add a hardcoded fallback for `ext == "md"` before the plugin registry lookup:

```rust
async fn soul_annotation_syntax_impl(
    &self,
    p: AnnotationSyntaxParams,
) -> IndexerResult<CallToolResult> {
    let ext = {
        let t = p.target.trim_start_matches('.');
        std::path::Path::new(t)
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| t.to_string())
    };
    // --- start new code ---
    if ext == "md" {
        return Ok(CallToolResult::success(vec![Content::text(
            r#"Markdown soul annotation syntax

  Template:
    <!-- soul id="<id>" -->

  Placement: on its own line within any Markdown (.md) file.

  Example:
    <!-- soul id="interaction.checkout.create-order" layer="backend" -->

  Rules:
  - `id` is required; all other keys are optional metadata
  - Values must be quoted strings: key="value" (space-separated)
  - The annotation must occupy the entire trimmed line
  - Single-line only
  - Backslash escapes supported: \", \\, \n, \r, \t"#
                .to_string(),
        )]));
    }
    // --- end new code ---
    match self.registry.parser_for_extension(&ext) {
        Some(parser) => Ok(CallToolResult::success(vec![Content::text(
            parser.syntax_guidance().to_string(),
        )])),
        None => Err(IndexerError::cli(format!(
            "no plugin registered for extension `.{ext}`"
        ))),
    }
}
```

## Files touched

| File | Action |
|------|--------|
| `crates/indexer/src/error/mod.rs` | **Edit** — add `AnnotationParse` variant and constructor to `IndexerError` |
| `crates/indexer/src/markdown/annotations.rs` | **Create** — annotation line parser |
| `crates/indexer/src/markdown/mod.rs` | **Edit** — declare module, re-export |
| `crates/indexer/src/scan/mod.rs` | **Edit** — call extractor in Document arm |
| `crates/indexer/src/mcp/mod.rs` | **Edit** — handle `.md` in syntax guidance |

**Implementation order:** 0 (error variant) → 1 (new file) → 2 (module declaration) → 3 (call site) → 4 (MCP guidance). The error variant must be added first because step 1's code references `IndexerError::annotation_parse`. Step 2 (module declaration) must precede step 3 (call site) to satisfy Rust compilation order.

No new crates, no new dependencies, no config changes.
