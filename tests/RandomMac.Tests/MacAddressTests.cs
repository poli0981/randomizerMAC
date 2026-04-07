using RandomMac.Core.Models;
using Xunit;

namespace RandomMac.Tests;

public class MacAddressTests
{
    [Fact]
    public void Parse_DashFormat_ReturnsCorrectBytes()
    {
        var mac = MacAddress.Parse("AA-BB-CC-DD-EE-FF");
        Assert.Equal("AA-BB-CC-DD-EE-FF", mac.ToDisplayString());
    }

    [Fact]
    public void Parse_ColonFormat_ReturnsCorrectBytes()
    {
        var mac = MacAddress.Parse("AA:BB:CC:DD:EE:FF");
        Assert.Equal("AA-BB-CC-DD-EE-FF", mac.ToDisplayString());
    }

    [Fact]
    public void Parse_RawFormat_ReturnsCorrectBytes()
    {
        var mac = MacAddress.Parse("AABBCCDDEEFF");
        Assert.Equal("AA-BB-CC-DD-EE-FF", mac.ToDisplayString());
    }

    [Fact]
    public void Parse_LowerCase_ReturnsUpperCase()
    {
        var mac = MacAddress.Parse("aa:bb:cc:dd:ee:ff");
        Assert.Equal("AA-BB-CC-DD-EE-FF", mac.ToDisplayString());
    }

    [Fact]
    public void ToRegistryString_ReturnsNoSeparators()
    {
        var mac = MacAddress.Parse("AA-BB-CC-DD-EE-FF");
        Assert.Equal("AABBCCDDEEFF", mac.ToRegistryString());
    }

    [Fact]
    public void TryParse_InvalidLength_ReturnsFalse()
    {
        Assert.False(MacAddress.TryParse("AA-BB-CC", out _));
    }

    [Fact]
    public void TryParse_Null_ReturnsFalse()
    {
        Assert.False(MacAddress.TryParse(null, out _));
    }

    [Fact]
    public void TryParse_Empty_ReturnsFalse()
    {
        Assert.False(MacAddress.TryParse("", out _));
    }

    [Fact]
    public void TryParse_InvalidHex_ReturnsFalse()
    {
        Assert.False(MacAddress.TryParse("GG-HH-II-JJ-KK-LL", out _));
    }

    [Fact]
    public void Equality_SameBytes_AreEqual()
    {
        var a = MacAddress.Parse("AA-BB-CC-DD-EE-FF");
        var b = MacAddress.Parse("AA:BB:CC:DD:EE:FF");
        Assert.Equal(a, b);
        Assert.True(a == b);
    }

    [Fact]
    public void Equality_DifferentBytes_AreNotEqual()
    {
        var a = MacAddress.Parse("AA-BB-CC-DD-EE-FF");
        var b = MacAddress.Parse("11-22-33-44-55-66");
        Assert.NotEqual(a, b);
        Assert.True(a != b);
    }

    [Fact]
    public void IsLocallyAdministered_Bit1Set_ReturnsTrue()
    {
        // 0x02 = bit 1 set
        var mac = new MacAddress([0x02, 0x00, 0x00, 0x00, 0x00, 0x00]);
        Assert.True(mac.IsLocallyAdministered);
    }

    [Fact]
    public void IsLocallyAdministered_Bit1Clear_ReturnsFalse()
    {
        var mac = new MacAddress([0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
        Assert.False(mac.IsLocallyAdministered);
    }

    [Fact]
    public void IsUnicast_Bit0Clear_ReturnsTrue()
    {
        var mac = new MacAddress([0x02, 0x00, 0x00, 0x00, 0x00, 0x00]);
        Assert.True(mac.IsUnicast);
    }

    [Fact]
    public void IsUnicast_Bit0Set_ReturnsFalse()
    {
        var mac = new MacAddress([0x03, 0x00, 0x00, 0x00, 0x00, 0x00]);
        Assert.False(mac.IsUnicast);
    }

    [Fact]
    public void Constructor_WrongLength_Throws()
    {
        Assert.Throws<ArgumentException>(() => new MacAddress([0x00, 0x00]));
    }

    [Fact]
    public void GetHashCode_SameBytes_SameHash()
    {
        var a = MacAddress.Parse("AA-BB-CC-DD-EE-FF");
        var b = MacAddress.Parse("AA-BB-CC-DD-EE-FF");
        Assert.Equal(a.GetHashCode(), b.GetHashCode());
    }
}
