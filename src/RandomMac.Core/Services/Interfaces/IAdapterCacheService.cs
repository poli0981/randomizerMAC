using RandomMac.Core.Models;

namespace RandomMac.Core.Services.Interfaces;

/// <summary>
/// Single-flight cache around <see cref="INetworkAdapterService.GetPhysicalAdaptersAsync"/>
/// to prevent duplicate WMI scans (and the duplicate "Detected adapter" /
/// "Excluded adapter" log block they emit) when multiple ViewModels resolve
/// simultaneously at startup.
/// </summary>
public interface IAdapterCacheService
{
    /// <summary>
    /// Last-known adapter list. Empty until <see cref="EnsureLoadedAsync"/>
    /// (or <see cref="RefreshAsync"/>) has completed at least once.
    /// </summary>
    IReadOnlyList<NetworkAdapterInfo> Adapters { get; }

    /// <summary>
    /// Loads the adapter list once. Subsequent calls are no-ops; concurrent
    /// callers all await the in-flight load and observe the same result.
    /// </summary>
    Task EnsureLoadedAsync();

    /// <summary>
    /// Forces a fresh WMI scan, replaces the cache, and raises
    /// <see cref="AdaptersRefreshed"/>. Use for user-triggered "Refresh"
    /// buttons. Concurrent calls are serialized by the same lock as
    /// <see cref="EnsureLoadedAsync"/>.
    /// </summary>
    Task RefreshAsync();

    /// <summary>
    /// Raised after every successful load or refresh. ViewModels subscribe
    /// to repopulate their bound collections.
    /// </summary>
    event EventHandler AdaptersRefreshed;
}
