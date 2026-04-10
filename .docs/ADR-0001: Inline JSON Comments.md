# ADR-0001: Repo-Soul Annotation Strategy (Inline JSON Comments)

## Status

Accepted

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

Annotations will be implemented as **single-line structured comments using inline JSON**, prefixed with `@soul`.

### Format

Rust / C#:

```
// @soul {"id":"interaction.checkout.create-order","role":"backend"}
```

### Rules

1. **Single-line only**

    * Each `@soul` annotation must exist entirely on one line
    * Multi-line annotations are not supported

2. **Strict JSON**

    * Must be valid JSON
    * Double quotes required
    * No trailing commas

3. **Prefix**

    * Must begin with `// @soul`
    * Anything else is ignored

4. **Primary Identifier**

    * Each annotation should include an `id` field when defining a primary relationship

5. **Multiple annotations allowed**

    * Multiple `@soul` lines may be attached to the same symbol

Example:

```rust
// @soul {"id":"interaction.checkout.create-order","role":"backend"}
async fn create_order(...) {}
```

```csharp
// @soul {"id":"interaction.checkout.create-order","role":"frontend"}
public partial class CheckoutPage : ComponentBase {}
```

## Rationale

### Why not attributes/macros?

* Require language-specific packages (Rust crate, NuGet)
* Introduce compile-time coupling
* May leak into compiled artifacts
* Slower to iterate in early stages

### Why not multi-line blocks?

* Require stateful parsing
* Introduce grouping ambiguity
* More fragile under formatting tools
* Harder to refactor safely

### Why not YAML/TOML in comments?

* Harder to keep atomic
* More verbose
* Less consistent across languages

### Why inline JSON?

* **Atomic**: one line = one record
* **Deterministic parsing**: trivial to extract
* **Language-agnostic**
* **Easy to validate**
* **Resilient to formatting (when wrap_comments = false)**
* **Maps cleanly to internal data structures**

## Consequences

### Positive

* Extremely simple parsing logic
* No compiler/toolchain dependency
* Fast to implement (ideal for Slice 1)
* Works uniformly across Rust, C#, and future languages
* Easy for LLMs to generate and modify
* Easy to hash and diff for change detection

### Negative

* Less ergonomic than native attributes
* No compile-time validation
* Can become long/unwieldy with many fields
* Requires discipline to keep JSON valid

### Mitigations

* Indexer will:

    * Validate JSON
    * Emit diagnostics for malformed annotations
    * Ignore invalid entries safely

* Repository will enforce:

```toml
# rustfmt.toml
wrap_comments = false
```

* Keep annotations small and focused

## Future Evolution

This approach is intentionally minimal and may evolve.

Possible future upgrades:

* Introduce Rust/C# attribute packages for stronger typing
* Add LSP validation and autocomplete for JSON fields
* Introduce schema validation for annotation content
* Support code actions for inserting/editing annotations
* Add refactor-safe ID rename across annotations

The inline JSON format is compatible with future migration to attributes.

## Related Decisions

* ADR-0002: Frontmatter as DSL for document identity and relationships (planned)
* ADR-0003: Wiki-style document linking with canonical IDs (planned)

## Summary

Use **inline JSON in single-line comments (`// @soul {...}`)** as the initial annotation mechanism to connect code to repo-level semantic concepts.

This provides the simplest, most robust foundation for building the repo-soul system across LSP, MCP, and indexing layers.
