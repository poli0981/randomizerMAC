using Microsoft.Extensions.Logging;
using RandomMac.Core.Models;
using RandomMac.Core.Services.Interfaces;

namespace RandomMac.Core.Services.Implementations;

public sealed class AdapterCacheService : IAdapterCacheService
{
    private readonly INetworkAdapterService _service;
    private readonly ILogger<AdapterCacheService> _logger;

    private readonly SemaphoreSlim _loadLock = new(1, 1);
    private List<NetworkAdapterInfo> _adapters = [];
    private bool _hasLoaded;

    public AdapterCacheService(INetworkAdapterService service, ILogger<AdapterCacheService> logger)
    {
        _service = service;
        _logger = logger;
    }

    public IReadOnlyList<NetworkAdapterInfo> Adapters => _adapters;

    public event EventHandler? AdaptersRefreshed;

    public async Task EnsureLoadedAsync()
    {
        if (_hasLoaded) return;

        await _loadLock.WaitAsync().ConfigureAwait(false);
        try
        {
            // Double-checked: another caller may have completed during the wait.
            if (_hasLoaded) return;

            await LoadInternalAsync().ConfigureAwait(false);
            _hasLoaded = true;
        }
        finally
        {
            _loadLock.Release();
        }

        AdaptersRefreshed?.Invoke(this, EventArgs.Empty);
    }

    public async Task RefreshAsync()
    {
        await _loadLock.WaitAsync().ConfigureAwait(false);
        try
        {
            await LoadInternalAsync().ConfigureAwait(false);
            _hasLoaded = true;
        }
        finally
        {
            _loadLock.Release();
        }

        AdaptersRefreshed?.Invoke(this, EventArgs.Empty);
    }

    private async Task LoadInternalAsync()
    {
        var fresh = await _service.GetPhysicalAdaptersAsync().ConfigureAwait(false);
        _adapters = [.. fresh];
        _logger.LogDebug("AdapterCacheService loaded {Count} adapters", _adapters.Count);
    }
}
