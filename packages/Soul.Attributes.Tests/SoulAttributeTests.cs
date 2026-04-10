using System;
using Soul.Attributes;
using Xunit;

public sealed class SoulAttributeTests
{
    [Fact]
    public void ConstructorTrimsAndRejectsWhitespaceOnlyIds()
    {
        var attribute = new SoulAttribute("  interaction.checkout.create-order  ");

        Assert.Equal("interaction.checkout.create-order", attribute.Id);
        Assert.Throws<ArgumentException>(() => new SoulAttribute("   "));
        Assert.Throws<ArgumentException>(() => new SoulAttribute(null!));
    }

    [Fact]
    public void MetadataJsonRejectsInvalidInputs()
    {
        var attribute = new SoulAttribute("interaction.checkout.create-order");

        Assert.Throws<ArgumentException>(() => attribute.MetadataJson = "   ");
        Assert.Throws<ArgumentException>(() => attribute.MetadataJson = "not json");
        Assert.Throws<ArgumentException>(() => attribute.MetadataJson = "[]");
        Assert.Throws<ArgumentException>(() => attribute.MetadataJson = "{\"id\":\"duplicate\"}");
        Assert.Throws<ArgumentException>(() => attribute.MetadataJson = "{\"surface\":\"frontend\",\"surface\":\"dup\"}");
    }

    [Fact]
    public void MetadataJsonAcceptsAUniqueObjectPayload()
    {
        var attribute = new SoulAttribute("interaction.checkout.create-order")
        {
            MetadataJson = "{\"surface\":\"frontend\",\"priority\":2}",
        };

        Assert.Equal("{\"surface\":\"frontend\",\"priority\":2}", attribute.MetadataJson);
    }
}
