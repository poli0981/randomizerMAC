using RandomMac.Core.Models;

namespace RandomMac.Core.Services.Interfaces;

public interface INetworkAdapterService
{
    /// <summary>
    /// Get all physical network adapters, excluding VPN, Hyper-V, and virtual adapters.
    /// </summary>
    Task<IReadOnlyList<NetworkAdapterInfo>> GetPhysicalAdaptersAsync();

    /// <summary>
    /// Refresh adapter status (current MAC, connection state).
    /// </summary>
    Task<NetworkAdapterInfo?> RefreshAdapterAsync(int deviceId);
}
