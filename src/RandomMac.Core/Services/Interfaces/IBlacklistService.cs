using RandomMac.Core.Models;

namespace RandomMac.Core.Services.Interfaces;

public interface IBlacklistService
{
    // Global blacklist
    IReadOnlySet<MacAddress> GetBlacklist();
    void Add(MacAddress mac);
    void Remove(MacAddress mac);
    void Clear();

    // Per-adapter blacklist
    IReadOnlySet<MacAddress> GetAdapterBlacklist(string pnpDeviceId);
    void AddToAdapter(string pnpDeviceId, MacAddress mac);
    void RemoveFromAdapter(string pnpDeviceId, MacAddress mac);
    void ClearAdapter(string pnpDeviceId);

    /// <summary>
    /// Combined blacklist: global union adapter-specific.
    /// </summary>
    IReadOnlySet<MacAddress> GetEffectiveBlacklist(string pnpDeviceId);

    Task LoadAsync();
    Task SaveAsync();
    string GetFilePath();
}
