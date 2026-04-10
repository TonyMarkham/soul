# soul-attributes

Proc-macro attributes for annotating code with [Soul](https://github.com/TonyMarkham/soul) semantic IDs.

Soul is a semantic indexer that links documentation to the source code that implements it. This crate
provides the `#[soul(...)]` attribute for Rust — the mechanism by which code symbols are connected to
Soul's semantic graph.

## Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
soul-attributes = "0.1"
```

Annotate any item with a Soul ID:

```rust
use soul_attributes::soul;

#[soul(id = "interaction.checkout.create-order", role = "backend")]
async fn create_order(/* ... */) {
    // ...
}
```

The attribute is **dev-time only** — it passes through the annotated item unchanged at compile time.
No runtime overhead, no generated code.

## Fields

| Field  | Required | Description |
|--------|----------|-------------|
| `id`   | Yes      | The Soul semantic ID. Must be a non-empty string literal. |
| `role` | No       | The symbol's role relative to the linked concept (e.g. `"backend"`, `"frontend"`). |

Additional named fields are accepted and stored as metadata by the Soul indexer.

## Validation

The attribute is validated at compile time:

- `id` is required and must not be empty
- All values must be string literals
- All keys must be valid identifiers
- Duplicate keys are rejected

## Multiple annotations

Multiple `#[soul(...)]` attributes may be applied to the same item:

```rust
#[soul(id = "interaction.checkout.create-order", role = "backend")]
#[soul(id = "concept.order", role = "implementation")]
async fn create_order(/* ... */) {}
```

## License

MIT
