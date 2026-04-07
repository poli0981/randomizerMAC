using RandomMac.Core.Models;
using System.Security.Cryptography;

namespace RandomMac.Core.Helpers;

/// <summary>
/// Generates random, valid MAC addresses using cryptographic RNG.
/// Generated addresses are locally administered and unicast.
/// </summary>
public static class MacAddressGenerator
{
    /// <summary>
    /// Generate a single random MAC address.
    /// Locally administered (bit 1 of octet 0 = 1) and unicast (bit 0 of octet 0 = 0).
    /// </summary>
    public static MacAddress Generate()
    {
        var bytes = RandomNumberGenerator.GetBytes(6);
        bytes[0] = (byte)((bytes[0] | 0x02) & 0xFE);
        return new MacAddress(bytes);
    }

    /// <summary>
    /// Generate a random MAC that is not in the given blacklist.
    /// Returns null if unable to generate after maxRetries attempts.
    /// </summary>
    public static MacAddress? Generate(IReadOnlySet<MacAddress> blacklist, int maxRetries = 10)
    {
        for (var i = 0; i < maxRetries; i++)
        {
            var mac = Generate();
            if (!blacklist.Contains(mac))
                return mac;
        }
        return null;
    }
}
