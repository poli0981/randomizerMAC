using System.Globalization;
using System.Text.RegularExpressions;

namespace RandomMac.Core.Models;

/// <summary>
/// Value object representing a MAC address (6 bytes).
/// </summary>
public readonly partial struct MacAddress : IEquatable<MacAddress>
{
    private readonly byte[] _bytes;

    public static readonly MacAddress Empty = new([0, 0, 0, 0, 0, 0]);

    public MacAddress(byte[] bytes)
    {
        if (bytes.Length != 6)
            throw new ArgumentException("MAC address must be exactly 6 bytes.", nameof(bytes));

        _bytes = [.. bytes];
    }

    public ReadOnlySpan<byte> Bytes => _bytes;

    /// <summary>
    /// Whether this is a locally administered address (bit 1 of first octet set).
    /// </summary>
    public bool IsLocallyAdministered => (_bytes[0] & 0x02) != 0;

    /// <summary>
    /// Whether this is a unicast address (bit 0 of first octet clear).
    /// </summary>
    public bool IsUnicast => (_bytes[0] & 0x01) == 0;

    /// <summary>
    /// Returns the MAC as "AA-BB-CC-DD-EE-FF".
    /// </summary>
    public string ToDisplayString()
        => string.Join("-", _bytes.Select(b => b.ToString("X2")));

    /// <summary>
    /// Returns the MAC as "AABBCCDDEEFF" (for registry).
    /// </summary>
    public string ToRegistryString()
        => string.Concat(_bytes.Select(b => b.ToString("X2")));

    /// <summary>
    /// Parse from various formats: "AA-BB-CC-DD-EE-FF", "AA:BB:CC:DD:EE:FF", "AABBCCDDEEFF".
    /// </summary>
    public static MacAddress Parse(string value)
    {
        if (!TryParse(value, out var result))
            throw new FormatException($"Invalid MAC address format: '{value}'");
        return result;
    }

    public static bool TryParse(string? value, out MacAddress result)
    {
        result = Empty;
        if (string.IsNullOrWhiteSpace(value))
            return false;

        var cleaned = MacCleanRegex().Replace(value.Trim(), "");
        if (cleaned.Length != 12)
            return false;

        var bytes = new byte[6];
        for (var i = 0; i < 6; i++)
        {
            if (!byte.TryParse(cleaned.AsSpan(i * 2, 2), NumberStyles.HexNumber, null, out bytes[i]))
                return false;
        }

        result = new MacAddress(bytes);
        return true;
    }

    public bool Equals(MacAddress other) => _bytes.AsSpan().SequenceEqual(other._bytes);
    public override bool Equals(object? obj) => obj is MacAddress other && Equals(other);
    public override int GetHashCode() => HashCode.Combine(_bytes[0], _bytes[1], _bytes[2], _bytes[3], _bytes[4], _bytes[5]);
    public override string ToString() => ToDisplayString();

    public static bool operator ==(MacAddress left, MacAddress right) => left.Equals(right);
    public static bool operator !=(MacAddress left, MacAddress right) => !left.Equals(right);

    [GeneratedRegex("[^0-9A-Fa-f]")]
    private static partial Regex MacCleanRegex();
}
