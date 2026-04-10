# Soul.Attributes

Attribute for annotating code with [Soul](https://github.com/TonyMarkham/soul) semantic IDs.

Soul is a semantic indexer that links documentation to the source code that implements it. This package
provides the `[Soul(...)]` attribute for C# — the mechanism by which code symbols are connected to
Soul's semantic graph.

## Usage

Add to your project:

```bash
dotnet add package Soul.Attributes
```

Annotate any type or member with a Soul ID:

```csharp
using Soul.Attributes;

[Soul("interaction.checkout.create-order", Role = "frontend")]
public partial class CheckoutPage : ComponentBase { }
```

The attribute is **dev-time only** — no runtime behavior is injected. It exists solely to be read
by the Soul indexer.

## Constructor

```csharp
[Soul(string id)]
```

`id` is required and must not be null, empty, or whitespace.

## Properties

| Property       | Required | Description |
|----------------|----------|-------------|
| `Role`         | No       | The symbol's role relative to the linked concept (e.g. `"frontend"`, `"backend"`). |
| `MetadataJson` | No       | A JSON object string carrying additional metadata. Must not use the reserved key `id`. |

## Validation

Validation is performed at construction time:

- `id` must not be null, empty, or whitespace
- `MetadataJson`, if provided, must be a valid JSON object
- `MetadataJson` must not contain the reserved property `id`
- `MetadataJson` must not contain duplicate property names

## Multiple annotations

`[Soul]` is repeatable — multiple attributes may be applied to the same symbol:

```csharp
[Soul("interaction.checkout.create-order", Role = "frontend")]
[Soul("concept.order", Role = "consumer")]
public partial class CheckoutPage : ComponentBase { }
```

## Targets

Applicable to: `class`, `struct`, `interface`, `enum`, `method`, `property`.

## License

MIT
