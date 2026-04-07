namespace RandomMac.Core.Models;

/// <summary>
/// A record of a MAC address change for history tracking.
/// </summary>
public sealed class MacHistoryEntry
{
    public required string AdapterName { get; init; }
    public required int AdapterDeviceId { get; init; }
    public required string PreviousMac { get; init; }
    public required string NewMac { get; init; }
    public required bool Success { get; init; }
    public string? ErrorMessage { get; init; }
    public DateTime Timestamp { get; init; } = DateTime.UtcNow;
}
