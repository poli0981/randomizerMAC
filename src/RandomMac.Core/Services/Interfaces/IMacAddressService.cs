using RandomMac.Core.Models;

namespace RandomMac.Core.Services.Interfaces;

public interface IMacAddressService
{
    /// <summary>
    /// Change the MAC address of the given adapter.
    /// Writes to registry, restarts adapter, then verifies.
    /// </summary>
    Task<MacChangeResult> ChangeMacAsync(NetworkAdapterInfo adapter, MacAddress newMac);

    /// <summary>
    /// Restore the original (factory) MAC address of the given adapter.
    /// </summary>
    Task<MacChangeResult> RestoreOriginalMacAsync(NetworkAdapterInfo adapter);
}
