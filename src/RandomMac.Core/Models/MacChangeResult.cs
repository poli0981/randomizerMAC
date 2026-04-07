namespace RandomMac.Core.Models;

/// <summary>
/// Result of a MAC address change operation.
/// </summary>
public sealed class MacChangeResult
{
    public required bool Success { get; init; }
    public required MacAddress PreviousMac { get; init; }
    public required MacAddress NewMac { get; init; }
    public MacAddress VerifiedMac { get; init; }
    public string? ErrorMessage { get; init; }
    public DateTime Timestamp { get; init; } = DateTime.UtcNow;

    public static MacChangeResult Ok(MacAddress previous, MacAddress newMac, MacAddress verified) => new()
    {
        Success = true,
        PreviousMac = previous,
        NewMac = newMac,
        VerifiedMac = verified
    };

    public static MacChangeResult Fail(MacAddress previous, MacAddress attempted, string error) => new()
    {
        Success = false,
        PreviousMac = previous,
        NewMac = attempted,
        ErrorMessage = error
    };
}
