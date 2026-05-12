# Wikilinks — Cross-Doc Soul ID References

## Goal

Add wiki link syntax to Soul Markdown documents so one Soul ID can reference another Soul ID from Markdown text, and so the existing indexer, explain output, MCP output, CLI output, and LSP navigation can all see that relationship.

Supported syntax:

- `[[soul.id]]`
- `[[soul.id|display text]]`

The implementation must extend the repo's current flow. Do not create a second scanner, a second graph, a second index reader, or an LSP-only parser. The existing path is:

1. Markdown/source files are discovered by `scan_repository` in `crates/indexer/src/scan/mod.rs`.
2. Markdown frontmatter is parsed by `parse_markdown` in `crates/indexer/src/markdown/mod.rs`.
3. Markdown HTML-comment annotations are extracted by `extract_annotations` in `crates/indexer/src/markdown/annotations.rs`.
4. Scan results are carried by `SemanticGraph` in `crates/indexer/src/model/semantic_graph.rs`.
5. `write_index`, `load_graph`, and `explain_from_index` in `crates/indexer/src/index/mod.rs` persist and reload graph data.
6. `explain` in `crates/indexer/src/graph/mod.rs` builds `ExplainResult` for live scans.
7. MCP and CLI format that same `ExplainResult`.
8. `crates/soul-lsp/src/server.rs` reads the indexed `SemanticGraph` and serves hover, definition, and references.

This feature extends those surfaces only.

## Existing repo surface that must be preserved

- `SemanticGraph` is the scan/index/explain carrier. Add fields to it; do not create a separate backlink graph.
- Parser APIs use `ParseReport<T>`. Wiki-link parse problems are recoverable and become `Diagnostic` rows.
- Fatal filesystem, config, plugin, DB, and corrupt-index operations return `IndexerResult<T>` / `IndexerError`.
- Stored paths are repo-relative display paths. New reference/source rows must follow `Document.path` and `CodeAnnotation.path`.
- Markdown internals stay crate-private unless another crate requires a public API. The LSP is a separate crate and requires only a small shared wiki-link token lookup API so it does not duplicate parser logic.
- Existing markdown document extraction and existing markdown HTML-comment annotation extraction must continue even when wiki links in that file are ignored or invalid.
- `soul_list_gaps` remains document/annotation based in the first pass. Broken wiki-link target reporting is out of scope.

## Implementation order

1. Model additions and `SemanticGraph` fields.
2. Frontmatter body coordinates, shared markdown visibility/token parsing, and wiki-link extraction.
   The subsection numbering in section 2 is thematic, not strictly linear: complete 2.3, 2.4, 2.5, and 2.7 before wiring the 2.2 payload or the 2.6 public helper, because those two subsections depend on the shared wiki-link token type and extractor.
3. Update `parse_markdown` to return the new markdown parse payload while preserving existing frontmatter behavior, after sections 2.3 and 2.7 exist; in the same compile checkpoint update the `CandidateKind::Document` caller in `scan_repository` from `report.value: Option<Document>` to the new payload shape.
4. Complete scan integration to build `Reference` rows without dropping existing documents or annotations.
5. Add DB migrations, write/load support, and corrupt-index validation.
6. Extend live and indexed explain results.
7. Extend MCP and CLI formatting.
8. Extend LSP using the shared wiki-link parser API, not an LSP-only parser.
9. Add and update tests.

Each step must compile before moving on to the next step.

## 1. Model additions

### 1.1 `Reference`

Create `crates/indexer/src/model/reference.rs`:

```rust
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reference {
    pub source_id: String,
    pub target_id: String,
    pub source_path: PathBuf,
    pub source_start_line: usize,
    pub source_start_col: usize,
    pub source_end_line: usize,
    pub source_end_col: usize,
    pub display_text: Option<String>,
}
```

Contract:

- `source_id` is the Soul ID of the Markdown source that contains the wiki link.
- `target_id` is the ID inside `[[...]]` before the optional display separator.
- `source_path` is repo-relative, matching the current display-path convention.
- Lines are 1-based.
- Columns are 0-based UTF-16 code units, matching LSP `Position.character`.
- Spans are half-open: start inclusive, end exclusive.
- First pass supports single-line wiki links only. Cross-line `[[...]]` sequences are diagnostics and must not produce `Reference` rows.

### 1.2 `MarkdownSource`

Create `crates/indexer/src/model/markdown_source.rs`:

```rust
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkdownSource {
    pub source_id: String,
    pub source_path: PathBuf,
}
```

This records accepted frontmatter-less Markdown files that contain exactly one unique Markdown HTML-comment Soul annotation ID and therefore may be a wiki-link source. The file may contain multiple real Markdown HTML-comment Soul annotations only when all of them use that same ID. Frontmatter-backed documents do not need `MarkdownSource` rows because their source identity is already represented by `Document`.
`source_path` follows the same repo-relative display-path convention as other stored model paths.

### 1.3 `WikiLinkToken`

Create `crates/indexer/src/model/wiki_link_token.rs`:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WikiLinkToken {
    pub target_id: String,
    pub start_line: usize,
    pub start_col: usize,
    pub end_line: usize,
    pub end_col: usize,
    pub display_text: Option<String>,
}
```

This is intentionally separate from `Reference` because a parsed token does not know `source_id` and does not own a path. `scan_repository` turns accepted tokens into `Reference` rows after source-ID validation. `soul-lsp` uses the same token type through the shared parser API to avoid a second parser.

### 1.4 Model exports

Update `crates/indexer/src/model/mod.rs`:

```rust
pub mod annotation_syntax;
pub mod code_annotation;
pub mod diagnostic;
pub mod diagnostic_severity;
pub mod document;
pub mod markdown_source;
pub mod parse_report;
pub mod reference;
pub mod semantic_graph;
pub mod wiki_link_token;

pub use annotation_syntax::AnnotationSyntax;
pub use code_annotation::CodeAnnotation;
pub use diagnostic::Diagnostic;
pub use diagnostic_severity::DiagnosticSeverity;
pub use document::Document;
pub use markdown_source::MarkdownSource;
pub use parse_report::ParseReport;
pub use reference::Reference;
pub use semantic_graph::SemanticGraph;
pub use wiki_link_token::WikiLinkToken;
```

Update the crate-root re-export in `crates/indexer/src/lib.rs` so public graph fields are backed by public model types:

```rust
pub use model::{
    CodeAnnotation, Diagnostic, DiagnosticSeverity, Document, MarkdownSource, Reference,
    SemanticGraph, WikiLinkToken,
};
```

### 1.5 `SemanticGraph`

Update `crates/indexer/src/model/semantic_graph.rs`:

```rust
use crate::model::{CodeAnnotation, Diagnostic, Document, MarkdownSource, Reference};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SemanticGraph {
    pub documents: Vec<Document>,
    pub annotations: Vec<CodeAnnotation>,
    pub references: Vec<Reference>,
    pub markdown_sources: Vec<MarkdownSource>,
    pub diagnostics: Vec<Diagnostic>,
}
```

Update every existing `SemanticGraph` literal and assertion to include the new fields or use `..SemanticGraph::default()`.

## 2. Markdown parser plumbing

### 2.1 Keep `parse_markdown` crate-private

`crates/indexer/src/lib.rs` currently exposes `pub mod markdown`, but markdown parsing is only used internally by the indexer scan path. Change `parse_markdown` in `crates/indexer/src/markdown/mod.rs` from `pub fn` to `pub(crate) fn` when changing its return type. This avoids a public function returning crate-private parse payloads.

The only public markdown API added in this feature is the LSP support function described in section 2.6.

### 2.2 `MarkdownParse`

Create `crates/indexer/src/markdown/markdown_parse.rs`:

Prerequisite: complete section 2.3 first, and have section 2.7 in place before wiring the wiki-link fields into the new payload.

```rust
use crate::model::{Document, WikiLinkToken};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MarkdownParse {
    pub(crate) document: Option<Document>,
    pub(crate) wiki_links: Vec<WikiLinkToken>,
    pub(crate) wiki_link_diagnostics: Vec<crate::model::Diagnostic>,
}
```

Update `parse_markdown` to return:

```rust
IndexerResult<ParseReport<MarkdownParse>>
```

Preserve existing frontmatter behavior exactly:

- absent frontmatter still produces no document and no diagnostic;
- unterminated frontmatter still produces the existing diagnostic;
- invalid YAML still produces the existing invalid-frontmatter diagnostic;
- missing or empty `id`/`kind` still produces the existing required-fields diagnostic;
- valid frontmatter still trims `id`, `kind`, and blank `title` as it does today.

Wiki-link extraction rules by frontmatter state:

- valid frontmatter: extract wiki links only from the body after the closing frontmatter delimiter, using original file line numbers;
- absent frontmatter: extract wiki links from the whole file so annotation-only reference docs can become sources after scan validation;
- invalid, unterminated, or incomplete frontmatter: do not extract wiki links; keep the existing frontmatter diagnostic and let existing annotation extraction continue independently.

Keep frontmatter diagnostics in `ParseReport::diagnostics`. Keep diagnostics produced by wiki-link extraction in `MarkdownParse::wiki_link_diagnostics` so `scan_repository` can suppress them for plain frontmatter-less Markdown files that have no Soul ID source while still preserving invalid-frontmatter diagnostics.

### 2.3 Frontmatter body coordinates

Extend `crates/indexer/src/markdown/frontmatter_block.rs` so `parse_markdown` can know the body slice and original body start line without re-scanning elsewhere:

```rust
pub(crate) enum FrontmatterBlock<'a> {
    Absent { body: &'a str, body_start_line: usize },
    Unterminated,
    Present {
        frontmatter: &'a str,
        body: &'a str,
        body_start_line: usize,
    },
}
```

Update `extract_frontmatter` in `crates/indexer/src/markdown/mod.rs` to return:

- `Absent { body: input, body_start_line: 1 }` when the file does not start with a frontmatter delimiter;
- `Present { frontmatter, body, body_start_line }` where `body` starts after the closing delimiter and optional following newline;
- `Unterminated` for the current unterminated cases.

Keep line numbers 1-based. For a frontmatter block closed as:

```text
---
id: x
kind: concept
---
Body
```

`Body` starts at line 5.

For a frontmatter block closed at EOF, the body is empty and `wiki_links` is empty.

### 2.4 Shared wiki-link validation

Create `crates/indexer/src/markdown/wikilink_validation.rs`:

```rust
pub(crate) const MAX_REFERENCE_DISPLAY_BYTES: usize = 1024;

pub(crate) fn is_valid_reference_target_id(input: &str) -> bool {
    let target = input;
    !target.is_empty()
        && target == target.trim()
        && target.chars().all(|ch| {
            !ch.is_whitespace() && !ch.is_control() && !matches!(ch, '[' | ']' | '|')
        })
}

pub(crate) fn normalize_reference_display_text(input: &str) -> Option<String> {
    let display_text = input.trim();
    (!display_text.is_empty()).then(|| display_text.to_string())
}

pub(crate) fn is_valid_reference_display_text(input: &str) -> bool {
    input.trim().len() <= MAX_REFERENCE_DISPLAY_BYTES
}
```

Parser call sites must trim the parsed target before calling `is_valid_reference_target_id` and turn invalid targets/display text into `Diagnostic`. Index-loading call sites must pass the raw persisted `target_id` to `is_valid_reference_target_id` so leading/trailing whitespace is rejected as corrupt persisted state, and must return `IndexerError::index_corruption` for invalid persisted rows.

### 2.5 Shared markdown visibility walker

Create the shared Markdown visibility types with one primary struct per file:

- `crates/indexer/src/markdown/scanned_markdown_line.rs` owns `ScannedMarkdownLine`;
- `crates/indexer/src/markdown/fence_state.rs` owns `FenceState`;
- `crates/indexer/src/markdown/markdown_line_scanner.rs` owns `MarkdownLineScanner`.

Both `annotations.rs` and `wikilinks.rs` must use `MarkdownLineScanner` so examples inside code blocks are ignored consistently.

Use this concrete crate-private API shape so annotation extraction and wiki-link extraction share the same visibility decisions without sharing annotation parsing:

`crates/indexer/src/markdown/scanned_markdown_line.rs`:

```rust
use std::ops::Range;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ScannedMarkdownLine<'a> {
    pub(crate) line_number: usize,
    pub(crate) raw: &'a str,
    pub(crate) is_fenced_code: bool,
    pub(crate) is_indented_code: bool,
    pub(crate) visible_ranges: Vec<Range<usize>>,
}

impl<'a> ScannedMarkdownLine<'a> {
    pub(crate) fn is_annotation_candidate(&self) -> bool {
        !self.is_fenced_code && !self.is_indented_code
    }
}
```

`crates/indexer/src/markdown/fence_state.rs`:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FenceState {
    pub(crate) marker: char,
    pub(crate) len: usize,
}
```

`crates/indexer/src/markdown/markdown_line_scanner.rs`:

```rust
use crate::markdown::{fence_state::FenceState, scanned_markdown_line::ScannedMarkdownLine};
use std::ops::Range;

#[derive(Debug, Default)]
pub(crate) struct MarkdownLineScanner {
    fence: Option<FenceState>,
    in_html_comment: bool,
}

impl MarkdownLineScanner {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn scan_line<'a>(
        &mut self,
        line_number: usize,
        line: &'a str,
    ) -> ScannedMarkdownLine<'a> {
        if let Some(fence) = self.fence.clone() {
            if closes_fence(line, fence.marker, fence.len) {
                self.fence = None;
            }
            return ScannedMarkdownLine {
                line_number,
                raw: line,
                is_fenced_code: true,
                is_indented_code: false,
                visible_ranges: Vec::new(),
            };
        }

        if let Some(fence) = opens_fence(line) {
            self.fence = Some(fence);
            return ScannedMarkdownLine {
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

        ScannedMarkdownLine {
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

fn visible_non_comment_non_inline_code_ranges(line: &str, in_html_comment: &mut bool) -> Vec<Range<usize>> {
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
        let ch = line[index..].chars().next().expect("index on char boundary");
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
```

Required behavior:

- Track fenced code blocks started by a line whose trimmed text begins with three or more backticks or tildes.
- End a fenced block only with a matching fence marker of at least the opening length, optionally followed by whitespace.
- Treat lines with at least four leading spaces or one leading tab as indented code when not already in a fenced block.
- Treat inline code spans delimited by backticks as hidden ranges for wiki-link extraction.
- Treat HTML comments as hidden ranges for wiki-link extraction, including multi-line comments.
- For annotation extraction, use `ScannedMarkdownLine::is_annotation_candidate()` and then apply the existing `annotations.rs` HTML-comment parser to `line.raw.trim()`. Do not use `visible_ranges` for annotation extraction, because `visible_ranges` intentionally hides HTML comments from wiki-link extraction.
- For wiki-link extraction, scan only visible non-code, non-comment ranges.

Do not move HTML-comment annotation parsing out of `annotations.rs`; only move shared Markdown visibility decisions into the scanner. `annotations.rs` must continue to own parsing of `<!-- soul ... -->` payloads and diagnostics.

### 2.6 Public LSP support function, backed by the same parser

Add a public function to `crates/indexer/src/markdown/mod.rs` because `soul-lsp` is a separate crate and must not duplicate wiki-link parsing:

Prerequisite: complete section 2.7 first, because this function delegates to `wikilinks::extract_wikilinks`.

```rust
pub fn wikilink_at_position(
    input: &str,
    line: usize,
    utf16_col: usize,
) -> Option<crate::model::WikiLinkToken> {
    let normalized = input.replace("\r\n", "\n");
    let report = match extract_frontmatter(&normalized) {
        FrontmatterBlock::Absent {
            body,
            body_start_line,
        } => wikilinks::extract_wikilinks(std::path::Path::new(""), body, body_start_line),
        FrontmatterBlock::Present {
            body,
            body_start_line,
            ..
        } => wikilinks::extract_wikilinks(std::path::Path::new(""), body, body_start_line),
        FrontmatterBlock::Unterminated => crate::model::ParseReport {
            value: Vec::new(),
            diagnostics: Vec::new(),
        },
    };
    report.value.into_iter().find(|token| {
        token.start_line == line && token.end_line == line && token.start_col <= utf16_col && utf16_col < token.end_col
    })
}
```

`wikilink_at_position` may ignore diagnostics because the LSP only needs a token under the cursor. The function must call the same `wikilinks::extract_wikilinks` implementation used by `parse_markdown`.

### 2.7 Wiki-link extraction

Create `crates/indexer/src/markdown/wikilinks.rs` with `extract_wikilinks(path, input, start_line) -> ParseReport<Vec<WikiLinkToken>>`.

The final function must:

- use `MarkdownLineScanner` to skip fenced code blocks, indented code blocks, inline code spans, and HTML comments;
- scan for `[[...]]` with a stateful parser rather than regex;
- treat `\[[not a link]]` as escaped and not a link;
- support multiple links on one line;
- parse nested brackets by producing only the innermost valid token for `[[outer [[inner]]]]`;
- split on the first `|`;
- trim the target and display portions;
- reject an empty target, any target with whitespace/control characters, or any target containing `[`, `]`, or `|`;
- normalize whitespace-only display text to `None`;
- reject display text whose trimmed UTF-8 byte length exceeds `MAX_REFERENCE_DISPLAY_BYTES`;
- reject any wiki-link sequence that crosses a line boundary with a diagnostic and no token;
- compute columns as UTF-16 code units from the start of the original line;
- return tokens sorted by `start_line`, `start_col`, `target_id`.

Diagnostics must use `DiagnosticSeverity::Error`, `path: path.to_path_buf()`, and `line: Some(line_number)` for malformed links.

Use this concrete parsing shape:

```rust
pub(crate) fn extract_wikilinks(
    path: &std::path::Path,
    input: &str,
    start_line: usize,
) -> ParseReport<Vec<WikiLinkToken>> {
    let mut scanner = MarkdownLineScanner::new();
    let mut tokens = Vec::new();
    let mut diagnostics = Vec::new();

    for (offset, line) in input.lines().enumerate() {
        let line_number = start_line + offset;
        let scanned = scanner.scan_line(line_number, line);
        if scanned.is_fenced_code || scanned.is_indented_code {
            continue;
        }

        for range in scanned.visible_ranges {
            scan_visible_range(path, line_number, line, range, &mut tokens, &mut diagnostics);
        }
    }

    tokens.sort_by(|left, right| {
        left.start_line
            .cmp(&right.start_line)
            .then(left.start_col.cmp(&right.start_col))
            .then(left.target_id.cmp(&right.target_id))
    });

    ParseReport {
        value: tokens,
        diagnostics,
    }
}
```

`scan_visible_range` must walk bytes/char boundaries within the visible range and use these rules:

1. Ignore any `[[` whose first `[` is immediately preceded by an unescaped backslash in the original line.
2. Keep a stack of unescaped `[[` byte offsets.
3. On `]]`, pop the most recent opener and parse only that innermost candidate.
4. Mark any earlier opener that contains the accepted innermost candidate as consumed so `[[outer [[inner]]]]` emits only `inner` and no outer diagnostic.
5. At end of the visible range, emit one diagnostic for each unclosed opener that was not consumed by an accepted nested token.
6. Split candidate text on the first `|`, trim target/display, validate with `wikilink_validation`, normalize display text, and push `WikiLinkToken`.
7. Convert byte offsets to UTF-16 columns with `line[..byte_offset].encode_utf16().count()`.
8. Malformed candidates (`[[]]`, `[[|text]]`, invalid target, overlong display) produce diagnostics and no token.

Because `input.lines()` cannot contain a newline inside a line, cross-line detection must be done before per-line extraction: if a line contains an unescaped `[[` with no matching `]]` before the line ends, emit the malformed-link diagnostic for that line and do not carry parser state to the next line.

### 2.8 Module declarations

Update `crates/indexer/src/markdown/mod.rs` declarations:

```rust
pub(crate) mod annotations;
pub(crate) mod frontmatter;
pub(crate) mod frontmatter_block;
pub(crate) mod fence_state;
pub(crate) mod markdown_line_scanner;
pub(crate) mod markdown_parse;
pub(crate) mod scanned_markdown_line;
pub(crate) mod wikilink_validation;
pub(crate) mod wikilinks;
```

## 3. Scan integration

Update only the existing `CandidateKind::Document` arm in `crates/indexer/src/scan/mod.rs`. Do not create a second Markdown scanner.

### 3.1 Per-file staging

For each Markdown file:

1. Read contents using the existing `fs::read_to_string` path.
2. Call `parse_markdown(&candidate.display_path, &contents)?`.
3. Call `extract_annotations(&candidate.display_path, &contents)?`.
4. Stage `document`, `annotations`, `wiki_links`, and diagnostics in local variables.
5. Decide whether wiki links from this file have a valid source ID.
6. Append the staged document and annotations regardless of wiki-link source validity.
7. Append references/markdown_sources only when source-ID validation passes.
8. Append frontmatter and annotation diagnostics from all stages. Append wiki-link diagnostics only for Markdown files with a valid wiki-link source ID or an ambiguous/conflicting wiki-link source ID diagnostic; suppress wiki-link diagnostics for frontmatter-less Markdown with zero Soul annotation IDs.

This preserves existing behavior: malformed or ambiguous wiki-link source identity must not drop an existing document or an existing Markdown HTML-comment annotation.

### 3.2 Source-ID decision

For one Markdown file, derive `annotation_ids` from the staged `ann_report.value` after `extract_annotations` has used `MarkdownLineScanner` to ignore examples.

Rules:

- If `document` is `Some(doc)` and `annotation_ids` is empty: wiki links use `doc.id`.
- If `document` is `Some(doc)` and every annotation ID equals `doc.id`: wiki links use `doc.id`.
- If `document` is `Some(doc)` and any annotation ID differs from `doc.id`: append a diagnostic to that file and skip only its wiki-link references.
- If `document` is `None` and there is exactly one unique annotation ID: wiki links use that ID and the file adds one `MarkdownSource { source_id, source_path }` row.
- If `document` is `None` and there are zero annotation IDs: ignore wiki links and wiki-link diagnostics silently because plain Markdown with no Soul ID is out of scope.
- If `document` is `None` and there are multiple unique annotation IDs: append a diagnostic to that file and skip only its wiki-link references.

Use deterministic diagnostic wording. Required substrings for tests:

- `wiki links skipped: frontmatter id` for document/annotation disagreement;
- `wiki links skipped: multiple markdown annotation ids` for frontmatter-less multi-ID Markdown.

### 3.3 Build references

For each accepted `WikiLinkToken`, push:

```rust
Reference {
    source_id: source_id.clone(),
    target_id: token.target_id,
    source_path: candidate.display_path.clone(),
    source_start_line: token.start_line,
    source_start_col: token.start_col,
    source_end_line: token.end_line,
    source_end_col: token.end_col,
    display_text: token.display_text,
}
```

Do not validate target existence during scan. Broken target reporting is out of scope for first pass.

### 3.4 Deduplication and filtering

Keep the existing document dedupe behavior: lexicographically first document path wins for duplicate document IDs and duplicate losers produce diagnostics.

After document dedupe:

- retain references from surviving frontmatter-backed documents only when `(source_id, source_path)` matches a surviving `Document`;
- retain references from annotation-only sources only when `(source_id, source_path)` matches an accepted `MarkdownSource`;
- drop references from discarded duplicate document paths;
- do not drop annotations from discarded duplicate documents, because annotations are existing graph behavior and are independent implementation locations.

### 3.5 Final ordering

Before returning from `scan_repository`, sort:

- `graph.documents` by `path`, then `id`;
- `graph.annotations` by `path`, then `line`, then `id`;
- `graph.references` by `source_path`, then `source_start_line`, then `source_start_col`, then `target_id`;
- `graph.markdown_sources` by `source_path`, then `source_id`;
- `graph.diagnostics` by `path`, then `line`, then `message`.

## 4. Index persistence

Extend `crates/indexer/src/index/mod.rs`. Do not create another index module.

### 4.1 Migrations

Create `crates/indexer/migrations/0002_doc_references.up.sql`:

```sql
CREATE TABLE doc_references (
    id                INTEGER PRIMARY KEY,
    source_id         TEXT NOT NULL CHECK (length(trim(source_id)) > 0),
    target_id         TEXT NOT NULL CHECK (length(trim(target_id)) > 0),
    source_path       TEXT NOT NULL CHECK (length(trim(source_path)) > 0),
    source_start_line INTEGER NOT NULL CHECK (source_start_line > 0),
    source_start_col  INTEGER NOT NULL CHECK (source_start_col >= 0),
    source_end_line   INTEGER NOT NULL CHECK (source_end_line = source_start_line),
    source_end_col    INTEGER NOT NULL CHECK (source_end_col > source_start_col),
    display_text      TEXT
);
```

Create `crates/indexer/migrations/0003_markdown_sources.up.sql`:

```sql
CREATE TABLE markdown_sources (
    source_id   TEXT NOT NULL CHECK (length(trim(source_id)) > 0),
    source_path TEXT NOT NULL CHECK (length(trim(source_path)) > 0),
    PRIMARY KEY (source_id, source_path)
);
```

### 4.2 Error type for corrupt persisted rows

Add a dedicated variant and constructor in `crates/indexer/src/error/mod.rs`:

```rust
#[error("corrupt index at `{path}`: {message} {location}")]
IndexCorruption {
    path: PathBuf,
    message: String,
    location: ErrorLocation,
},
```

```rust
#[track_caller]
pub fn index_corruption(path: PathBuf, message: impl Into<String>) -> Self {
    Self::IndexCorruption {
        path,
        message: message.into(),
        location: ErrorLocation::from(Location::caller()),
    }
}
```

Use `IndexerError::index_db` only for SQLx/migration failures. Use `IndexerError::index_corruption` when a row loads successfully from SQLite but violates the model contract.

Because `load_graph(pool)` and `explain_from_index(pool, id)` currently receive only `&SqlitePool`, add a private helper in `crates/indexer/src/index/mod.rs` to recover the main database path for corruption errors without changing public call signatures:

```rust
async fn pool_index_path(pool: &SqlitePool) -> PathBuf {
    let row = sqlx::query("PRAGMA database_list")
        .fetch_all(pool)
        .await
        .ok()
        .and_then(|rows| {
            rows.into_iter().find_map(|row| {
                use sqlx::Row as _;
                let name = row.try_get::<String, _>("name").ok()?;
                (name == "main").then(|| row.try_get::<String, _>("file").ok()).flatten()
            })
        });

    row.map(PathBuf::from).unwrap_or_default()
}
```

Call `let index_path = pool_index_path(pool).await;` once near the start of `load_graph` and `explain_from_index`. Use that `index_path.clone()` for all `IndexerError::index_corruption` calls in those functions. Keep the existing SQLx error mapping behavior for database/query failures.

### 4.3 Write path

In `write_index`:

- keep using the existing migration call;
- in the existing transaction, delete from `doc_references` and `markdown_sources` before inserts;
- continue deleting documents, annotations, and diagnostics as today;
- insert `graph.markdown_sources` into `markdown_sources`;
- insert `graph.references` into `doc_references`;
- bind line/column `usize` values as `i64`, matching the current annotation line binding.

### 4.4 Load helpers

Add private helpers inside `crates/indexer/src/index/mod.rs`, not a new index layer:

- row-to-`Document`, preserving current behavior;
- row-to-`CodeAnnotation`, preserving current metadata/syntax fallback behavior;
- row-to-`Diagnostic`, preserving current severity fallback behavior;
- row-to-`MarkdownSource`, validating non-empty trimmed ID and repo-relative `source_path`;
- row-to-`Reference`, validating IDs, repo-relative `source_path`, target grammar, display text length, single-line half-open span, positive line, non-negative columns, and integer conversion.

Use concrete helper signatures like these so corrupt-row validation is not reimplemented differently in `load_graph` and `explain_from_index`:

```rust
fn row_to_document(row: &sqlx::sqlite::SqliteRow) -> Document;

fn row_to_annotation(row: &sqlx::sqlite::SqliteRow) -> CodeAnnotation;

fn row_to_diagnostic(row: &sqlx::sqlite::SqliteRow) -> Diagnostic;

fn row_to_markdown_source(
    row: &sqlx::sqlite::SqliteRow,
    index_path: &Path,
) -> IndexerResult<MarkdownSource>;

fn row_to_reference(
    row: &sqlx::sqlite::SqliteRow,
    index_path: &Path,
) -> IndexerResult<Reference>;
```

`row_to_document`, `row_to_annotation`, and `row_to_diagnostic` preserve today's permissive load behavior, including metadata/syntax/severity fallback. `row_to_markdown_source` and `row_to_reference` enforce the new persisted model contract.

For integer fields in `row_to_reference`, do not cast with `as usize`. Load as `i64`, validate, then convert:

```rust
fn positive_usize(index_path: &Path, field: &str, value: i64) -> IndexerResult<usize> {
    if value <= 0 {
        return Err(IndexerError::index_corruption(
            index_path.to_path_buf(),
            format!("invalid `{field}`: expected positive integer, got {value}"),
        ));
    }
    usize::try_from(value).map_err(|_| {
        IndexerError::index_corruption(
            index_path.to_path_buf(),
            format!("invalid `{field}`: out of range for usize"),
        )
    })
}

fn non_negative_usize(index_path: &Path, field: &str, value: i64) -> IndexerResult<usize> {
    if value < 0 {
        return Err(IndexerError::index_corruption(
            index_path.to_path_buf(),
            format!("invalid `{field}`: expected non-negative integer, got {value}"),
        ));
    }
    usize::try_from(value).map_err(|_| {
        IndexerError::index_corruption(
            index_path.to_path_buf(),
            format!("invalid `{field}`: out of range for usize"),
        )
    })
}
```

`row_to_reference` must:

1. trim-check `source_id`, `target_id`, and `source_path`;
2. validate `target_id` with `is_valid_reference_target_id`;
3. validate `display_text` with `is_valid_reference_display_text` when present;
4. require `source_end_line == source_start_line`;
5. require `source_end_col > source_start_col`;
6. return `IndexerError::index_corruption(index_path.to_path_buf(), message)` for every model-contract violation.

When validating references, require every loaded reference source to resolve to one of:

- a frontmatter-backed document pair `(document.id, document.path)`;
- a promoted markdown source pair `(markdown_source.source_id, markdown_source.source_path)`.

If a persisted reference has no source pair, return `IndexerError::index_corruption`.

Reject duplicate `doc_references` rows with the same `(source_id, target_id, source_path, source_start_line, source_start_col, source_end_line, source_end_col, display_text)` tuple during row loading. Duplicates are corrupt index state.

Implement source-pair and duplicate validation after rows are converted:

```rust
let document_sources: BTreeSet<(String, PathBuf)> = documents
    .iter()
    .map(|document| (document.id.clone(), document.path.clone()))
    .collect();
let markdown_source_pairs: BTreeSet<(String, PathBuf)> = markdown_sources
    .iter()
    .map(|source| (source.source_id.clone(), source.source_path.clone()))
    .collect();

let mut seen_references = BTreeSet::new();
for reference in &references {
    let source_pair = (reference.source_id.clone(), reference.source_path.clone());
    if !document_sources.contains(&source_pair) && !markdown_source_pairs.contains(&source_pair) {
        return Err(IndexerError::index_corruption(
            index_path.clone(),
            format!(
                "reference source `{}` at `{}` does not match any document or markdown source",
                reference.source_id,
                reference.source_path.display()
            ),
        ));
    }

    let key = (
        reference.source_id.clone(),
        reference.target_id.clone(),
        reference.source_path.clone(),
        reference.source_start_line,
        reference.source_start_col,
        reference.source_end_line,
        reference.source_end_col,
        reference.display_text.clone(),
    );
    if !seen_references.insert(key) {
        return Err(IndexerError::index_corruption(
            index_path.clone(),
            "duplicate doc_references row",
        ));
    }
}
```

### 4.5 Query ordering

Update `load_graph` to use stable ordering:

- documents: `ORDER BY path, id`;
- annotations: `ORDER BY path, line, id`;
- diagnostics: `ORDER BY path, line, message`;
- markdown sources: `ORDER BY source_path, source_id`;
- references: `ORDER BY source_path, source_start_line, source_start_col, target_id`.

Update `explain_from_index` to:

- load documents where `id = ?` with `ORDER BY path, id`;
- load annotations where `id = ?` with `ORDER BY path, line, id`;
- load references where `target_id = ?` with `ORDER BY source_path, source_start_line, source_start_col, target_id`;
- load diagnostics with `ORDER BY path, line, message`;
- separately load all document source pairs and all `markdown_sources` source pairs needed for validation, then validate returned reference rows against that full source set. Do not validate `target_id = ?` reference rows only against the result documents for the target ID.

Return `SemanticGraph { documents, annotations, references, markdown_sources, diagnostics }` from `load_graph`.

## 5. Explain graph and result

### 5.1 `ExplainResult`

Update `crates/indexer/src/graph/explain_result.rs`:

```rust
use crate::model::{CodeAnnotation, Diagnostic, Document, Reference};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExplainResult {
    pub id: String,
    pub documents: Vec<Document>,
    pub annotations: Vec<CodeAnnotation>,
    pub references: Vec<Reference>,
    pub scan_diagnostics: Vec<Diagnostic>,
}
```

### 5.2 Live explain

Update `crates/indexer/src/graph/mod.rs` so `explain(graph, id)` keeps current document/annotation filtering and adds:

```rust
let references = graph
    .references
    .iter()
    .filter(|reference| reference.target_id == id)
    .cloned()
    .collect();
```

Include `references` in the returned `ExplainResult`.

### 5.3 Indexed explain

Update `explain_from_index` as described in section 4.5 and include `references` in `ExplainResult`.

## 6. MCP and CLI output

### 6.1 Untrusted display data

Treat all values loaded from repo files or SQLite as untrusted display data:

- IDs;
- kinds;
- titles;
- paths;
- annotation metadata keys/values;
- annotation raw strings;
- wiki-link display text;
- diagnostic messages.

Add small private formatting helpers in the formatter files that render control characters visibly. Required escaping contract:

- `\n` renders as `\\n`;
- `\r` renders as `\\r`;
- `\t` renders as `\\t`;
- other control characters render as `\\u{HEX}`;
- Markdown prose fields use a dedicated escaping helper so headings, bullets, emphasis, links, and code spans cannot be forged.
- code-span fields either escape backticks or fall back to escaped Markdown text instead of emitting a malformed code span.

Use these helpers in new reference output and opportunistically on existing document, annotation, and diagnostic output touched by this feature.

Add these private helpers to `crates/indexer/src/mcp/format.rs` and use them from `explain_result`, `document`, `annotation`, the new reference formatter, diagnostics formatting, and any touched `gaps` output:

```rust
fn visible_text(input: &str) -> String {
    let mut out = String::new();
    for ch in input.chars() {
        match ch {
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ch if ch.is_control() => out.push_str(&format!("\\u{{{:X}}}", ch as u32)),
            ch => out.push(ch),
        }
    }
    out
}

fn visible_path(path: &std::path::Path) -> String {
    visible_text(&path.display().to_string())
}

fn escape_mcp_markdown_text(input: &str) -> String {
    let mut out = String::new();
    for ch in input.chars() {
        match ch {
            '\\' | '`' | '*' | '_' | '{' | '}' | '[' | ']' | '(' | ')' | '#' | '+' | '-'
            | '!' | '>' | '|' => {
                out.push('\\');
                out.push(ch);
            }
            ch => out.push(ch),
        }
    }
    out
}

fn markdown_text(input: &str) -> String {
    escape_mcp_markdown_text(&visible_text(input))
}

fn markdown_code_span(input: &str) -> String {
    let visible = visible_text(input);
    if visible.contains('`') {
        markdown_text(input)
    } else {
        format!("`{visible}`")
    }
}
```

Add equivalent private helpers to `crates/indexer/src/bin/commands/explain.rs` for CLI output, but the CLI helper does not need `markdown_code_span` because CLI output is plain text:

```rust
fn visible_text(input: &str) -> String {
    let mut out = String::new();
    for ch in input.chars() {
        match ch {
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ch if ch.is_control() => out.push_str(&format!("\\u{{{:X}}}", ch as u32)),
            ch => out.push(ch),
        }
    }
    out
}

fn visible_path(path: &std::path::Path) -> String {
    visible_text(&path.display().to_string())
}
```

Add the same `visible_text` and `visible_path` private helpers to `crates/indexer/src/bin/commands/index.rs` because the touched index count output includes the generated index path:

```rust
fn visible_text(input: &str) -> String {
    let mut out = String::new();
    for ch in input.chars() {
        match ch {
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ch if ch.is_control() => out.push_str(&format!("\\u{{{:X}}}", ch as u32)),
            ch => out.push(ch),
        }
    }
    out
}

fn visible_path(path: &std::path::Path) -> String {
    visible_text(&path.display().to_string())
}
```

Every new or touched `format!`/`println!` that includes repo or DB data must pass IDs, kinds, titles, paths, metadata keys/values, raw annotation strings, display text, and diagnostic messages through these helpers before interpolation.

### 6.2 MCP format

Update `crates/indexer/src/mcp/format.rs`:

- `explain_result` must consider documents, annotations, and references before emitting the empty message.
- The empty message becomes `No documents, annotations, or references found for this ID.`
- Add a `## Referenced by` section after annotations and before diagnostics.
- Each reference row must include source ID, source path, line/column range, and display text when present.
- Render scan diagnostics after references.
- Do not return early before diagnostics are rendered.
- Use `markdown_text` for prose/emphasis fields and `markdown_code_span` for code/path fields in all touched MCP Markdown output so untrusted data cannot forge headings, bullets, or code spans.

Output order:

1. `# Soul ID: ...`
2. documents section when non-empty;
3. annotations section when non-empty;
4. referenced-by section when non-empty;
5. empty message when all three collections are empty;
6. scan diagnostics section when non-empty.

Update the `soul_explain` tool description in `crates/indexer/src/mcp/mod.rs` to mention wiki-link references separately from code annotations.

Update `soul_index_impl` count text in `crates/indexer/src/mcp/mod.rs`:

```text
Indexed {documents} documents, {annotations} annotations, {references} references, {diagnostics} diagnostics.
```

Update the `soul_index` tool description to match the four-count output.

### 6.3 CLI output

Update `crates/indexer/src/bin/commands/explain.rs`:

- keep the existing `ID`, `Documents`, `Annotations`, and `Diagnostics` sections;
- add a `Referenced by:` section between `Annotations` and `Diagnostics`;
- print `none` when there are no references;
- each reference row prints `source_path:start_line:start_col-end_col -> source_id`, plus display text when present;
- keep diagnostics visible.

Update `crates/indexer/src/bin/commands/index.rs` count text:

```text
Indexed {documents} documents, {annotations} annotations, {references} references, {diagnostics} diagnostics → {path}
```

The `{path}` value in that output must be produced with `visible_path(&index_path)`, not `index_path.display()` directly.

## 7. LSP navigation

Update `crates/soul-lsp/src/server.rs`. Extend the existing `Server`; do not create a second server or bypass `SemanticGraph`.

### 7.1 Open document cache

Add an open-document cache:

- field: `open_documents: RwLock<BTreeMap<Uri, String>>`;
- initialize it in `Server::new`;
- advertise full text document sync in `initialize` with open/close and full-change support;
- implement `did_open`, `did_change`, and `did_close` to maintain the cache.

Use full sync for the first pass. Incremental sync is out of scope.

Use the existing `tower_lsp_server::ls_types` API that is already imported in `server.rs`. Add imports for:

```rust
DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
TextDocumentSyncCapability, TextDocumentSyncKind, TextDocumentSyncOptions,
```

Also add `collections::BTreeMap` to the existing `std` imports.

Update `Server` and `Server::new`:

```rust
pub struct Server {
    client: Client,
    root: PathBuf,
    graph: RwLock<Option<SemanticGraph>>,
    open_documents: RwLock<BTreeMap<Uri, String>>,
}

impl Server {
    pub fn new(client: Client, root: PathBuf) -> Self {
        Self {
            client,
            root,
            graph: RwLock::new(None),
            open_documents: RwLock::new(BTreeMap::new()),
        }
    }
}
```

In `initialize`, add `text_document_sync` to `ServerCapabilities`:

```rust
text_document_sync: Some(TextDocumentSyncCapability::Options(TextDocumentSyncOptions {
    open_close: Some(true),
    change: Some(TextDocumentSyncKind::FULL),
    ..Default::default()
})),
```

Add the notification handlers to the existing `impl LanguageServer for Server`:

```rust
async fn did_open(&self, params: DidOpenTextDocumentParams) {
    self.open_documents.write().await.insert(
        params.text_document.uri,
        params.text_document.text,
    );
}

async fn did_change(&self, params: DidChangeTextDocumentParams) {
    if let Some(change) = params.content_changes.into_iter().rev().find(|change| change.range.is_none()) {
        self.open_documents
            .write()
            .await
            .insert(params.text_document.uri, change.text);
    } else {
        self.client
            .log_message(
                MessageType::WARNING,
                "soul-lsp: ignoring incremental text change because only full sync is supported",
            )
            .await;
    }
}

async fn did_close(&self, params: DidCloseTextDocumentParams) {
    self.open_documents
        .write()
        .await
        .remove(&params.text_document.uri);
}
```

### 7.2 Shared token lookup

Add a helper in `server.rs` that:

1. accepts the request URI and LSP `Position`;
2. reads the current text from `open_documents` when present;
3. falls back to reading the file from disk when the document is not open;
4. calls `indexer::markdown::wikilink_at_position(&text, line + 1, character as usize)`;
5. returns the `WikiLinkToken` when the cursor is inside a wiki-link span.

Do not parse `[[...]]` manually in `soul-lsp`.

Use this concrete cache/disk split and pure token helper shape so cache preference and cursor-position behavior can be tested without constructing a full LSP server:

```rust
pub(crate) fn wikilink_lookup_text(cached_text: Option<String>, uri: &Uri) -> Option<String> {
    cached_text.or_else(|| {
        let path = uri.to_file_path().ok()?;
        std::fs::read_to_string(path).ok()
    })
}

pub(crate) fn wikilink_token_from_text(
    cached_text: Option<String>,
    uri: &Uri,
    position: Position,
) -> Option<indexer::WikiLinkToken> {
    let text = wikilink_lookup_text(cached_text, uri)?;
    indexer::markdown::wikilink_at_position(
        &text,
        (position.line + 1) as usize,
        position.character as usize,
    )
}

async fn wikilink_token_at_position(
    &self,
    uri: &Uri,
    position: Position,
) -> Option<indexer::WikiLinkToken> {
    let cached_text = self.open_documents.read().await.get(uri).cloned();
    wikilink_token_from_text(cached_text, uri, position)
}
```

Use `std::fs::read_to_string` for the first pass because `tokio::fs` is not currently enabled by `soul-lsp`'s `tokio` features and this fallback is only used for unopened documents.

### 7.3 Definition

Update `goto_definition` in this order:

1. If the cursor is on a wiki link, resolve `token.target_id`.
   - If a document with that ID exists, return a scalar location for the document path at `Range::default()`.
   - If no document exists, return annotation locations for that ID using the existing annotation location behavior.
2. Else keep existing annotation-to-document behavior.
3. Else keep existing document-to-annotations behavior.
4. Else return `None`.

To keep this testable without introducing a broad LSP integration harness or inline same-file test modules, extract the existing annotation/document lookup logic into small `pub(crate)` helpers in `server.rs` and use them from `goto_definition`:

```rust
pub(crate) fn annotation_locations_for_id(graph: &SemanticGraph, root: &Path, id: &str) -> Vec<Location>;

pub(crate) fn reference_locations_for_id(graph: &SemanticGraph, root: &Path, id: &str) -> Vec<Location>;

pub(crate) fn definition_response_for_id(graph: &SemanticGraph, root: &Path, id: &str) -> Option<GotoDefinitionResponse>;
```

`definition_response_for_id` must implement “document first, otherwise annotation locations” for wiki-link targets. Keep the current annotation-to-document and document-to-annotations behavior by calling the same helpers instead of duplicating location-building code.

### 7.4 Hover

Update `hover` in this order:

1. If the cursor is on a wiki link, show the target ID.
2. If the target document exists, include its kind, title when present, and path.
3. Include the number of code annotations for the target ID when non-zero.
4. Else keep existing annotation hover behavior.
5. Else keep existing document hover behavior.
6. Else return `None`.

Use explicit LSP Markdown escaping for every repo/index value inserted into hover Markdown. Control characters must render visibly, and Markdown metacharacters must not let IDs, titles, kinds, paths, or other values forge emphasis, headings, bullets, links, tables, or code spans.

Add private helpers in `server.rs`:

```rust
fn lsp_visible_text(input: &str) -> String {
    let mut out = String::new();
    for ch in input.chars() {
        match ch {
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ch if ch.is_control() => out.push_str(&format!("\\u{{{:X}}}", ch as u32)),
            ch => out.push(ch),
        }
    }
    out
}

fn escape_lsp_markdown_text(input: &str) -> String {
    let mut out = String::new();
    for ch in input.chars() {
        match ch {
            '\\' | '`' | '*' | '_' | '{' | '}' | '[' | ']' | '(' | ')' | '#'
            | '+' | '-' | '.' | '!' | '>' | '|' => {
                out.push('\\');
                out.push(ch);
            }
            ch => out.push(ch),
        }
    }
    out
}

fn lsp_markdown_text(input: &str) -> String {
    escape_lsp_markdown_text(&lsp_visible_text(input))
}

fn lsp_markdown_code_span(input: &str) -> String {
    let visible = lsp_visible_text(input);
    if visible.contains('`') {
        escape_lsp_markdown_text(&visible)
    } else {
        format!("`{visible}`")
    }
}
```

Extract hover Markdown construction into `pub(crate)` pure helpers so repo-pattern sibling test modules can assert output without driving a full LSP server:

```rust
pub(crate) fn hover_markdown_for_wikilink_target(graph: &SemanticGraph, target_id: &str) -> Option<String>;

pub(crate) fn hover_markdown_for_annotation(graph: &SemanticGraph, annotation: &CodeAnnotation) -> String;

pub(crate) fn hover_markdown_for_document(graph: &SemanticGraph, document: &Document) -> String;
```

These helpers must use `lsp_markdown_text` for prose/emphasis fields and `lsp_markdown_code_span` for path/location fields before inserting IDs, titles, kinds, paths, counts, or annotation locations into Markdown.

### 7.5 References

Update `references` so the resolved ID can come from:

- the wiki-link token under the cursor;
- the annotation under the cursor;
- the document for the current Markdown file.

Return both:

- existing code annotation locations for the ID;
- wiki-link reference locations from `graph.references` where `target_id == id`.

For reference locations, convert stored spans to LSP ranges by subtracting 1 from stored lines and using stored UTF-16 columns directly.

Return `None` only when the combined location list is empty.

Implement ID resolution through a pure helper plus a thin async `Server` method so the wiki-link, annotation, and document cases are tested independently without constructing `tower_lsp_server::Client`:

```rust
pub(crate) fn resolved_id_from_position_inputs(
    graph: &SemanticGraph,
    root: &Path,
    uri: &Uri,
    position: Position,
    wikilink_token: Option<indexer::WikiLinkToken>,
) -> Option<String> {
    if let Some(token) = wikilink_token {
        return Some(token.target_id);
    }
    if let Some(annotation) = annotation_at(graph, root, uri, position.line) {
        return Some(annotation.id.clone());
    }
    document_at(graph, root, uri).map(|document| document.id.clone())
}

async fn resolved_id_at_position(&self, graph: &SemanticGraph, uri: &Uri, position: Position) -> Option<String> {
    let wikilink_token = self.wikilink_token_at_position(uri, position.clone()).await;
    resolved_id_from_position_inputs(graph, &self.root, uri, position, wikilink_token)
}
```

`references` then builds `let mut locations = annotation_locations_for_id(...); locations.extend(reference_locations_for_id(...));` and returns `None` only when that combined vector is empty.

## 8. Tests and verification

### 8.1 Model/compile fallout

Update existing tests that construct `SemanticGraph` or `ExplainResult`:

- `crates/indexer/src/tests/graph.rs`;
- any new/changed tests in `crates/indexer/tests/explain.rs`;
- any LSP tests added in `crates/soul-lsp`.

Use concrete model updates in existing tests instead of broad “fix compile fallout” edits. Existing `SemanticGraph` literals must either provide both new collections or use `..SemanticGraph::default()`. Existing `ExplainResult` assertions must assert `references`.

Update the first `crates/indexer/src/tests/graph.rs` test to include a reference row and assert live explain returns it:

```rust
use crate::{
    graph::explain,
    model::{
        AnnotationSyntax, CodeAnnotation, Diagnostic, DiagnosticSeverity, Document, Reference,
        SemanticGraph,
    },
};
use std::path::PathBuf;

#[test]
fn returns_matches_references_and_preserves_global_scan_diagnostics() {
    let graph = SemanticGraph {
        documents: vec![Document {
            id: "interaction.checkout.create-order".to_string(),
            kind: "interaction".to_string(),
            title: Some("Create order".to_string()),
            path: PathBuf::from(".docs/interactions/checkout.md"),
        }],
        annotations: vec![CodeAnnotation {
            id: "interaction.checkout.create-order".to_string(),
            metadata: serde_json::Map::new(),
            path: PathBuf::from("fixtures/backend.rs"),
            line: 2,
            syntax: AnnotationSyntax("rust-attribute".to_string()),
            raw: r#"#[soul(id = "interaction.checkout.create-order")]"#.to_string(),
        }],
        references: vec![Reference {
            source_id: "interaction.checkout.flow".to_string(),
            target_id: "interaction.checkout.create-order".to_string(),
            source_path: PathBuf::from(".docs/interactions/flow.md"),
            source_start_line: 6,
            source_start_col: 4,
            source_end_line: 6,
            source_end_col: 54,
            display_text: Some("Create order".to_string()),
        }],
        markdown_sources: Vec::new(),
        diagnostics: vec![Diagnostic {
            severity: DiagnosticSeverity::Error,
            path: PathBuf::from("fixtures/bad.rs"),
            line: Some(1),
            message: "malformed soul attribute for interaction.checkout.create-order".to_string(),
        }],
    };

    let result = explain(&graph, "interaction.checkout.create-order");

    assert_eq!(result.id, "interaction.checkout.create-order");
    assert_eq!(result.documents.len(), 1);
    assert_eq!(result.annotations.len(), 1);
    assert_eq!(result.references.len(), 1);
    assert_eq!(result.references[0].source_id, "interaction.checkout.flow");
    assert_eq!(result.scan_diagnostics.len(), 1);
}
```

Update the no-match graph test so it also asserts no references:

```rust
#[test]
fn no_match_still_returns_global_scan_diagnostics() {
    let graph = SemanticGraph {
        diagnostics: vec![Diagnostic {
            severity: DiagnosticSeverity::Error,
            path: PathBuf::from("fixtures/bad.rs"),
            line: Some(1),
            message: "malformed soul attribute".to_string(),
        }],
        ..SemanticGraph::default()
    };

    let result = explain(&graph, "interaction.checkout.missing");

    assert!(result.documents.is_empty());
    assert!(result.annotations.is_empty());
    assert!(result.references.is_empty());
    assert_eq!(result.scan_diagnostics.len(), 1);
}
```

### 8.2 Markdown parser tests

Update `crates/indexer/src/tests/markdown.rs` for the new `ParseReport<MarkdownParse>` payload. Assert frontmatter diagnostics through `ParseReport::diagnostics` and wiki-link malformed-token diagnostics through `MarkdownParse::wiki_link_diagnostics`. Add tests for:

- valid frontmatter still produces the same `Document` fields;
- markdown without frontmatter still produces no document and no diagnostics;
- existing invalid-frontmatter diagnostics remain unchanged;
- basic `[[id]]`;
- `[[id|Display text]]`;
- multiple links on one line;
- escaped brackets: `\[[not-a-link]]` ignored;
- nested brackets: `[[outer [[inner]]]]` produces only `inner`;
- malformed `[[unclosed`, `[[]]`, `[[|text]]` diagnostics;
- invalid target IDs with whitespace/control/bracket/separator characters;
- whitespace-only display text normalizes to `None`;
- overlong display text becomes a diagnostic;
- cross-line wiki links become diagnostics and no token;
- frontmatter offset plus non-BMP text before the link proves 1-based line and UTF-16 column handling;
- ignored wiki-link-looking text inside fenced code blocks, indented code blocks, inline code spans, and HTML comments.

Use this import/helper shape when updating `crates/indexer/src/tests/markdown.rs`:

```rust
use std::path::Path;

use crate::markdown::{markdown_parse::MarkdownParse, parse_markdown};

fn parse_ok(input: &str) -> MarkdownParse {
    let report = parse_markdown(Path::new("doc.md"), input).expect("parse");
    assert!(report.diagnostics.is_empty());
    report.value
}
```

Update existing frontmatter tests to read `report.value.document` and to assert wiki-link payloads separately:

```rust
#[test]
fn parses_valid_frontmatter() {
    let input = "\
---
id: interaction.checkout.create-order
kind: interaction
title: Create order
---

# Checkout
";
    let report = parse_markdown(Path::new("checkout.md"), input).expect("parse");

    assert!(report.diagnostics.is_empty());
    assert!(report.value.wiki_links.is_empty());
    assert!(report.value.wiki_link_diagnostics.is_empty());
    let document = report.value.document.expect("document");
    assert_eq!(document.id, "interaction.checkout.create-order");
    assert_eq!(document.kind, "interaction");
    assert_eq!(document.title.as_deref(), Some("Create order"));
}

#[test]
fn ignores_markdown_without_frontmatter_as_document_but_still_parses_links() {
    let report = parse_markdown(Path::new("plain.md"), "See [[target.id]].").expect("parse");

    assert!(report.diagnostics.is_empty());
    assert!(report.value.document.is_none());
    assert_eq!(report.value.wiki_links.len(), 1);
    assert_eq!(report.value.wiki_links[0].target_id, "target.id");
    assert!(report.value.wiki_link_diagnostics.is_empty());
}

#[test]
fn invalid_frontmatter_keeps_frontmatter_diagnostics_and_suppresses_wikilinks() {
    let input = "\
---
id: [
kind: interaction
---
See [[target.id]].
";
    let report = parse_markdown(Path::new("bad.md"), input).expect("parse");

    assert!(report.value.document.is_none());
    assert!(report.value.wiki_links.is_empty());
    assert!(report.value.wiki_link_diagnostics.is_empty());
    assert_eq!(report.diagnostics.len(), 1);
    assert!(report.diagnostics[0].message.contains("invalid frontmatter"));
}
```

Add these parser tests for the new wiki-link payload:

```rust
#[test]
fn extracts_basic_display_and_multiple_wikilinks() {
    let input = "\
---
id: source.doc
kind: concept
---
Before [[target.one]] and [[target.two|Target Two]].
";
    let parsed = parse_ok(input);

    assert_eq!(parsed.wiki_links.len(), 2);
    assert_eq!(parsed.wiki_links[0].target_id, "target.one");
    assert_eq!(parsed.wiki_links[0].display_text, None);
    assert_eq!(parsed.wiki_links[0].start_line, 5);
    assert_eq!(parsed.wiki_links[0].start_col, 7);
    assert_eq!(parsed.wiki_links[0].end_col, 21);

    assert_eq!(parsed.wiki_links[1].target_id, "target.two");
    assert_eq!(parsed.wiki_links[1].display_text.as_deref(), Some("Target Two"));
    assert_eq!(parsed.wiki_links[1].start_line, 5);
    assert_eq!(parsed.wiki_links[1].start_col, 26);
    assert_eq!(parsed.wiki_links[1].end_col, 51);
}

#[test]
fn ignores_escaped_links_and_keeps_only_innermost_nested_link() {
    let parsed = parse_ok("\\[[not-a-link]] and [[outer [[inner]]]]");

    assert_eq!(parsed.wiki_links.len(), 1);
    assert_eq!(parsed.wiki_links[0].target_id, "inner");
    assert_eq!(parsed.wiki_links[0].start_line, 1);
    assert_eq!(parsed.wiki_links[0].start_col, 28);
    assert_eq!(parsed.wiki_links[0].end_col, 37);
    assert!(parsed.wiki_link_diagnostics.is_empty());
}

#[test]
fn reports_malformed_wikilinks_separately_from_frontmatter_diagnostics() {
    let input = "\
[[unclosed
[[]]
[[|text]]
[[bad id]]
[[ok.target|   ]]
";
    let parsed = parse_ok(input);

    assert_eq!(parsed.wiki_links.len(), 1);
    assert_eq!(parsed.wiki_links[0].target_id, "ok.target");
    assert_eq!(parsed.wiki_links[0].display_text, None);
    assert_eq!(parsed.wiki_link_diagnostics.len(), 4);
    assert!(
        parsed
            .wiki_link_diagnostics
            .iter()
            .all(|diagnostic| diagnostic.line.is_some())
    );
}

#[test]
fn rejects_overlong_display_text() {
    let display = "x".repeat(crate::markdown::wikilink_validation::MAX_REFERENCE_DISPLAY_BYTES + 1);
    let input = format!("[[target.id|{display}]]");
    let parsed = parse_ok(&input);

    assert!(parsed.wiki_links.is_empty());
    assert_eq!(parsed.wiki_link_diagnostics.len(), 1);
    assert!(parsed.wiki_link_diagnostics[0].message.contains("display"));
}

#[test]
fn computes_frontmatter_offset_and_utf16_columns() {
    let input = "\
---
id: source.doc
kind: concept
---
😀 [[target.id]]
";
    let parsed = parse_ok(input);

    assert_eq!(parsed.wiki_links.len(), 1);
    assert_eq!(parsed.wiki_links[0].start_line, 5);
    assert_eq!(parsed.wiki_links[0].start_col, 3);
    assert_eq!(parsed.wiki_links[0].end_col, 16);
}

#[test]
fn ignores_links_inside_code_and_html_comment_ranges() {
    let fence = "`".repeat(3);
    let input = format!(
        "{fence}\n[[fenced]]\n{fence}\n    [[indented]]\n`[[inline]]`\n<!-- [[comment]] -->\n[[visible]]\n"
    );
    let parsed = parse_ok(&input);

    assert_eq!(parsed.wiki_links.len(), 1);
    assert_eq!(parsed.wiki_links[0].target_id, "visible");
    assert_eq!(parsed.wiki_links[0].start_line, 7);
}
```

### 8.3 Markdown annotation visibility tests

Update/add tests for `crates/indexer/src/markdown/annotations.rs` through scan or parser-visible behavior:

- real top-level `<!-- soul id="..." -->` annotations are still extracted;
- annotations inside fenced code examples are ignored;
- annotations inside indented code examples are ignored;
- malformed real annotations still produce diagnostics.

Add concrete parser-visible tests in `crates/indexer/src/tests/markdown.rs` or a dedicated markdown-annotation test module:

```rust
use crate::markdown::annotations::extract_annotations;

#[test]
fn extracts_real_markdown_annotations_and_ignores_code_examples() {
    let fence = "`".repeat(3);
    let input = format!(
        "<!-- soul id=\"real.id\" -->\n{fence}\n<!-- soul id=\"fenced.id\" -->\n{fence}\n    <!-- soul id=\"indented.id\" -->\n"
    );
    let report = extract_annotations(Path::new("docs/reference.md"), &input).expect("annotations");

    assert!(report.diagnostics.is_empty());
    assert_eq!(report.value.len(), 1);
    assert_eq!(report.value[0].id, "real.id");
    assert_eq!(report.value[0].line, 1);
}

#[test]
fn reports_malformed_real_markdown_annotation() {
    let report =
        extract_annotations(Path::new("docs/reference.md"), "<!-- soul id=\"real.id\"\n")
            .expect("annotations");

    assert!(report.value.is_empty());
    assert_eq!(report.diagnostics.len(), 1);
    assert_eq!(report.diagnostics[0].line, Some(1));
    assert!(
        report.diagnostics[0]
            .message
            .contains("HTML comment not closed")
    );
}
```

### 8.4 Scan tests

Update `crates/indexer/src/tests/scan.rs` and add tests for:

- when editing this file, move `std::os::unix::fs::PermissionsExt` behind `#[cfg(unix)]` or into the existing Unix-only test bodies so `cargo test -p indexer` compiles on Windows;
- frontmatter-backed document with wiki link produces one `Reference`;
- frontmatter-less Markdown with exactly one unique real annotation ID produces one `MarkdownSource` and one `Reference`, including the case where multiple real annotations in that file all use the same ID;
- frontmatter-less Markdown with no annotation ignores wiki links and malformed wiki-link diagnostics silently;
- frontmatter-less Markdown with multiple annotation IDs emits a diagnostic and produces no references for that file while preserving annotations;
- frontmatter ID/annotation ID disagreement emits a diagnostic and produces no references for that file while preserving the document and annotations;
- duplicate document ID drops references from the discarded duplicate document path;
- final ordering for annotations, references, markdown sources, and diagnostics.

Make the Windows compile fix explicit:

```rust
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
```

Add these concrete scan tests to `crates/indexer/src/tests/scan.rs`, reusing the existing `test_config_and_registry` helper:

```rust
#[test]
fn frontmatter_document_with_wikilink_produces_reference() {
    let root = tempdir().expect("tempdir");
    fs::create_dir_all(root.path().join(".docs")).expect("docs dir");

    fs::write(
        root.path().join(".docs/source.md"),
        "\
---
id: source.id
kind: concept
---
See [[target.id|Target]].
",
    )
    .expect("source doc");

    let (config, registry) = test_config_and_registry(root.path());
    let graph = scan_repository(root.path(), &config, &registry).expect("scan");

    assert_eq!(graph.documents.len(), 1);
    assert_eq!(graph.references.len(), 1);
    assert!(graph.markdown_sources.is_empty());
    assert_eq!(graph.references[0].source_id, "source.id");
    assert_eq!(graph.references[0].target_id, "target.id");
    assert_eq!(graph.references[0].source_path, PathBuf::from(".docs/source.md"));
    assert_eq!(graph.references[0].source_start_line, 5);
    assert_eq!(graph.references[0].display_text.as_deref(), Some("Target"));
}

#[test]
fn frontmatter_less_markdown_with_one_annotation_id_is_promoted_source() {
    let root = tempdir().expect("tempdir");
    fs::create_dir_all(root.path().join("docs")).expect("docs dir");

    fs::write(
        root.path().join("docs/reference.md"),
        "\
<!-- soul id=\"reference.id\" -->
<!-- soul id=\"reference.id\" layer=\"notes\" -->
See [[target.id]].
",
    )
    .expect("reference doc");

    let (config, registry) = test_config_and_registry(root.path());
    let graph = scan_repository(root.path(), &config, &registry).expect("scan");

    assert!(graph.documents.is_empty());
    assert_eq!(graph.annotations.len(), 2);
    assert_eq!(graph.markdown_sources.len(), 1);
    assert_eq!(graph.markdown_sources[0].source_id, "reference.id");
    assert_eq!(graph.markdown_sources[0].source_path, PathBuf::from("docs/reference.md"));
    assert_eq!(graph.references.len(), 1);
    assert_eq!(graph.references[0].source_id, "reference.id");
    assert_eq!(graph.references[0].target_id, "target.id");
}

#[test]
fn frontmatter_less_markdown_without_annotation_suppresses_wikilink_diagnostics() {
    let root = tempdir().expect("tempdir");
    fs::create_dir_all(root.path().join("docs")).expect("docs dir");
    fs::write(root.path().join("docs/plain.md"), "[[bad id]]\n").expect("plain doc");

    let (config, registry) = test_config_and_registry(root.path());
    let graph = scan_repository(root.path(), &config, &registry).expect("scan");

    assert!(graph.documents.is_empty());
    assert!(graph.annotations.is_empty());
    assert!(graph.references.is_empty());
    assert!(graph.markdown_sources.is_empty());
    assert!(graph.diagnostics.is_empty());
}

#[test]
fn ambiguous_or_conflicting_markdown_source_ids_skip_only_references() {
    let root = tempdir().expect("tempdir");
    fs::create_dir_all(root.path().join("docs")).expect("docs dir");

    fs::write(
        root.path().join("docs/multi.md"),
        "\
<!-- soul id=\"first.id\" -->
<!-- soul id=\"second.id\" -->
[[target.id]]
",
    )
    .expect("multi doc");

    fs::write(
        root.path().join("docs/conflict.md"),
        "\
---
id: doc.id
kind: concept
---
<!-- soul id=\"annotation.id\" -->
[[target.id]]
",
    )
    .expect("conflict doc");

    let (config, registry) = test_config_and_registry(root.path());
    let graph = scan_repository(root.path(), &config, &registry).expect("scan");

    assert_eq!(graph.documents.len(), 1);
    assert_eq!(graph.annotations.len(), 3);
    assert!(graph.references.is_empty());
    assert!(graph.markdown_sources.is_empty());
    assert!(graph.diagnostics.iter().any(|diagnostic| {
        diagnostic
            .message
            .contains("wiki links skipped: multiple markdown annotation ids")
    }));
    assert!(graph.diagnostics.iter().any(|diagnostic| {
        diagnostic.message.contains("wiki links skipped: frontmatter id")
    }));
}

#[test]
fn duplicate_document_id_drops_references_from_discarded_document_path() {
    let root = tempdir().expect("tempdir");
    fs::create_dir_all(root.path().join(".docs/a")).expect("docs a");
    fs::create_dir_all(root.path().join(".docs/b")).expect("docs b");

    fs::write(
        root.path().join(".docs/a/first.md"),
        "\
---
id: duplicate.id
kind: concept
---
[[target.from.first]]
",
    )
    .expect("first");

    fs::write(
        root.path().join(".docs/b/second.md"),
        "\
---
id: duplicate.id
kind: concept
---
[[target.from.second]]
",
    )
    .expect("second");

    let (config, registry) = test_config_and_registry(root.path());
    let graph = scan_repository(root.path(), &config, &registry).expect("scan");

    assert_eq!(graph.documents.len(), 1);
    assert_eq!(graph.documents[0].path, PathBuf::from(".docs/a/first.md"));
    assert_eq!(graph.references.len(), 1);
    assert_eq!(graph.references[0].target_id, "target.from.first");
    assert_eq!(graph.references[0].source_path, PathBuf::from(".docs/a/first.md"));
}

#[test]
fn scan_orders_annotations_references_markdown_sources_and_diagnostics() {
    let root = tempdir().expect("tempdir");
    fs::create_dir_all(root.path().join("docs")).expect("docs dir");

    fs::write(
        root.path().join("docs/b.md"),
        "<!-- soul id=\"b.source\" -->\n[[target.b]]\n",
    )
    .expect("b");
    fs::write(
        root.path().join("docs/a.md"),
        "<!-- soul id=\"a.source\" -->\n[[target.a]]\n",
    )
    .expect("a");
    fs::write(
        root.path().join("docs/y.md"),
        "<!-- soul id=\"y.one\" -->\n<!-- soul id=\"y.two\" -->\n[[target.y]]\n",
    )
    .expect("y");
    fs::write(root.path().join("docs/z.md"), "<!-- soul id=\"broken.id\"\n").expect("z");

    let (config, registry) = test_config_and_registry(root.path());
    let graph = scan_repository(root.path(), &config, &registry).expect("scan");

    let annotation_keys: Vec<_> = graph
        .annotations
        .iter()
        .map(|annotation| (annotation.path.clone(), annotation.line, annotation.id.clone()))
        .collect();
    assert_eq!(
        annotation_keys,
        vec![
            (PathBuf::from("docs/a.md"), 1, "a.source".to_string()),
            (PathBuf::from("docs/b.md"), 1, "b.source".to_string()),
            (PathBuf::from("docs/y.md"), 1, "y.one".to_string()),
            (PathBuf::from("docs/y.md"), 2, "y.two".to_string()),
        ]
    );

    assert_eq!(graph.markdown_sources[0].source_path, PathBuf::from("docs/a.md"));
    assert_eq!(graph.markdown_sources[1].source_path, PathBuf::from("docs/b.md"));
    assert_eq!(graph.references[0].source_path, PathBuf::from("docs/a.md"));
    assert_eq!(graph.references[1].source_path, PathBuf::from("docs/b.md"));
    assert!(graph.diagnostics.len() >= 2);
    assert_eq!(graph.diagnostics[0].path, PathBuf::from("docs/y.md"));
    assert_eq!(graph.diagnostics[1].path, PathBuf::from("docs/z.md"));
}
```

### 8.5 DB tests

Create `crates/indexer/tests/index.rs` if it does not exist. Add async tests for:

- `write_index`/`load_graph` roundtrip with documents, annotations, diagnostics, references, and markdown sources;
- stable load ordering;
- `explain_from_index` returns references for `target_id`;
- promoted annotation-only Markdown source and its references survive roundtrip;
- invalid reference target ID fails `load_graph` and `explain_from_index` visibly;
- invalid display text fails visibly;
- cross-line, reversed, zero-length, negative-column, or zero-line spans fail visibly;
- invalid `markdown_sources` rows fail visibly;
- reference rows with missing `(source_id, source_path)` fail visibly;
- duplicate `doc_references` tuples fail visibly.

Use direct SQL inserts only inside tests that intentionally create corrupt persisted state. For corrupt rows that violate schema `CHECK` constraints before the load-time validator can inspect them, temporarily set `PRAGMA ignore_check_constraints = ON` for the corrupt insert and restore it afterward so the test proves `load_graph`/`explain_from_index` reject the persisted row.

Create `crates/indexer/tests/index.rs` with this concrete shape, then extend the corrupt-row table only by adding more cases to the same helpers:

```rust
use indexer::{
    index::{explain_from_index, load_graph, open_index, write_index},
    model::AnnotationSyntax,
    CodeAnnotation, Diagnostic, DiagnosticSeverity, Document, IndexerError, MarkdownSource,
    Reference, SemanticGraph,
};
use sqlx::SqlitePool;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

fn graph_for_index_tests() -> SemanticGraph {
    SemanticGraph {
        documents: vec![
            Document {
                id: "source.id".to_string(),
                kind: "concept".to_string(),
                title: Some("Source".to_string()),
                path: PathBuf::from("docs/source.md"),
            },
            Document {
                id: "target.id".to_string(),
                kind: "concept".to_string(),
                title: Some("Target".to_string()),
                path: PathBuf::from("docs/target.md"),
            },
        ],
        annotations: vec![CodeAnnotation {
            id: "target.id".to_string(),
            metadata: serde_json::Map::new(),
            path: PathBuf::from("src/lib.rs"),
            line: 12,
            syntax: AnnotationSyntax("rust-attribute".to_string()),
            raw: r#"#[soul(id = "target.id")]"#.to_string(),
        }],
        references: vec![
            Reference {
                source_id: "reference.id".to_string(),
                target_id: "target.id".to_string(),
                source_path: PathBuf::from("docs/reference.md"),
                source_start_line: 3,
                source_start_col: 0,
                source_end_line: 3,
                source_end_col: 13,
                display_text: Some("Target".to_string()),
            },
            Reference {
                source_id: "source.id".to_string(),
                target_id: "target.id".to_string(),
                source_path: PathBuf::from("docs/source.md"),
                source_start_line: 5,
                source_start_col: 4,
                source_end_line: 5,
                source_end_col: 17,
                display_text: None,
            },
        ],
        markdown_sources: vec![MarkdownSource {
            source_id: "reference.id".to_string(),
            source_path: PathBuf::from("docs/reference.md"),
        }],
        diagnostics: vec![Diagnostic {
            severity: DiagnosticSeverity::Error,
            path: PathBuf::from("docs/bad.md"),
            line: Some(1),
            message: "bad markdown".to_string(),
        }],
    }
}

async fn seeded_pool(root: &Path) -> SqlitePool {
    write_index(root, &graph_for_index_tests())
        .await
        .expect("write index");
    open_index(root)
        .await
        .expect("open index")
        .expect("index exists")
}

fn assert_index_corruption(error: IndexerError) {
    assert!(matches!(error, IndexerError::IndexCorruption { .. }));
}

#[tokio::test]
async fn write_index_and_load_graph_roundtrip_references_and_markdown_sources() {
    let root = tempdir().expect("tempdir");
    let pool = seeded_pool(root.path()).await;

    let loaded = load_graph(&pool).await.expect("load graph");

    assert_eq!(loaded.documents, graph_for_index_tests().documents);
    assert_eq!(loaded.annotations, graph_for_index_tests().annotations);
    assert_eq!(loaded.references, graph_for_index_tests().references);
    assert_eq!(loaded.markdown_sources, graph_for_index_tests().markdown_sources);
    assert_eq!(loaded.diagnostics, graph_for_index_tests().diagnostics);
}

#[tokio::test]
async fn explain_from_index_returns_references_for_target_id() {
    let root = tempdir().expect("tempdir");
    let pool = seeded_pool(root.path()).await;

    let result = explain_from_index(&pool, "target.id")
        .await
        .expect("explain");

    assert_eq!(result.documents.len(), 1);
    assert_eq!(result.annotations.len(), 1);
    assert_eq!(result.references.len(), 2);
    assert_eq!(result.references[0].source_path, PathBuf::from("docs/reference.md"));
    assert_eq!(result.references[1].source_path, PathBuf::from("docs/source.md"));
}

async fn ignore_checks(pool: &SqlitePool, enabled: bool) {
    let value = if enabled { "ON" } else { "OFF" };
    sqlx::query(&format!("PRAGMA ignore_check_constraints = {value}"))
        .execute(pool)
        .await
        .expect("set pragma");
}

async fn replace_with_corrupt_reference(
    pool: &SqlitePool,
    source_id: &str,
    target_id: &str,
    source_path: &str,
    start_line: i64,
    start_col: i64,
    end_line: i64,
    end_col: i64,
    display_text: Option<&str>,
) {
    sqlx::query("DELETE FROM doc_references")
        .execute(pool)
        .await
        .expect("delete references");
    ignore_checks(pool, true).await;
    sqlx::query(
        "INSERT INTO doc_references \
         (source_id, target_id, source_path, source_start_line, source_start_col, source_end_line, source_end_col, display_text) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(source_id)
    .bind(target_id)
    .bind(source_path)
    .bind(start_line)
    .bind(start_col)
    .bind(end_line)
    .bind(end_col)
    .bind(display_text)
    .execute(pool)
    .await
    .expect("insert corrupt reference");
    ignore_checks(pool, false).await;
}

#[tokio::test]
async fn corrupt_reference_rows_fail_load_graph_and_explain_visibly() {
    let cases = [
        ("source.id", "bad target", "docs/source.md", 5, 4, 5, 17, None),
        ("source.id", "target.id", "docs/source.md", 5, 4, 6, 17, None),
        ("source.id", "target.id", "docs/source.md", 5, 4, 5, 4, None),
        ("source.id", "target.id", "docs/source.md", 5, -1, 5, 17, None),
        ("source.id", "target.id", "docs/source.md", 0, 4, 0, 17, None),
        ("source.id", "target.id", "/abs/source.md", 5, 4, 5, 17, None),
        ("missing.id", "target.id", "docs/missing.md", 5, 4, 5, 17, None),
    ];

    for case in cases {
        let root = tempdir().expect("tempdir");
        let pool = seeded_pool(root.path()).await;
        replace_with_corrupt_reference(
            &pool, case.0, case.1, case.2, case.3, case.4, case.5, case.6, case.7,
        )
        .await;

        assert_index_corruption(load_graph(&pool).await.expect_err("load should fail"));
        assert_index_corruption(
            explain_from_index(&pool, case.1)
                .await
                .expect_err("explain should fail"),
        );
    }
}

#[tokio::test]
async fn corrupt_display_text_markdown_source_and_duplicate_reference_fail_visibly() {
    let root = tempdir().expect("tempdir");
    let pool = seeded_pool(root.path()).await;
    let overlong_display = "x".repeat(1025);
    replace_with_corrupt_reference(
        &pool,
        "source.id",
        "target.id",
        "docs/source.md",
        5,
        4,
        5,
        17,
        Some(&overlong_display),
    )
    .await;
    assert_index_corruption(load_graph(&pool).await.expect_err("display text"));

    let root = tempdir().expect("tempdir");
    let pool = seeded_pool(root.path()).await;
    ignore_checks(&pool, true).await;
    sqlx::query("INSERT INTO markdown_sources (source_id, source_path) VALUES ('source.id', '/abs/bad.md')")
        .execute(&pool)
        .await
        .expect("insert corrupt source");
    ignore_checks(&pool, false).await;
    assert_index_corruption(load_graph(&pool).await.expect_err("markdown source"));

    let root = tempdir().expect("tempdir");
    let pool = seeded_pool(root.path()).await;
    sqlx::query(
        "INSERT INTO doc_references \
         (source_id, target_id, source_path, source_start_line, source_start_col, source_end_line, source_end_col, display_text) \
         VALUES ('source.id', 'target.id', 'docs/source.md', 5, 4, 5, 17, NULL)",
    )
    .execute(&pool)
    .await
    .expect("insert duplicate");
    assert_index_corruption(load_graph(&pool).await.expect_err("duplicate reference"));
}
```

### 8.6 Explain/MCP/CLI tests

Update `crates/indexer/src/tests/graph.rs` for live explain references.

Follow the repo's existing unit-test organization: add `pub mod mcp;` to `crates/indexer/src/tests/mod.rs` and put MCP unit tests in a new `crates/indexer/src/tests/mcp.rs`. Do not add inline `#[cfg(test)] mod tests` blocks to `crates/indexer/src/mcp/format.rs` or use `use super::*`.

Add unit tests in `crates/indexer/src/tests/mcp.rs` for:

- `Referenced by` section renders when references exist;
- references alone prevent the empty-result message;
- diagnostics still render when documents, annotations, and references are empty;
- line breaks, tabs, control characters, Markdown metacharacters, and backticks in display text and other fields cannot forge headings, bullets, or code spans.

Update `crates/indexer/tests/explain.rs` for CLI output:

- explain includes `Referenced by` rows;
- missing ID output includes `Referenced by:\nnone` and the empty-message line;
- diagnostics still render.

Add or update tests for CLI and MCP `soul_index` count text so references are included.

Add concrete MCP formatter tests in `crates/indexer/src/tests/mcp.rs`:

```rust
use crate::{
    graph::ExplainResult,
    mcp::format::explain_result,
    model::{Diagnostic, DiagnosticSeverity, Reference},
};
use std::path::PathBuf;

fn reference(display_text: Option<&str>) -> Reference {
    Reference {
        source_id: "source.id".to_string(),
        target_id: "target.id".to_string(),
        source_path: PathBuf::from("docs/source.md"),
        source_start_line: 5,
        source_start_col: 4,
        source_end_line: 5,
        source_end_col: 17,
        display_text: display_text.map(str::to_string),
    }
}

#[test]
fn referenced_by_section_renders_and_prevents_empty_message() {
    let result = ExplainResult {
        id: "target.id".to_string(),
        documents: Vec::new(),
        annotations: Vec::new(),
        references: vec![reference(Some("Target"))],
        scan_diagnostics: Vec::new(),
    };

    let output = explain_result(&result);

    assert!(output.contains("## Referenced by"));
    assert!(output.contains("source.id"));
    assert!(output.contains("docs/source.md"));
    assert!(output.contains("5:4-17"));
    assert!(output.contains("Target"));
    assert!(!output.contains("No documents, annotations, or references found"));
}

#[test]
fn diagnostics_render_even_when_explain_collections_are_empty() {
    let result = ExplainResult {
        id: "missing.id".to_string(),
        documents: Vec::new(),
        annotations: Vec::new(),
        references: Vec::new(),
        scan_diagnostics: vec![Diagnostic {
            severity: DiagnosticSeverity::Error,
            path: PathBuf::from("docs/bad.md"),
            line: Some(1),
            message: "bad markdown".to_string(),
        }],
    };

    let output = explain_result(&result);

    assert!(output.contains("No documents, annotations, or references found"));
    assert!(output.contains("docs/bad.md"));
    assert!(output.contains("bad markdown"));
}

#[test]
fn display_data_cannot_forge_markdown_structure() {
    let result = ExplainResult {
        id: "target.id".to_string(),
        documents: Vec::new(),
        annotations: Vec::new(),
        references: vec![reference(Some("line\n## forged\t`code`"))],
        scan_diagnostics: Vec::new(),
    };

    let output = explain_result(&result);

    assert!(output.contains("line\\n"));
    assert!(output.contains("forged"));
    assert!(!output.contains("\n## forged"));
    assert!(!output.contains("`code`"));
}
```

Update `crates/indexer/tests/explain.rs` expected output by adding a source document with a wiki link and asserting the exact CLI reference section. The reference row format is part of this plan:

```rust
fs::write(
    root.path().join(".docs/interactions/flow.md"),
    "\
---
id: interaction.checkout.flow
kind: interaction
title: Checkout flow
---
See [[interaction.checkout.create-order|Create order]].
",
)
.expect("flow doc");
```

The `explain_command_prints_matches_and_diagnostics` expected stdout must include `Referenced by` between `Annotations` and `Diagnostics`:

```rust
let expected = "\
ID: interaction.checkout.create-order

Documents:
- .docs/interactions/checkout.md [kind=interaction, title=Create order]

Annotations:
- fixtures/backend.rs:3
- fixtures/frontend.cs:5

Referenced by:
- .docs/interactions/flow.md:6:4-54 -> interaction.checkout.flow [display=Create order]

Diagnostics:
- .docs/interactions/bad.md frontmatter block is missing a closing `---` delimiter
- fixtures/bad.rs:1 malformed soul attribute
- fixtures/invalid_utf8.rs file is not valid UTF-8
";
assert_eq!(stdout, expected);
```

The missing-ID CLI expected stdout must include an empty reference section:

```rust
let expected = "\
ID: interaction.checkout.missing

Documents:
none

Annotations:
none

Referenced by:
none

No documents, annotations, or references found for this ID.
";
assert_eq!(stdout, expected);
```

Add an index-count integration test for `crates/indexer/src/bin/commands/index.rs`:

```rust
#[test]
fn index_command_prints_reference_count() {
    let root = tempdir().expect("tempdir");
    write_test_config(root.path());
    fs::create_dir_all(root.path().join(".docs")).expect("docs dir");
    fs::write(
        root.path().join(".docs/source.md"),
        "\
---
id: source.id
kind: concept
---
[[target.id]]
",
    )
    .expect("source doc");

    let output = Command::new(env!("CARGO_BIN_EXE_indexer"))
        .args(["index", "--root", root.path().to_str().expect("root path")])
        .output()
        .expect("run indexer");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout");
    assert!(stdout.contains("Indexed 1 documents, 0 annotations, 1 references, 0 diagnostics"));
}
```

For MCP count coverage, change the existing `SoulServer::soul_index_impl` method signature from `async fn soul_index_impl(&self) -> IndexerResult<CallToolResult>` to `pub(crate) async fn soul_index_impl(&self) -> IndexerResult<CallToolResult>` so the repo-pattern test module can call it. Keep the existing method body and update only the count text described in section 6.2.

Add a matching MCP count test to `crates/indexer/src/tests/mcp.rs` so `soul_index_impl` cannot drift from the CLI count text. Reuse the explicit fixture-writing style from the existing CLI integration tests; do not add an inline test module in `mcp/mod.rs`:

```rust
use crate::mcp::SoulServer;
use std::{fs, path::Path};
use tempfile::tempdir;

fn write_mcp_test_config(root: &Path) {
    let soul_dir = root.join(".soul");
    fs::create_dir_all(&soul_dir).expect(".soul dir");
    let plugins = crate::tests::plugin_helper::test_plugin_entries();
    let mut config = "\
[scan]
excluded_dirs = [\".git\", \".soul\", \"target\", \".idea\", \".vscode\", \".vs\", \".codex\", \"node_modules\", \"obj\"]
excluded_dir_suffixes = [\"Tests\", \".Tests\"]
excluded_bin_except_under = [\"src\"]
"
    .to_string();
    for plugin in plugins {
        config.push_str(&format!(
            "\n[[plugins]]\nlanguage = \"{}\"\npath = '{}'\n",
            plugin.language,
            plugin.path.display()
        ));
    }
    fs::write(soul_dir.join("soul.toml"), config).expect("soul.toml");
}

#[tokio::test]
async fn soul_index_impl_prints_reference_count() {
    let root = tempdir().expect("tempdir");
    write_mcp_test_config(root.path());
    fs::create_dir_all(root.path().join(".docs")).expect("docs dir");
    fs::write(
        root.path().join(".docs/source.md"),
        "\
---
id: source.id
kind: concept
---
[[target.id]]
",
    )
    .expect("source doc");

    let server = SoulServer::new(root.path()).expect("server");
    let result = server.soul_index_impl().await.expect("index");
    let text = result.content[0].as_text().expect("text").text.as_str();

    assert_eq!(
        text,
        "Indexed 1 documents, 0 annotations, 1 references, 0 diagnostics."
    );
}
```

### 8.7 LSP tests

Add LSP helper/unit tests around pure helper behavior where possible:

- cached open Markdown text is preferred over disk text for token lookup;
- wiki-link go-to-definition resolves to target document when present;
- wiki-link go-to-definition falls back to annotation locations when no document exists;
- hover on a wiki link includes target document kind/title/path;
- LSP hover escaping renders line breaks, tabs, control characters, Markdown metacharacters, and backticks in IDs, titles, kinds, paths, and annotation locations without forging Markdown structure;
- references on a wiki link return both code annotations and wiki-link reference locations;
- stored UTF-16 columns are used directly for returned wiki-link ranges.

If no existing LSP integration harness exists, keep these as focused server/helper tests and do not introduce a broad new test framework.

Required helper-level test targets in `crates/soul-lsp/src/tests.rs`:

- `wikilink_lookup_text` proves cached open text is preferred over disk text, and `wikilink_token_from_text` uses that helper;
- `definition_response_for_id` proves document-first and annotation fallback behavior;
- `hover_markdown_for_wikilink_target` proves kind/title/path/count rendering and escaping;
- `reference_locations_for_id` proves stored UTF-16 columns are used directly;
- `resolved_id_from_position_inputs` proves wiki-link target ID wins over annotation/document fallback.

Follow the repo's separate-test-file style. Do not add an inline `#[cfg(test)] mod tests` block to `server.rs` and do not use `use super::*`. Add this test module declaration to `crates/soul-lsp/src/main.rs`:

```rust
#[cfg(test)]
mod tests;
```

Create `crates/soul-lsp/src/tests.rs` with concrete helper tests:

```rust
use crate::server::{
    definition_response_for_id, hover_markdown_for_wikilink_target, reference_locations_for_id,
    resolved_id_from_position_inputs, wikilink_lookup_text, wikilink_token_from_text,
};
use indexer::{
    model::AnnotationSyntax, CodeAnnotation, Document, Reference, SemanticGraph, WikiLinkToken,
};
use std::path::PathBuf;
use tower_lsp_server::ls_types::{GotoDefinitionResponse, Position, Uri};

fn file_uri(path: PathBuf) -> Uri {
    let path = path.display().to_string().replace('\\', "/");
    let path = path.trim_start_matches('/');
    format!("file:///{}", path).parse().expect("uri")
}

fn graph() -> SemanticGraph {
    SemanticGraph {
        documents: vec![Document {
            id: "target.id".to_string(),
            kind: "concept".to_string(),
            title: Some("Target title".to_string()),
            path: PathBuf::from("docs/target.md"),
        }],
        annotations: vec![CodeAnnotation {
            id: "target.id".to_string(),
            metadata: serde_json::Map::new(),
            path: PathBuf::from("src/lib.rs"),
            line: 7,
            syntax: AnnotationSyntax("rust-attribute".to_string()),
            raw: r#"#[soul(id = "target.id")]"#.to_string(),
        }],
        references: vec![Reference {
            source_id: "source.id".to_string(),
            target_id: "target.id".to_string(),
            source_path: PathBuf::from("docs/source.md"),
            source_start_line: 3,
            source_start_col: 5,
            source_end_line: 3,
            source_end_col: 17,
            display_text: None,
        }],
        markdown_sources: Vec::new(),
        diagnostics: Vec::new(),
    }
}

#[test]
fn wikilink_lookup_text_prefers_cached_text() {
    let root = std::env::current_dir().expect("cwd");
    let uri = file_uri(root.join("missing.md"));

    let text = wikilink_lookup_text(Some("cached [[target.id]]".to_string()), &uri)
        .expect("cached text");

    assert_eq!(text, "cached [[target.id]]");
}

#[test]
fn wikilink_token_from_text_uses_cached_text() {
    let root = std::env::current_dir().expect("cwd");
    let uri = file_uri(root.join("missing.md"));
    let token = wikilink_token_from_text(
        Some("See [[target.id]].".to_string()),
        &uri,
        Position {
            line: 0,
            character: 6,
        },
    )
    .expect("token");

    assert_eq!(token.target_id, "target.id");
}

#[test]
fn definition_response_for_id_prefers_document_then_annotations() {
    let root = std::env::current_dir().expect("cwd");
    let graph = graph();

    let response =
        definition_response_for_id(&graph, &root, "target.id").expect("document response");
    assert!(matches!(response, GotoDefinitionResponse::Scalar(_)));

    let response = definition_response_for_id(&graph, &root, "annotation.only");
    assert!(response.is_none());

    let mut fallback_graph = graph;
    fallback_graph.documents.clear();
    fallback_graph.annotations[0].id = "annotation.only".to_string();
    let response = definition_response_for_id(&fallback_graph, &root, "annotation.only")
        .expect("annotation response");
    match response {
        GotoDefinitionResponse::Array(locations) => assert_eq!(locations.len(), 1),
        other => panic!("expected annotation array, got {other:?}"),
    }
}

#[test]
fn hover_markdown_for_wikilink_target_renders_and_escapes_fields() {
    let mut graph = graph();
    graph.documents[0].title = Some("Target\n# forged `title`".to_string());
    graph.documents[0].kind = "concept|table".to_string();
    graph.documents[0].path = PathBuf::from("docs/target`doc`.md");

    let markdown = hover_markdown_for_wikilink_target(&graph, "target.id").expect("hover");

    assert!(markdown.contains("target\\.id"));
    assert!(markdown.contains("concept\\|table"));
    assert!(markdown.contains("Target\\n\\# forged \\`title\\`"));
    assert!(markdown.contains("docs/target\\`doc\\`\\.md"));
    assert!(!markdown.contains("\n# forged"));
}

#[test]
fn reference_locations_for_id_uses_stored_utf16_columns_directly() {
    let root = std::env::current_dir().expect("cwd");
    let locations = reference_locations_for_id(&graph(), &root, "target.id");

    assert_eq!(locations.len(), 1);
    assert_eq!(locations[0].range.start.line, 2);
    assert_eq!(locations[0].range.start.character, 5);
    assert_eq!(locations[0].range.end.line, 2);
    assert_eq!(locations[0].range.end.character, 17);
}

#[test]
fn resolved_id_from_position_inputs_prefers_wikilink_token() {
    let root = std::env::current_dir().expect("cwd");
    let uri = file_uri(root.join("docs/target.md"));
    let token = WikiLinkToken {
        target_id: "wikilink.target".to_string(),
        start_line: 1,
        start_col: 0,
        end_line: 1,
        end_col: 17,
        display_text: None,
    };

    let resolved = resolved_id_from_position_inputs(
        &graph(),
        &root,
        &uri,
        Position {
            line: 0,
            character: 4,
        },
        Some(token),
    );

    assert_eq!(resolved.as_deref(), Some("wikilink.target"));
}
```

### 8.8 Verification commands

Run, in order:

```text
cargo fmt
cargo test -p indexer
cargo test -p soul-lsp
cargo test
```

If a command fails, fix the implementation or the tests before proceeding to the next command.

## 9. Files touched

| File | Action |
|------|--------|
| `crates/indexer/src/model/reference.rs` | **Create** — persisted/reference graph row |
| `crates/indexer/src/model/markdown_source.rs` | **Create** — promoted annotation-only Markdown source row |
| `crates/indexer/src/model/wiki_link_token.rs` | **Create** — parsed wiki-link token shared by parser and LSP |
| `crates/indexer/src/model/mod.rs` | **Edit** — add modules and re-exports |
| `crates/indexer/src/lib.rs` | **Edit** — re-export public model types with public graph fields |
| `crates/indexer/src/model/semantic_graph.rs` | **Edit** — add `references` and `markdown_sources` |
| `crates/indexer/src/markdown/frontmatter_block.rs` | **Edit** — carry body slice and body start line |
| `crates/indexer/src/markdown/markdown_parse.rs` | **Create** — crate-private parse payload |
| `crates/indexer/src/markdown/scanned_markdown_line.rs` | **Create** — scanned line result for shared Markdown visibility |
| `crates/indexer/src/markdown/fence_state.rs` | **Create** — fenced-code-block state for Markdown visibility scanner |
| `crates/indexer/src/markdown/markdown_line_scanner.rs` | **Create** — shared Markdown visibility state |
| `crates/indexer/src/markdown/wikilink_validation.rs` | **Create** — shared target/display validation helpers |
| `crates/indexer/src/markdown/wikilinks.rs` | **Create** — wiki-link parser |
| `crates/indexer/src/markdown/annotations.rs` | **Edit** — use shared visibility scanner, preserve annotation parsing |
| `crates/indexer/src/markdown/mod.rs` | **Edit** — crate-private `parse_markdown`, new modules, public LSP token lookup |
| `crates/indexer/src/scan/mod.rs` | **Edit** — stage Markdown parse/annotation data, build references/sources |
| `crates/indexer/src/index/mod.rs` | **Edit** — persist/load/validate references and markdown sources |
| `crates/indexer/src/error/mod.rs` | **Edit** — add corrupt-index error handling |
| `crates/indexer/migrations/0002_doc_references.up.sql` | **Create** — reference schema |
| `crates/indexer/migrations/0003_markdown_sources.up.sql` | **Create** — promoted source schema |
| `crates/indexer/src/graph/mod.rs` | **Edit** — include references in live explain |
| `crates/indexer/src/graph/explain_result.rs` | **Edit** — add references field |
| `crates/indexer/src/mcp/format.rs` | **Edit** — render references, diagnostics, and escaped Markdown display data |
| `crates/indexer/src/mcp/mod.rs` | **Edit** — update descriptions, index count text, and `pub(crate)` test access for `soul_index_impl` |
| `crates/indexer/src/bin/commands/explain.rs` | **Edit** — render references in CLI explain |
| `crates/indexer/src/bin/commands/index.rs` | **Edit** — include reference count |
| `crates/soul-lsp/src/server.rs` | **Edit** — open-doc sync, shared wiki-link token lookup, navigation |
| `crates/indexer/src/tests/markdown.rs` | **Edit** — parser/frontmatter/wiki-link tests |
| `crates/indexer/src/tests/scan.rs` | **Edit** — scan/source-ID/dedupe tests |
| `crates/indexer/src/tests/graph.rs` | **Edit** — explain-result tests |
| `crates/indexer/src/tests/mcp.rs` | **Create** — MCP format/count tests using repo test-module pattern |
| `crates/indexer/src/tests/mod.rs` | **Edit** — add `pub mod mcp;` |
| `crates/indexer/tests/index.rs` | **Create** — DB roundtrip/corruption tests |
| `crates/indexer/tests/explain.rs` | **Edit** — CLI explain output tests |
| `crates/soul-lsp/src/tests.rs` | **Create** — LSP helper tests using repo separate-test-file pattern |
| `crates/soul-lsp/src/main.rs` | **Edit** — add `#[cfg(test)] mod tests;` |

## 10. Out of scope for first pass

- Plain `.md` files with no frontmatter and no Soul ID annotation as wiki-link sources.
- `soul_list_gaps` integration for broken wiki links.
- Backlink search tools beyond `soul_explain` and LSP references.
- Editor auto-completion for `[[...]]`.
- Incremental LSP text sync.
