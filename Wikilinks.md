# Wikilinks — Cross-Doc Soul ID References

## Problem

A Soul doc can link to code via `#[soul(...)]` annotations, and the LSP navigates from code to the Soul doc. But there is no way for a Soul doc to reference **another Soul doc or reference doc** by ID and have the LSP resolve that reference.

Example broken chain:

```
Rust code  →  Soul doc  →  ? (no way to navigate to reference doc)
```

The reference doc (`docs/doc-references.md`) is annotated with `<!-- soul id="..." -->` and participates in `soul_explain`, but the Soul doc body can only contain plain Markdown links that the LSP ignores.

## Proposed solution

Add wiki link syntax `[[soul.id]]` (and optionally `[[soul.id|display text]]`) to Soul doc bodies. The indexer parses these during scans and stores them as structured references. The LSP resolves them to offer go-to-definition and find-references across docs.

## Scope of work

### 1. New model — `Reference`

**File:** `crates/indexer/src/model/reference.rs` (new)

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reference {
    pub source_id: String,       // the doc that contains the [[link]]
    pub target_id: String,       // the ID being linked to
    pub source_path: PathBuf,    // file containing the link
    pub source_start_line: usize, // 1-based start position
    pub source_start_col: usize,
    pub source_end_line: usize,   // 1-based end position
    pub source_end_col: usize,
    pub display_text: Option<String>, // optional [[id|custom text]]
}
```

Use 1-based source lines and 0-based columns for stored spans, with columns measured in UTF-16 code units so the LSP can turn them into `Position` values without guesswork. Keep the span half-open. When wiki links are extracted from a Markdown body, translate the body-relative positions back to the original file coordinates before constructing each `Reference`; do not persist body-relative line numbers.

`source_path` must follow the same repository-relative display-path convention as `Document.path` and `CodeAnnotation.path` during scans, not an absolute filesystem path. Put the reusable target/display invariants next to `Reference` so the Markdown scanner and SQLite loaders can share them instead of drifting:

- `pub(crate) const MAX_REFERENCE_DISPLAY_BYTES: usize = 1024;`
- `pub(crate) fn is_valid_reference_target_id(input: &str) -> bool`
- `pub(crate) fn normalize_reference_display_text(input: &str) -> Result<Option<String>, String>`

Add `pub mod reference;` and `pub use reference::Reference;` to `crates/indexer/src/model/mod.rs`.
Also add `Reference` to the crate-root re-exports in `crates/indexer/src/lib.rs` so the public API stays aligned with the other model types.

Add `references: Vec<Reference>` to `SemanticGraph`.
Update any existing `SemanticGraph` struct literals and tests to populate the new field.

### 2. Graph — Store references

**File:** `crates/indexer/src/model/semantic_graph.rs`

```rust
pub struct SemanticGraph {
    pub documents: Vec<Document>,
    pub annotations: Vec<CodeAnnotation>,
    pub references: Vec<Reference>,
    pub diagnostics: Vec<Diagnostic>,
}
```

### 3. Parser — Extract `[[id]]` from doc bodies

**File:** `crates/indexer/src/markdown/wikilinks.rs` (new)

- After frontmatter extraction in `parse_markdown`, pass the body text plus the body's original starting line to a new `extract_wikilinks` function so the parser can emit file-relative spans rather than body-relative spans
- Return extracted wiki-link candidates to the scan layer as part of a dedicated markdown parse payload (`document`, `wiki_links`, `diagnostics`) rather than overloading `ParseReport<Option<Document>>`. The parser should not finalize `source_id` yet; `scan_repository` resolves it after it knows whether the file has frontmatter or exactly one Soul ID annotation, then appends the finalized `Reference` rows to `SemanticGraph`
- Keep the extracted wiki-link hits in a deterministic order before persistence/output so explain results and DB roundtrips stay stable
- Scan for `[[...]]` patterns with a small stateful parser that records exact source spans; regex alone is not enough for escaped brackets, nested brackets, or multiple links on one line
- Ignore wiki-link-looking text inside fenced code blocks, indented code blocks, inline code spans, and HTML comments so examples and hidden markup do not become backlinks
- For each match:
  - Split on the first `|` to separate ID from optional display text, then trim both parts; an empty trimmed target is malformed, the target must be non-empty after trimming, contain no ASCII or Unicode whitespace, contain no control characters, and contain no `[` `]` or `|` characters; an empty trimmed display text normalizes to `None`
  - Emit a wiki-link candidate with the target ID, optional display text, and exact source span; the scan layer fills `source_id` once it knows the source context
- Markdown files with frontmatter use the frontmatter `id` as the authoritative `source_id`. Markdown files without frontmatter can still be sources when they carry exactly one real Soul ID annotation outside fenced or indented example blocks; use that annotation ID as `source_id` so reference docs can also emit backlinks. If a frontmatter-less file has no real Soul ID annotation, ignore its wiki links. If the file has multiple real Soul ID annotations or the frontmatter ID and annotation ID disagree, skip it with a diagnostic. Use the same block-state Markdown walker for wiki links and Markdown comment annotations so fenced or indented examples do not count as real annotations and do not disqualify reference docs.
- Stage each Markdown file's document, annotation, wiki-link, and source-id decision in a local per-file buffer before mutating `SemanticGraph`; only append the buffered rows after source-id validation passes so a skipped file cannot leak a partial document, annotation set, or backlink set into the graph.
- Persist promoted annotation-only Markdown sources in a small source table keyed by `(source_id, source_path)` so DB loads can validate `doc_references` against both frontmatter-backed documents and promoted Markdown sources after a round trip. Without that persisted source layer, the `load_graph` validation for annotation-only Markdown backlinks is impossible to enforce.

Add module declaration and re-export in `crates/indexer/src/markdown/mod.rs`.
Update `crates/indexer/src/scan/mod.rs` to consume the new markdown parse payload and append `references` to `SemanticGraph`.
After document deduping, retain references only for surviving document paths and any accepted annotation-only Markdown sources that were promoted to a source ID, so duplicate docs cannot leak backlinks from discarded source files.
Sort `graph.annotations` by `path`, `line`, and `id`; sort `graph.references` by `source_path`, `source_start_line`, and `source_start_col`; and sort `graph.diagnostics` by `path`, `line`, and `message`, before returning so live scans and indexed reads preserve the same order.

### 4. Index DB — Persist references

**File:** `crates/indexer/src/index/mod.rs`

**Migrations:** `crates/indexer/migrations/0002_doc_references.up.sql`, `crates/indexer/migrations/0003_markdown_sources.up.sql`

- Add a `doc_references` table: `(id INTEGER PRIMARY KEY, source_id TEXT NOT NULL CHECK (length(trim(source_id)) > 0), target_id TEXT NOT NULL CHECK (length(trim(target_id)) > 0), source_path TEXT NOT NULL CHECK (length(trim(source_path)) > 0), source_start_line INTEGER NOT NULL, source_start_col INTEGER NOT NULL, source_end_line INTEGER NOT NULL, source_end_col INTEGER NOT NULL, display_text TEXT)`
- Enforce single-line, half-open spans in the table contract as well: `source_start_line` and `source_end_line` must match, `source_start_col` must be non-negative, and `source_end_col` must be strictly greater than `source_start_col` so a persisted reference cannot load as a reversed or zero-length range
- Add a companion `markdown_sources` table for promoted annotation-only Markdown sources: `(source_id TEXT NOT NULL CHECK (length(trim(source_id)) > 0), source_path TEXT NOT NULL CHECK (length(trim(source_path)) > 0), PRIMARY KEY (source_id, source_path))`
- Add `pub mod markdown_source;` and `pub use markdown_source::MarkdownSource;` to `crates/indexer/src/model/mod.rs` so the new row model is actually available to the index layer
- Add a dedicated index-corruption error variant/constructor in `crates/indexer/src/error/mod.rs` and use it for invalid persisted reference/source rows instead of falling back to generic SQLx errors or scan diagnostics
- Write references during `write_index`
- Write promoted Markdown source rows during `write_index`
- Clear `doc_references` during `write_index` before inserting the new rows so re-indexes do not accumulate stale references
- Clear `markdown_sources` during `write_index` before inserting the new rows so re-indexes do not accumulate stale promoted source rows
- Read documents, annotations, diagnostics, and references during `load_graph`, and load promoted Markdown source rows into a validation set; return each returned collection in a stable `ORDER BY` order (`documents`: `path, id`; `annotations`: `path, line, id`; `diagnostics`: `path, line, message`; `references`: `source_path, source_start_line, source_start_col, target_id`) and use the same ordering in `explain_from_index` so explain output and LSP results do not reshuffle across DB roundtrips
- Load `doc_references` through a shared row-to-model helper that validates persisted IDs, source paths, target-ID grammar, display-text invariants, and integer ranges before constructing `Reference`; fail visibly on corrupt rows instead of silently coercing them, and use the same helper from both `load_graph` and `explain_from_index`
- Load promoted Markdown source rows through a shared row-to-model helper as well, and require each loaded `doc_references` row to resolve back to either a frontmatter-backed document row or a promoted Markdown source row with the same `(source_id, source_path)` pair. A backlink that no longer has a valid source is corrupt persisted state and should fail the load visibly rather than surfacing an orphaned reference
- Reject duplicate `doc_references` rows for the same persisted source/target/range/display tuple in the shared reference-row loader so corrupt persisted backlinks do not render twice or shadow later validation

### 5. Explain — Surface references in `soul_explain`

**Files:** `crates/indexer/src/graph/mod.rs`, `crates/indexer/src/graph/explain_result.rs`, `crates/indexer/src/mcp/format.rs`, `crates/indexer/src/mcp/mod.rs`, `crates/indexer/src/bin/commands/explain.rs`

- When explaining an ID, also return all references where `target_id` matches
- Extend `ExplainResult` with a `references` field and populate it from both the live graph and the indexed DB path
- Show them as a "Referenced by" section in the output
- Render `scan_diagnostics` in `crates/indexer/src/mcp/format.rs` beneath the reference section so the MCP response still shows warnings when documents, annotations, and references are all empty
- Treat wiki-link display text, IDs, titles, kinds, paths, annotation metadata, and diagnostic messages as untrusted display data when rendering CLI, MCP, and LSP output so local Markdown cannot forge rows, headings, or code spans
- Update the empty-result guard so references alone still render, and keep scan diagnostics visible even when documents, annotations, and references are all empty; only print the "No documents, annotations, or references found" message when all three collections are empty
- Update the `soul_explain` tool description in `crates/indexer/src/mcp/mod.rs` so the MCP contract text names wiki-link references alongside documents and annotations, including the fallback sentence that says no documents, annotations, or references were found
- Update the index summaries in both `crates/indexer/src/bin/commands/index.rs` and `crates/indexer/src/mcp/mod.rs` so the count sentence includes references alongside documents, annotations, and diagnostics
- Update the `soul_index` tool description in `crates/indexer/src/mcp/mod.rs` so it matches the new four-part count sentence

### 6. LSP — Navigate wiki links

**File:** `crates/soul-lsp/src/server.rs`

- Add `textDocumentSync` plus an open-document cache so the server can resolve `[[...]]` spans from the current Markdown buffer; use the cached text from `didOpen`/`didChange`/`didClose` when mapping positions for go-to-definition, hover, and references
- On go-to-definition for a `[[id]]` token, resolve to the target Soul doc (or the list of all annotated locations if no doc exists)
- On hover, show the target doc's title and kind
- On references requests, return both code annotations and wiki-link locations for the resolved target ID

### 7. Parser edge cases

- `[[id]]` — basic form
- `[[id|Display text]]` — with display text
- `[[id1]] [[id2]]` — multiple on one line
- Escaped brackets: `\[[not a link]]` — must be ignored
- Nested brackets: `[[outer [[inner]]]]` — parse inner only
- Malformed: `[[unclosed`, `[[]]`, `[[|text]]` — emit diagnostics

## Validation

- Parser unit tests for basic, escaped, nested, multiple-link, and malformed cases
- Parser tests for invalid target IDs and whitespace-only display text normalization
- Parser test for a wiki link that appears after frontmatter and after at least one non-BMP character, so the stored span proves the frontmatter offset and UTF-16 column conversion are both correct
- Parser/scan test for a frontmatter-less `.md` source with exactly one Soul ID annotation, confirming its wiki links survive once the scan layer resolves `source_id`
- Parser/scan test for a Markdown source with conflicting Soul IDs, including frontmatter/annotation disagreement, confirming the file is skipped with a diagnostic and its wiki links are not indexed
- Parser/scan test for a frontmatter-less `.md` source whose fenced code examples contain illustrative `<!-- soul ... -->` snippets, confirming source-id inspection ignores those examples
- Parser tests for ignored fenced code blocks, indented code blocks, inline code spans, and HTML comments
- DB roundtrip tests for `write_index`/`load_graph` with `doc_references`, including stable document, annotation, and reference ordering, with annotation ties broken by `id` (`crates/indexer/tests/index.rs`)
- DB roundtrip test proving a promoted annotation-only Markdown source and its references survive `write_index`/`load_graph` together, not just frontmatter-backed documents
- DB corruption tests proving invalid `doc_references` rows fail `load_graph`/`explain_from_index` visibly instead of being silently coerced
- DB corruption tests proving invalid `doc_references` rows with cross-line, reversed, or zero-length spans fail `load_graph`/`explain_from_index` visibly instead of loading as bogus ranges
- DB corruption tests proving invalid `markdown_sources` rows fail `load_graph`/`explain_from_index` visibly instead of creating orphaned promoted sources
- DB corruption tests proving mismatched `(source_id, source_path)` pairs fail `load_graph`/`explain_from_index` visibly instead of creating orphaned backlinks
- DB corruption tests proving duplicate `doc_references` rows for the same source/target/range/display tuple fail `load_graph`/`explain_from_index` visibly instead of duplicating backlinks
- Scan tests for duplicate document containment so references from discarded duplicate docs are dropped with the duplicate document
- Explain output tests for the new "Referenced by" section in MCP output (`crates/indexer/src/mcp/format.rs` unit tests) and CLI output (`crates/indexer/tests/explain.rs`)
- MCP output test proving diagnostics still render when an ID has no documents, annotations, or references but the scan produced diagnostics
- Explain output tests proving line breaks, tabs, and control characters in wiki-link display text and other untrusted fields do not break CLI, MCP, or LSP output structure
- Index summary tests proving both CLI and MCP `soul_index` output include the reference count
- LSP smoke checks for wiki links, go-to-definition, hover, and references on a `[[...]]` token; if live document sync is part of the first pass, open the file and send the current buffer text before asserting positions

## Files touched

| File | Action |
|------|--------|
| `crates/indexer/src/model/reference.rs` | **Create** — `Reference` struct |
| `crates/indexer/src/model/markdown_source.rs` | **Create** — promoted Markdown source row model |
| `crates/indexer/src/model/mod.rs` | **Edit** — add modules + re-exports for `Reference` and `MarkdownSource` |
| `crates/indexer/src/lib.rs` | **Edit** — re-export `Reference` from the crate root |
| `crates/indexer/src/model/semantic_graph.rs` | **Edit** — add `references` field |
| `crates/indexer/src/markdown/wikilinks.rs` | **Create** — wiki link parser |
| `crates/indexer/src/markdown/annotations.rs` | **Edit** — share the block-state Markdown walker so annotation counting matches wiki-link parsing |
| `crates/indexer/src/markdown/mod.rs` | **Edit** — declare module, call parser |
| `crates/indexer/src/scan/mod.rs` | **Edit** — collect wiki references into the graph |
| `crates/indexer/src/index/mod.rs` | **Edit** — persist references |
| `crates/indexer/src/error/mod.rs` | **Edit** — add index-corruption error handling for invalid persisted rows |
| `crates/indexer/migrations/0002_doc_references.up.sql` | **Create** — persist reference schema |
| `crates/indexer/migrations/0003_markdown_sources.up.sql` | **Create** — persist promoted Markdown source rows |
| `crates/indexer/src/graph/mod.rs` | **Edit** — include references in explain output |
| `crates/indexer/src/graph/explain_result.rs` | **Edit** — add references to explain payload |
| `crates/indexer/src/mcp/format.rs` | **Edit** — render "Referenced by" in MCP output and add unit tests |
| `crates/indexer/src/mcp/mod.rs` | **Edit** — update tool descriptions to mention references and the reference count |
| `crates/indexer/src/bin/commands/explain.rs` | **Edit** — render "Referenced by" in CLI output |
| `crates/indexer/src/bin/commands/index.rs` | **Edit** — include reference count in the index summary |
| `crates/soul-lsp/src/server.rs` | **Edit** — navigate wiki links |
| `crates/indexer/tests/explain.rs` | **Edit** — update CLI explain output assertions |
| `crates/indexer/tests/index.rs` | **Create** — DB roundtrip coverage for `write_index`/`load_graph` with `doc_references` |
| `crates/indexer/src/tests/markdown.rs` | **Edit** — update parser assertions for the new payload and link cases |
| `crates/indexer/src/tests/scan.rs` | **Edit** — add duplicate-source containment tests |
| `crates/indexer/src/tests/graph.rs` | **Edit** — update `SemanticGraph` literals and explain-result assertions |

## Out of scope (first pass)

- Plain `.md` files with no frontmatter and no Soul ID annotation
- Backlinks index (requires bidirectional query — can be added later)
- `soul_list_gaps` integration for broken wiki links (dead references)
- Auto-completion of `[[` in editors

## Next steps

1. Decide priority — block this feature or schedule for later
2. Approve the `Reference` model shape
3. Implement in order: model/report plumbing → graph storage → parser + scan → DB migration → explain surfaces → LSP
