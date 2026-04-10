# ADR-0001: Repo-Soul Annotation Strategy (Code-Native Attributes)

## Status

Superseded — original decision (inline JSON comments) was replaced during implementation. Code-native attributes were chosen instead.

## Context

The system aims to build a "repo soul" — a semantic layer connecting:

* Backend code (Rust)
* Frontend code (C# / Blazor)
* Markdown documentation (with YAML frontmatter)

This semantic layer is consumed by:

* A local indexer (Rust)
* An LSP server (for IDE integration)
* An MCP server (for LLM interaction)

To connect code to higher-level concepts (interactions, invariants, etc.), annotations must be embedded directly in source code.

Key constraints:

* Annotations must be **dev-time only**
* No runtime behavior or overhead
* No coupling to language-specific runtime systems
* Must be **portable across languages**
* Must be **easy to parse deterministically**
* Must be **resilient to formatting tools (e.g., rustfmt)**
* Must be **easy for humans and LLMs to read/write**

## Decision

Annotations are implemented as **code-native attributes** using a `soul-attributes` proc-macro crate (Rust) and a `[Soul(...)]` attribute (C#).

### Format

Rust:

```rust
#[soul(id = "interaction.checkout.create-order", role = "backend")]
async fn create_order(...) {}
```

C#:

```csharp
[Soul("interaction.checkout.create-order", Role = "frontend")]
public partial class CheckoutPage : ComponentBase {}
```

### Rules

1. **Primary Identifier**

    * Each annotation must include an `id` field

2. **Role field**

    * Optional; indicates the symbol's role relative to the linked concept

3. **Additional metadata**

    * Extra key-value pairs may be attached as named arguments

4. **Multiple annotations allowed**

    * Multiple `@soul` attributes may be attached to the same symbol

## Rationale

### Why not inline JSON comments?

* Comments are invisible to the compiler — no validation at build time
* Harder to tooling-assist (no autocomplete, no refactor support)
* Annotation intent is clearer expressed as a language construct

### Why code-native attributes?

* **Compiler-checked**: malformed annotations are caught at build time
* **IDE-friendly**: hover, autocomplete, and refactoring work naturally
* **Idiomatic**: matches how developers already annotate code (derives, cfg, etc.)
* **Strongly typed**: the `soul-attributes` proc-macro validates structure at compile time
* **Refactor-safe**: tools that rename symbols naturally carry attributes with them

## Consequences

### Positive

* Compile-time validation of annotation structure
* IDE support (autocomplete, go-to-definition on attribute fields)
* Familiar syntax for Rust/C# developers
* Refactor-safe: symbol renames carry attributes

### Negative

* Language-specific packages required (`soul-attributes` crate, future C# NuGet)
* Slightly more coupling than comment-based approach
* LLMs must know the attribute syntax (vs. freeform JSON)

## Future Evolution

* Add LSP validation and autocomplete for attribute field values
* Introduce schema validation for known `id` values
* Support code actions for inserting/editing annotations
* Add refactor-safe ID rename across annotations and docs
* Publish `soul-attributes` as a standalone crate/NuGet for consumer repos

## Related Decisions

* ADR-0002: Frontmatter as DSL for document identity and relationships (planned)
* ADR-0003: Wiki-style document linking with canonical IDs (planned)

## Summary

Use **code-native attributes (`#[soul(...)]` / `[Soul(...)]`)** as the annotation mechanism to connect code to repo-level semantic concepts.

This provides compile-time validation, IDE integration, and idiomatic developer ergonomics across the LSP, MCP, and indexing layers.
