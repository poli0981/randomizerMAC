namespace RandomMac.Core.Models;

/// <summary>
/// Represents a physical network adapter detected on the system.
/// </summary>
public sealed class NetworkAdapterInfo
{
    /// <summary>WMI DeviceID (index).</summary>
    public required int DeviceId { get; init; }

    /// <summary>Display name (e.g. "Intel Wi-Fi 6 AX201").</summary>
    public required string Name { get; init; }

    /// <summary>Adapter description from WMI.</summary>
    public string Description { get; init; } = string.Empty;

    /// <summary>PNP Device ID (e.g. "PCI\VEN_8086...").</summary>
    public string PnpDeviceId { get; init; } = string.Empty;

    /// <summary>Service name (driver).</summary>
    public string ServiceName { get; init; } = string.Empty;

    /// <summary>Registry subkey index under the network class GUID.</summary>
    public string? RegistrySubKey { get; init; }

    /// <summary>Current MAC address read from the OS.</summary>
    public MacAddress CurrentMac { get; set; }

    /// <summary>Original (factory) MAC address, saved on first detection.</summary>
    public MacAddress OriginalMac { get; set; }

    /// <summary>Whether the adapter currently has network connectivity.</summary>
    public bool IsConnected { get; set; }

    /// <summary>Adapter type: Wi-Fi, Ethernet, etc.</summary>
    public AdapterType Type { get; init; }

    /// <summary>Whether the adapter is enabled.</summary>
    public bool IsEnabled { get; set; }
}

public enum AdapterType
{
    Ethernet,
    WiFi,
    Other
}
