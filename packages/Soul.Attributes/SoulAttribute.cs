using System;
using System.Collections.Generic;
using System.Text.Json;

namespace Soul.Attributes;

[AttributeUsage(
    AttributeTargets.Class |
    AttributeTargets.Struct |
    AttributeTargets.Interface |
    AttributeTargets.Enum |
    AttributeTargets.Method |
    AttributeTargets.Property,
    AllowMultiple = true,
    Inherited = false)]
public sealed class SoulAttribute : Attribute
{
    private string? _metadataJson;

    public SoulAttribute(string id)
    {
        Id = RequireNonEmpty(id, nameof(id));
    }

    public string Id { get; }

    public string? MetadataJson
    {
        get => _metadataJson;
        set => _metadataJson = NormalizeAndValidateMetadata(value, nameof(MetadataJson));
    }

    private static string RequireNonEmpty(string value, string paramName)
    {
        if (string.IsNullOrWhiteSpace(value))
        {
            throw new ArgumentException(
                "Value must not be null, empty, or whitespace.",
                paramName);
        }

        return value.Trim();
    }

    private static string? NormalizeAndValidateMetadata(string? value, string paramName)
    {
        if (value is null)
        {
            return null;
        }

        var normalized = value.Trim();
        if (normalized.Length == 0)
        {
            throw new ArgumentException(
                "Soul attribute metadata must be valid JSON.",
                paramName);
        }

        try
        {
            using var document = JsonDocument.Parse(normalized);
            if (document.RootElement.ValueKind != JsonValueKind.Object)
            {
                throw new ArgumentException(
                    "Soul attribute metadata must be a JSON object.",
                    paramName);
            }

            var seen = new HashSet<string>(StringComparer.Ordinal);
            foreach (var property in document.RootElement.EnumerateObject())
            {
                if (property.Name is "id")
                {
                    throw new ArgumentException(
                        "Soul attribute metadata must not use reserved property names.",
                        paramName);
                }

                if (!seen.Add(property.Name))
                {
                    throw new ArgumentException(
                        "Soul attribute metadata must not contain duplicate property names.",
                        paramName);
                }
            }
        }
        catch (JsonException ex)
        {
            throw new ArgumentException(
                "Soul attribute metadata must be valid JSON.",
                paramName,
                ex);
        }

        return normalized;
    }
}
