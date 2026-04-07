using RandomMac.Core.Helpers;
using RandomMac.Core.Models;
using Xunit;

namespace RandomMac.Tests;

public class MacAddressGeneratorTests
{
    [Fact]
    public void Generate_ReturnsLocallyAdministeredUnicast()
    {
        for (var i = 0; i < 100; i++)
        {
            var mac = MacAddressGenerator.Generate();
            Assert.True(mac.IsLocallyAdministered, $"MAC {mac} should be locally administered");
            Assert.True(mac.IsUnicast, $"MAC {mac} should be unicast");
        }
    }

    [Fact]
    public void Generate_ReturnsDifferentValues()
    {
        var macs = new HashSet<MacAddress>();
        for (var i = 0; i < 50; i++)
            macs.Add(MacAddressGenerator.Generate());

        // With 50 random MACs from 2^46 space, collisions are astronomically unlikely
        Assert.True(macs.Count > 45, $"Expected mostly unique MACs, got {macs.Count}/50 unique");
    }

    [Fact]
    public void Generate_WithBlacklist_AvoidsBlacklistedMacs()
    {
        var blacklist = new HashSet<MacAddress>();

        // Generate a MAC and add it to blacklist
        var blocked = MacAddressGenerator.Generate();
        blacklist.Add(blocked);

        // Generate with blacklist - should not match blocked MAC
        for (var i = 0; i < 20; i++)
        {
            var mac = MacAddressGenerator.Generate(blacklist);
            Assert.NotNull(mac);
            Assert.NotEqual(blocked, mac.Value);
        }
    }

    [Fact]
    public void Generate_WithBlacklist_ReturnsNullWhenExhausted()
    {
        // Create an impossibly large blacklist scenario by using max retries = 0
        // In practice, blacklist would need to cover the entire MAC space
        // We test the null return path by setting maxRetries = 0
        var blacklist = new HashSet<MacAddress>();
        var result = MacAddressGenerator.Generate(blacklist, maxRetries: 0);
        Assert.Null(result);
    }

    [Fact]
    public void Generate_FirstOctet_HasCorrectBits()
    {
        for (var i = 0; i < 100; i++)
        {
            var mac = MacAddressGenerator.Generate();
            var firstByte = mac.Bytes[0];

            // Bit 1 (LA) must be set
            Assert.True((firstByte & 0x02) != 0, $"LA bit not set: 0x{firstByte:X2}");

            // Bit 0 (multicast) must be clear
            Assert.True((firstByte & 0x01) == 0, $"Multicast bit set: 0x{firstByte:X2}");
        }
    }

    [Fact]
    public void Generate_Returns6Bytes()
    {
        var mac = MacAddressGenerator.Generate();
        Assert.Equal(6, mac.Bytes.Length);
    }
}
