using Microsoft.Extensions.Logging;
using RandomMac.Core.Helpers;
using RandomMac.Core.Models;
using RandomMac.Core.Services.Interfaces;
using System.Management;
using System.Net.NetworkInformation;

namespace RandomMac.Core.Services.Implementations;

public sealed class NetworkAdapterService : INetworkAdapterService
{
    private readonly ILogger<NetworkAdapterService> _logger;

    // Filters to exclude virtual, VPN, and Hyper-V adapters
    private static readonly string[] ExcludedServiceNames =
    [
        "vpn", "hyper-v", "virtual", "vmnet", "vbox", "tap",
        "tunnel", "teredo", "6to4", "isatap", "wintun", "wireguard"
    ];

    private static readonly string[] ExcludedPnpPrefixes =
    [
        "ROOT\\", "SWD\\", "BTH\\"
    ];

    public NetworkAdapterService(ILogger<NetworkAdapterService> logger)
    {
        _logger = logger;
    }

    public Task<IReadOnlyList<NetworkAdapterInfo>> GetPhysicalAdaptersAsync()
    {
        return Task.Run(() =>
        {
            var adapters = new List<NetworkAdapterInfo>();

            try
            {
                using var searcher = new ManagementObjectSearcher(
                    "SELECT * FROM Win32_NetworkAdapter WHERE PhysicalAdapter = TRUE AND MACAddress IS NOT NULL");

                foreach (ManagementObject obj in searcher.Get())
                {
                    var pnpDeviceId = obj["PNPDeviceID"]?.ToString() ?? "";
                    var serviceName = obj["ServiceName"]?.ToString() ?? "";
                    var name = obj["Name"]?.ToString() ?? obj["Description"]?.ToString() ?? "Unknown";
                    var description = obj["Description"]?.ToString() ?? "";
                    var macString = obj["MACAddress"]?.ToString() ?? "";

                    // Filter out virtual/VPN/Hyper-V adapters
                    if (IsExcluded(pnpDeviceId, serviceName, name, description))
                    {
                        _logger.LogDebug("Excluded adapter: {Name} (Service: {Service}, PNP: {Pnp})",
                            name, serviceName, pnpDeviceId);
                        continue;
                    }

                    var deviceId = Convert.ToInt32(obj["DeviceID"]);
                    var adapterType = DetermineAdapterType(name, description, pnpDeviceId);

                    // Find registry subkey
                    var regSubKey = RegistryHelper.FindAdapterRegistrySubKey(description, pnpDeviceId);

                    MacAddress.TryParse(macString, out var currentMac);

                    var adapter = new NetworkAdapterInfo
                    {
                        DeviceId = deviceId,
                        Name = name,
                        Description = description,
                        PnpDeviceId = pnpDeviceId,
                        ServiceName = serviceName,
                        RegistrySubKey = regSubKey,
                        CurrentMac = currentMac,
                        OriginalMac = currentMac, // Will be overridden from saved data
                        IsConnected = IsAdapterConnected(macString),
                        Type = adapterType,
                        IsEnabled = Convert.ToInt32(obj["NetConnectionStatus"] ?? 0) >= 1
                    };

                    adapters.Add(adapter);
                    _logger.LogInformation("Detected adapter: {Name} [{Mac}] (RegKey: {RegKey})",
                        name, currentMac, regSubKey ?? "N/A");
                }
            }
            catch (Exception ex)
            {
                _logger.LogError(ex, "Failed to enumerate network adapters via WMI");
            }

            return (IReadOnlyList<NetworkAdapterInfo>)adapters.AsReadOnly();
        });
    }

    public Task<NetworkAdapterInfo?> RefreshAdapterAsync(int deviceId)
    {
        return Task.Run(() =>
        {
            try
            {
                using var searcher = new ManagementObjectSearcher(
                    $"SELECT * FROM Win32_NetworkAdapter WHERE DeviceID = '{deviceId}'");

                foreach (ManagementObject obj in searcher.Get())
                {
                    var macString = obj["MACAddress"]?.ToString() ?? "";
                    MacAddress.TryParse(macString, out var currentMac);

                    var name = obj["Name"]?.ToString() ?? "";
                    var description = obj["Description"]?.ToString() ?? "";
                    var pnpDeviceId = obj["PNPDeviceID"]?.ToString() ?? "";

                    return new NetworkAdapterInfo
                    {
                        DeviceId = deviceId,
                        Name = name,
                        Description = description,
                        PnpDeviceId = pnpDeviceId,
                        ServiceName = obj["ServiceName"]?.ToString() ?? "",
                        RegistrySubKey = RegistryHelper.FindAdapterRegistrySubKey(description, pnpDeviceId),
                        CurrentMac = currentMac,
                        OriginalMac = currentMac,
                        IsConnected = IsAdapterConnected(macString),
                        Type = DetermineAdapterType(name, description, pnpDeviceId),
                        IsEnabled = Convert.ToInt32(obj["NetConnectionStatus"] ?? 0) >= 1
                    };
                }
            }
            catch (Exception ex)
            {
                _logger.LogError(ex, "Failed to refresh adapter {DeviceId}", deviceId);
            }

            return null;
        });
    }

    private static bool IsExcluded(string pnpDeviceId, string serviceName, string name, string description)
    {
        // Exclude by PNP prefix (non-physical buses)
        if (ExcludedPnpPrefixes.Any(p => pnpDeviceId.StartsWith(p, StringComparison.OrdinalIgnoreCase)))
            return true;

        // Exclude by service name keywords
        var combined = $"{serviceName} {name} {description}".ToLowerInvariant();
        return ExcludedServiceNames.Any(keyword => combined.Contains(keyword, StringComparison.OrdinalIgnoreCase));
    }

    private static AdapterType DetermineAdapterType(string name, string description, string pnpDeviceId)
    {
        var combined = $"{name} {description}".ToLowerInvariant();

        if (combined.Contains("wi-fi") || combined.Contains("wifi") || combined.Contains("wireless")
            || combined.Contains("wlan") || combined.Contains("802.11"))
            return AdapterType.WiFi;

        if (combined.Contains("ethernet") || combined.Contains("gigabit")
            || combined.Contains("realtek") || pnpDeviceId.StartsWith("PCI\\", StringComparison.OrdinalIgnoreCase))
            return AdapterType.Ethernet;

        return AdapterType.Other;
    }

    private static bool IsAdapterConnected(string macString)
    {
        if (string.IsNullOrEmpty(macString)) return false;

        try
        {
            var cleanMac = macString.Replace(":", "").Replace("-", "").ToUpperInvariant();
            foreach (var nic in NetworkInterface.GetAllNetworkInterfaces())
            {
                var nicMac = BitConverter.ToString(nic.GetPhysicalAddress().GetAddressBytes()).Replace("-", "");
                if (nicMac.Equals(cleanMac, StringComparison.OrdinalIgnoreCase)
                    && nic.OperationalStatus == OperationalStatus.Up)
                {
                    return true;
                }
            }
        }
        catch
        {
            // Swallow - connectivity is informational only
        }

        return false;
    }
}
