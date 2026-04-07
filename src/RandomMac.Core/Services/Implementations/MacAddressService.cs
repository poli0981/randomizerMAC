using Microsoft.Extensions.Logging;
using RandomMac.Core.Helpers;
using RandomMac.Core.Models;
using RandomMac.Core.Services.Interfaces;
using System.Management;

namespace RandomMac.Core.Services.Implementations;

public sealed class MacAddressService : IMacAddressService
{
    private readonly ILogger<MacAddressService> _logger;

    public MacAddressService(ILogger<MacAddressService> logger)
    {
        _logger = logger;
    }

    public async Task<MacChangeResult> ChangeMacAsync(NetworkAdapterInfo adapter, MacAddress newMac)
    {
        var previousMac = adapter.CurrentMac;

        if (string.IsNullOrEmpty(adapter.RegistrySubKey))
        {
            _logger.LogError("No registry subkey found for adapter {Name}", adapter.Name);
            return MacChangeResult.Fail(previousMac, newMac, "Registry subkey not found for this adapter.");
        }

        _logger.LogInformation("Changing MAC for {Name}: {Old} -> {New}",
            adapter.Name, previousMac, newMac);

        try
        {
            // Step 1: Write MAC to registry
            if (!RegistryHelper.WriteNetworkAddress(adapter.RegistrySubKey, newMac.ToRegistryString()))
            {
                return MacChangeResult.Fail(previousMac, newMac, "Failed to write MAC address to registry.");
            }
            _logger.LogDebug("Registry updated for adapter {Name}", adapter.Name);

            // Step 2: Restart adapter (disable then enable)
            await RestartAdapterAsync(adapter.DeviceId);
            _logger.LogDebug("Adapter {Name} restarted", adapter.Name);

            // Step 3: Wait for adapter to come back online
            await Task.Delay(2000);

            // Step 4: Verify the change
            var verifiedMac = await ReadCurrentMacAsync(adapter.DeviceId);

            if (verifiedMac == newMac)
            {
                _logger.LogInformation("MAC change verified for {Name}: {Mac}", adapter.Name, verifiedMac);
                return MacChangeResult.Ok(previousMac, newMac, verifiedMac);
            }
            else
            {
                _logger.LogWarning("MAC verification mismatch for {Name}: expected {Expected}, got {Actual}",
                    adapter.Name, newMac, verifiedMac);
                return MacChangeResult.Fail(previousMac, newMac,
                    $"MAC change may not have been applied. Expected {newMac}, got {verifiedMac}. The adapter driver may not support MAC spoofing.");
            }
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Failed to change MAC for adapter {Name}", adapter.Name);
            return MacChangeResult.Fail(previousMac, newMac, $"Error: {ex.Message}");
        }
    }

    public async Task<MacChangeResult> RestoreOriginalMacAsync(NetworkAdapterInfo adapter)
    {
        var previousMac = adapter.CurrentMac;

        if (string.IsNullOrEmpty(adapter.RegistrySubKey))
        {
            return MacChangeResult.Fail(previousMac, adapter.OriginalMac, "Registry subkey not found.");
        }

        _logger.LogInformation("Restoring original MAC for {Name}: {Original}", adapter.Name, adapter.OriginalMac);

        try
        {
            // Remove NetworkAddress from registry to restore factory MAC
            if (!RegistryHelper.RemoveNetworkAddress(adapter.RegistrySubKey))
            {
                return MacChangeResult.Fail(previousMac, adapter.OriginalMac, "Failed to remove MAC from registry.");
            }

            await RestartAdapterAsync(adapter.DeviceId);
            await Task.Delay(2000);

            var verifiedMac = await ReadCurrentMacAsync(adapter.DeviceId);

            _logger.LogInformation("Original MAC restored for {Name}: {Mac}", adapter.Name, verifiedMac);
            return MacChangeResult.Ok(previousMac, adapter.OriginalMac, verifiedMac);
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Failed to restore original MAC for {Name}", adapter.Name);
            return MacChangeResult.Fail(previousMac, adapter.OriginalMac, $"Error: {ex.Message}");
        }
    }

    private static Task RestartAdapterAsync(int deviceId)
    {
        return Task.Run(() =>
        {
            using var searcher = new ManagementObjectSearcher(
                $"SELECT * FROM Win32_NetworkAdapter WHERE DeviceID = '{deviceId}'");

            foreach (ManagementObject obj in searcher.Get())
            {
                obj.InvokeMethod("Disable", null);
                Thread.Sleep(1000);
                obj.InvokeMethod("Enable", null);
            }
        });
    }

    private static Task<MacAddress> ReadCurrentMacAsync(int deviceId)
    {
        return Task.Run(() =>
        {
            using var searcher = new ManagementObjectSearcher(
                $"SELECT MACAddress FROM Win32_NetworkAdapter WHERE DeviceID = '{deviceId}'");

            foreach (ManagementObject obj in searcher.Get())
            {
                var macString = obj["MACAddress"]?.ToString();
                if (MacAddress.TryParse(macString, out var mac))
                    return mac;
            }

            return MacAddress.Empty;
        });
    }
}
