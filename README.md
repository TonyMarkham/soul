# Soul

Soul is a semantic indexer that links documentation to the source code that implements it.

Documents are Markdown files anywhere in the repository with a structured frontmatter header. Source files carry lightweight annotations in native language syntax. Soul ties them together by a shared `id`, so you can ask: _what documents describe this interaction, and where in the codebase is it implemented?_

## How it works

**Documents** are Markdown files with a frontmatter block:

```markdown
---
id: interaction.checkout.create-order
kind: interaction
title: Create order
---
```

**Annotations** are native-language attributes on the functions, methods, or classes that implement the interaction.

Rust:

```rust
#[soul(id = "interaction.checkout.create-order", role = "backend")]
pub fn create_order() { ... }
```

C#:

```csharp
[Soul("interaction.checkout.create-order", Role = "frontend")]
public void CreateOrder() { ... }
```

Soul scans the repository, links documents and annotations by `id`, and lets you query the result.

## Usage

### Initialise a repository

```bash
indexer init
```

Creates `.soul/soul.toml` from the built-in template. Edit it to tune exclusion rules and annotation extensions for your project.

### Build the index

```bash
indexer index
```

Scans the repository and writes `.soul/index.db`. Re-running performs a full rebuild.

### Query an ID

```bash
indexer explain <id>
```

Prints all documents and annotations linked to `id`. Falls back to a live scan if no index exists.

```
$ indexer explain interaction.checkout.create-order

ID: interaction.checkout.create-order

Documents:
- docs/checkout.md [kind=interaction, title=Create order]

Annotations:
- src/checkout.rs:12 [role=backend]
- src/CheckoutController.cs:8 [role=frontend]
```

All commands accept `--root <path>` (defaults to `.`). All printed paths are relative to root.

Exit codes: `1` for fatal errors (invalid root, missing config, traversal failure), `0` for everything else. Malformed annotations and unreadable files are reported as diagnostics rather than aborting the scan.

## Repository layout

```
.soul/                  soul config and index artifact
  soul.toml             scan configuration
  index.db              SQLite index (generated, not committed)
crates/
  indexer/              CLI and scan/graph/index library
  soul-attributes/      Rust proc-macro crate (#[soul(...)])
docs/                   soul documents (Markdown with frontmatter)
packages/
  Soul.Attributes/      C# attribute package ([Soul(...)])
  Soul.Attributes.Tests/
soul.toml.example       canonical config template
```

## Running tests

```bash
cargo test -p indexer
cargo test -p soul-attributes
dotnet test packages/Soul.Attributes.Tests/Soul.Attributes.Tests.csproj
```
