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
pub struct Reference {
    pub source_id: String,       // the doc that contains the [[link]]
    pub target_id: String,       // the ID being linked to
    pub source_path: PathBuf,    // file containing the link
    pub source_line: usize,      // line number
    pub display_text: Option<String>, // optional [[id|custom text]]
}
```

Add `pub mod reference;` and `pub use reference::Reference;` to `crates/indexer/src/model/mod.rs`.

Add `references: Vec<Reference>` to `SemanticGraph`.

### 2. Parser — Extract `[[id]]` from doc bodies

**File:** `crates/indexer/src/markdown/wikilinks.rs` (new)

- After frontmatter extraction in `parse_markdown`, pass the body text to a new `extract_wikilinks` function
- Scan for `[[...]]` patterns using simple string matching or regex
- For each match:
  - Split on `|` to separate ID from optional display text
  - Emit a `Reference` with the current doc's ID as `source_id`
- Doc IDs without frontmatter (e.g., reference docs) cannot be sources for wiki links

Add module declaration and re-export in `crates/indexer/src/markdown/mod.rs`.

### 3. Graph — Store references

**File:** `crates/indexer/src/model/semantic_graph.rs`

```rust
pub struct SemanticGraph {
    pub documents: Vec<Document>,
    pub annotations: Vec<CodeAnnotation>,
    pub references: Vec<Reference>,
    pub diagnostics: Vec<Diagnostic>,
}
```

### 4. Index DB — Persist references

**File:** `crates/indexer/src/index/` (exact file TBD)

- Add a `doc_references` table: `(id INTEGER PRIMARY KEY, source_id TEXT, target_id TEXT, source_path TEXT, source_line INTEGER, display_text TEXT)`
- Write references during `write_index`
- Read references during `load_graph`

### 5. Explain — Surface references in `soul_explain`

**File:** `crates/indexer/src/graph/explain.rs`

- When explaining an ID, also return all references where `target_id` matches
- Show them as a "Referenced by" section in the output

### 6. LSP — Navigate wiki links

**File:** `crates/soul-lsp/src/` (exact file TBD)

- In the document body, detect `[[id]]` tokens via the LSP's semantic tokens or document link provider
- On go-to-definition for a `[[id]]` token, resolve to the target Soul doc (or the list of all annotated locations if no doc exists)
- On hover, show the target doc's title and kind

### 7. Parser edge cases

- `[[id]]` — basic form
- `[[id|Display text]]` — with display text
- `[[id1]] [[id2]]` — multiple on one line
- Escaped brackets: `\[[not a link]]` — must be ignored
- Nested brackets: `[[outer [[inner]]]]` — parse inner only
- Malformed: `[[unclosed`, `[[]]`, `[[|text]]` — emit diagnostics

## Files touched

| File | Action |
|------|--------|
| `crates/indexer/src/model/reference.rs` | **Create** — `Reference` struct |
| `crates/indexer/src/model/mod.rs` | **Edit** — add module + re-export |
| `crates/indexer/src/model/semantic_graph.rs` | **Edit** — add `references` field |
| `crates/indexer/src/markdown/wikilinks.rs` | **Create** — wiki link parser |
| `crates/indexer/src/markdown/mod.rs` | **Edit** — declare module, call parser |
| `crates/indexer/src/index/` | **Edit** — persist references |
| `crates/indexer/src/graph/explain.rs` | **Edit** — surface references in output |
| `crates/soul-lsp/src/` | **Edit** — navigate wiki links |

## Out of scope (first pass)

- Wiki links from non-frontmatter `.md` files (reference docs can't be link sources)
- Backlinks index (requires bidirectional query — can be added later)
- `soul_list_gaps` integration for broken wiki links (dead references)
- Auto-completion of `[[` in editors

## Next steps

1. Decide priority — block this feature or schedule for later
2. Approve the `Reference` model shape
3. Implement in order: model → parser → graph → DB → explain → LSP
