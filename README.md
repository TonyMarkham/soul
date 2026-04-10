# Soul

Soul is a semantic indexer that links documentation to the source code that implements it.

Documents live in `.docs/` as Markdown files with a structured frontmatter header. Source files carry lightweight annotations in native language syntax. Soul ties them together by a shared `id`, so you can ask: _what documents describe this interaction, and where in the codebase is it implemented?_

## How it works

**Documents** are Markdown files under `.docs/` with a frontmatter block:

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

The indexer scans the repository, links documents and annotations by `id`, and prints the result:

```
$ cargo run -p indexer -- explain interaction.checkout.create-order

ID: interaction.checkout.create-order

Documents:
- .docs/interactions/checkout.md [kind=interaction, title=Create order]

Annotations:
- fixtures/backend.rs:3 [role=backend]
- fixtures/frontend.cs:5 [role=frontend]
```

## Usage

```
cargo run -p indexer -- explain <id> [--root <path>]
```

`--root` defaults to the current directory. All printed paths are relative to the root.

Exit codes: `1` for fatal errors (invalid root, traversal failure), `0` for everything else including missing IDs and malformed content. Malformed annotations and unreadable files are reported as diagnostics rather than aborting the scan.

## Repository layout

```
.docs/                  soul documents (Markdown with frontmatter)
crates/
  indexer/              CLI and scan/graph library
  soul-attributes/      Rust proc-macro crate (#[soul(...)])
fixtures/               example annotated source files
packages/
  Soul.Attributes/      C# attribute package ([Soul(...)])
  Soul.Attributes.Tests/
```

## Running tests

```bash
cargo test -p indexer
cargo test -p soul-attributes
dotnet test packages/Soul.Attributes.Tests/Soul.Attributes.Tests.csproj
```
