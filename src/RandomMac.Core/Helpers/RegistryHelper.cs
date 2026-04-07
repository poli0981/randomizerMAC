using Microsoft.Win32;
using System.Runtime.Versioning;

namespace RandomMac.Core.Helpers;

/// <summary>
/// Helper for Windows registry operations related to network adapter MAC addresses.
/// </summary>
[SupportedOSPlatform("windows")]
public static class RegistryHelper
{
    private const string NetworkClassGuid = @"SYSTEM\CurrentControlSet\Control\Class\{4d36e972-e325-11ce-bfc1-08002be10318}";
    private const string StartupRegKey = @"SOFTWARE\Microsoft\Windows\CurrentVersion\Run";
    private const string AppName = "RandomMac";

    /// <summary>
    /// Find the registry subkey index for a given adapter by matching DriverDesc or InstanceId.
    /// </summary>
    public static string? FindAdapterRegistrySubKey(string adapterDescription, string pnpDeviceId)
    {
        using var baseKey = Registry.LocalMachine.OpenSubKey(NetworkClassGuid);
        if (baseKey is null) return null;

        foreach (var subKeyName in baseKey.GetSubKeyNames())
        {
            if (!int.TryParse(subKeyName, out _)) continue;

            using var subKey = baseKey.OpenSubKey(subKeyName);
            if (subKey is null) continue;

            var driverDesc = subKey.GetValue("DriverDesc") as string;
            var instanceId = subKey.GetValue("MatchingDeviceId") as string;

            if ((!string.IsNullOrEmpty(driverDesc) && driverDesc.Equals(adapterDescription, StringComparison.OrdinalIgnoreCase))
                || (!string.IsNullOrEmpty(instanceId) && pnpDeviceId.Contains(instanceId, StringComparison.OrdinalIgnoreCase)))
            {
                return subKeyName;
            }
        }

        return null;
    }

    /// <summary>
    /// Write a MAC address to the registry for the given adapter subkey.
    /// </summary>
    public static bool WriteNetworkAddress(string subKey, string macAddress)
    {
        var keyPath = $@"{NetworkClassGuid}\{subKey}";
        using var key = Registry.LocalMachine.OpenSubKey(keyPath, writable: true);
        if (key is null) return false;

        key.SetValue("NetworkAddress", macAddress, RegistryValueKind.String);
        return true;
    }

    /// <summary>
    /// Read the current NetworkAddress value from registry for the given adapter subkey.
    /// </summary>
    public static string? ReadNetworkAddress(string subKey)
    {
        var keyPath = $@"{NetworkClassGuid}\{subKey}";
        using var key = Registry.LocalMachine.OpenSubKey(keyPath);
        return key?.GetValue("NetworkAddress") as string;
    }

    /// <summary>
    /// Remove the NetworkAddress value to restore original MAC.
    /// </summary>
    public static bool RemoveNetworkAddress(string subKey)
    {
        var keyPath = $@"{NetworkClassGuid}\{subKey}";
        using var key = Registry.LocalMachine.OpenSubKey(keyPath, writable: true);
        if (key is null) return false;

        try
        {
            key.DeleteValue("NetworkAddress", throwOnMissingValue: false);
            return true;
        }
        catch
        {
            return false;
        }
    }

    /// <summary>
    /// Set or remove the app from Windows startup using Task Scheduler.
    /// Task Scheduler supports "Run with highest privileges" to avoid UAC prompts.
    /// </summary>
    public static bool SetStartup(bool enable, string? exePath = null, bool startMinimized = false)
    {
        try
        {
            // Cleanup old registry-based startup entry (migration)
            try
            {
                using var regKey = Registry.CurrentUser.OpenSubKey(StartupRegKey, writable: true);
                regKey?.DeleteValue(AppName, throwOnMissingValue: false);
            }
            catch { /* ignore cleanup errors */ }

            if (enable && exePath is not null)
            {
                var args = startMinimized ? "--minimized" : "";
                var psi = new System.Diagnostics.ProcessStartInfo
                {
                    FileName = "schtasks",
                    Arguments = $"/Create /TN \"{AppName}\" /TR \"\\\"{exePath}\\\" {args}\" /SC ONLOGON /RL HIGHEST /F",
                    UseShellExecute = false,
                    CreateNoWindow = true,
                    RedirectStandardOutput = true,
                    RedirectStandardError = true
                };
                var proc = System.Diagnostics.Process.Start(psi);
                proc?.WaitForExit(5000);
                return proc?.ExitCode == 0;
            }
            else
            {
                var psi = new System.Diagnostics.ProcessStartInfo
                {
                    FileName = "schtasks",
                    Arguments = $"/Delete /TN \"{AppName}\" /F",
                    UseShellExecute = false,
                    CreateNoWindow = true,
                    RedirectStandardOutput = true,
                    RedirectStandardError = true
                };
                var proc = System.Diagnostics.Process.Start(psi);
                proc?.WaitForExit(5000);
                return true; // OK even if task didn't exist
            }
        }
        catch
        {
            return false;
        }
    }

    /// <summary>
    /// Check if the app is set to run at startup via Task Scheduler.
    /// </summary>
    public static bool IsStartupEnabled()
    {
        try
        {
            var psi = new System.Diagnostics.ProcessStartInfo
            {
                FileName = "schtasks",
                Arguments = $"/Query /TN \"{AppName}\"",
                UseShellExecute = false,
                CreateNoWindow = true,
                RedirectStandardOutput = true,
                RedirectStandardError = true
            };
            var proc = System.Diagnostics.Process.Start(psi);
            proc?.WaitForExit(3000);
            return proc?.ExitCode == 0;
        }
        catch
        {
            return false;
        }
    }
}
