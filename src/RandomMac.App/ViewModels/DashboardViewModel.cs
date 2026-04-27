using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using Microsoft.Extensions.Logging;
using RandomMac.App.Localization;
using RandomMac.App.Services;
using RandomMac.Core.Helpers;
using RandomMac.Core.Models;
using RandomMac.Core.Services.Interfaces;
using System.Collections.ObjectModel;

namespace RandomMac.App.ViewModels;

public partial class DashboardViewModel : ViewModelBase
{
    private readonly INetworkAdapterService _adapterService;
    private readonly IAdapterCacheService _cache;
    private readonly IMacAddressService _macService;
    private readonly IBlacklistService _blacklistService;
    private readonly IHistoryService _historyService;
    private readonly NotificationService _notificationService;
    private readonly ILogger<DashboardViewModel> _logger;

    public override string Title => "Dashboard";
    public override string IconKey => "Home";

    public ObservableCollection<NetworkAdapterInfo> Adapters { get; } = [];
    public ObservableCollection<MacHistoryEntry> RecentHistory { get; } = [];

    [ObservableProperty]
    private NetworkAdapterInfo? _selectedAdapter;

    [ObservableProperty]
    private string _currentMacDisplay = "--";

    [ObservableProperty]
    private string _originalMacDisplay = "--";

    [ObservableProperty]
    private string _previewMac = "--";

    private const int StatusAutoClearMs = 5000;
    private CancellationTokenSource? _statusClearCts;

    [ObservableProperty]
    private string _statusMessage = "";

    /// <summary>True when StatusMessage has user-facing content (drives the InfoBar).</summary>
    public bool HasStatusMessage => !string.IsNullOrEmpty(StatusMessage);

    partial void OnStatusMessageChanged(string value)
    {
        OnPropertyChanged(nameof(HasStatusMessage));

        _statusClearCts?.Cancel();
        if (string.IsNullOrEmpty(value)) return;

        _statusClearCts = new CancellationTokenSource();
        _ = ClearStatusAfterDelayAsync(value, _statusClearCts.Token);
    }

    private async Task ClearStatusAfterDelayAsync(string snapshot, CancellationToken token)
    {
        try { await Task.Delay(StatusAutoClearMs, token); }
        catch (OperationCanceledException) { return; }

        if (StatusMessage != snapshot) return; // already replaced by a newer message

        if (App.MainDispatcher is { } d && !d.HasThreadAccess)
            d.TryEnqueue(() => { if (StatusMessage == snapshot) StatusMessage = ""; });
        else
            StatusMessage = "";
    }

    [ObservableProperty]
    private bool _isLoading;

    [ObservableProperty]
    private bool _isChanging;

    [ObservableProperty]
    private bool _hasPreview;

    [ObservableProperty]
    private string _adapterType = "";

    [ObservableProperty]
    private bool _isConnected;

    [ObservableProperty]
    private string _currentVendor = "";

    [ObservableProperty]
    private string _previewVendor = "";

    [ObservableProperty]
    private bool _isMacChanged;

    public DashboardViewModel(
        INetworkAdapterService adapterService,
        IAdapterCacheService cache,
        IMacAddressService macService,
        IBlacklistService blacklistService,
        IHistoryService historyService,
        NotificationService notificationService,
        ILogger<DashboardViewModel> logger)
    {
        _adapterService = adapterService;
        _cache = cache;
        _macService = macService;
        _blacklistService = blacklistService;
        _historyService = historyService;
        _notificationService = notificationService;
        _logger = logger;

        // Read the warmed-up cache synchronously. App.OnLaunched calls
        // EnsureLoadedAsync before this VM is constructed.
        PopulateAdaptersFromCache();
        PopulateRecentHistory();

        // Repopulate when the cache is refreshed (user clicks Refresh, or
        // SettingsViewModel triggers a rescan).
        _cache.AdaptersRefreshed += OnAdaptersRefreshed;
    }

    private void OnAdaptersRefreshed(object? sender, EventArgs e)
    {
        // The cache fires this event from whatever thread RefreshAsync's
        // continuation lands on (threadpool, due to ConfigureAwait(false)).
        // Marshal back to the UI thread before mutating the bound
        // ObservableCollection — otherwise we get a cross-thread exception
        // that LoadAdaptersAsync's catch block surfaces as "Error loading
        // adapters" on every Refresh click.
        var dispatcher = App.MainDispatcher;
        if (dispatcher is null || dispatcher.HasThreadAccess)
            PopulateAdaptersFromCache();
        else
            dispatcher.TryEnqueue(PopulateAdaptersFromCache);
    }

    private void PopulateAdaptersFromCache()
    {
        var current = SelectedAdapter?.PnpDeviceId;
        Adapters.Clear();
        foreach (var a in _cache.Adapters)
            Adapters.Add(a);

        if (current is not null)
            SelectedAdapter = Adapters.FirstOrDefault(a => a.PnpDeviceId == current);

        StatusMessage = $"Found {Adapters.Count} adapter(s).";
    }

    private void PopulateRecentHistory()
    {
        RecentHistory.Clear();
        foreach (var entry in _historyService.GetHistory())
            RecentHistory.Add(entry);
    }

    partial void OnSelectedAdapterChanged(NetworkAdapterInfo? value)
    {
        if (value is null)
        {
            CurrentMacDisplay = "--";
            OriginalMacDisplay = "--";
            AdapterType = "";
            IsConnected = false;
            CurrentVendor = "";
            PreviewVendor = "";
            // Empty-state hint is shown by the InfoBar in DashboardView; no
            // need to populate StatusMessage with a "select an adapter" prompt.
            StatusMessage = "";
            HasPreview = false;
            return;
        }

        CurrentMacDisplay = value.CurrentMac.ToDisplayString();
        OriginalMacDisplay = value.OriginalMac.ToDisplayString();
        AdapterType = value.Type.ToString();
        IsConnected = value.IsConnected;
        CurrentVendor = OuiLookup.GetVendor(value.CurrentMac) ?? "Unknown";
        HasPreview = false;
        PreviewMac = "--";
        PreviewVendor = "";
        StatusMessage = $"Selected: {value.Name}";
    }

    [RelayCommand]
    private async Task LoadAdaptersAsync()
    {
        IsLoading = true;
        StatusMessage = "Detecting adapters...";

        try
        {
            await _cache.RefreshAsync();
            // PopulateAdaptersFromCache runs via the AdaptersRefreshed event.
            _logger.LogInformation("Loaded {Count} physical adapters", _cache.Adapters.Count);
        }
        catch (Exception ex)
        {
            StatusMessage = $"Error loading adapters: {ex.Message}";
            _logger.LogError(ex, "Failed to load adapters");
        }
        finally
        {
            IsLoading = false;
        }

        PopulateRecentHistory();
    }

    [RelayCommand]
    private void GeneratePreview()
    {
        if (SelectedAdapter is null) return;

        if (!IsConnected)
        {
            StatusMessage = "Adapter is disconnected. Cannot randomize.";
            _notificationService.Warning(Loc.Get("Notif_Disconnected"));
            _logger.LogWarning("Attempted to generate preview on disconnected adapter");
            return;
        }

        var blacklist = !string.IsNullOrEmpty(SelectedAdapter.PnpDeviceId)
            ? _blacklistService.GetEffectiveBlacklist(SelectedAdapter.PnpDeviceId)
            : _blacklistService.GetBlacklist();
        var mac = MacAddressGenerator.Generate(blacklist);

        if (mac is null)
        {
            StatusMessage = "Failed to generate a non-blacklisted MAC after retries.";
            _logger.LogWarning("MAC generation exhausted retries (blacklist size: {Size})", blacklist.Count);
            return;
        }

        PreviewMac = mac.Value.ToDisplayString();
        PreviewVendor = OuiLookup.GetVendor(mac.Value) ?? "Unknown";
        HasPreview = true;
        StatusMessage = "Preview generated. Click 'Apply' to change.";
        _logger.LogDebug("Generated preview MAC: {Mac}", mac.Value);
    }

    [RelayCommand]
    private async Task ApplyMacAsync()
    {
        if (SelectedAdapter is null || !HasPreview) return;
        if (!MacAddress.TryParse(PreviewMac, out var newMac)) return;

        IsChanging = true;
        StatusMessage = "Applying MAC address...";

        try
        {
            var result = await _macService.ChangeMacAsync(SelectedAdapter, newMac);

            // Record history
            var historyEntry = new MacHistoryEntry
            {
                AdapterName = SelectedAdapter.Name,
                AdapterDeviceId = SelectedAdapter.DeviceId,
                PreviousMac = result.PreviousMac.ToDisplayString(),
                NewMac = result.NewMac.ToDisplayString(),
                Success = result.Success,
                ErrorMessage = result.ErrorMessage,
                Timestamp = result.Timestamp
            };
            _historyService.Add(historyEntry);
            RecentHistory.Insert(0, historyEntry);
            await _historyService.SaveAsync();

            if (result.Success)
            {
                SelectedAdapter.CurrentMac = result.VerifiedMac;
                CurrentMacDisplay = result.VerifiedMac.ToDisplayString();
                CurrentVendor = OuiLookup.GetVendor(result.VerifiedMac) ?? "Unknown";
                HasPreview = false;
                IsMacChanged = CurrentMacDisplay != OriginalMacDisplay;
                StatusMessage = $"MAC changed successfully to {result.VerifiedMac}";
                _notificationService.Success(Loc.Get("Notif_MacChanged", result.VerifiedMac));

                // Auto-refresh adapter after 3s to update connection status
                _ = Task.Run(async () =>
                {
                    await Task.Delay(3000);
                    await RefreshAdapterAsync();
                });
            }
            else
            {
                // Add failed MAC to adapter-specific blacklist
                if (!string.IsNullOrEmpty(SelectedAdapter.PnpDeviceId))
                    _blacklistService.AddToAdapter(SelectedAdapter.PnpDeviceId, newMac);
                else
                    _blacklistService.Add(newMac);
                await _blacklistService.SaveAsync();

                StatusMessage = $"Failed: {result.ErrorMessage}";
                _notificationService.Error(Loc.Get("Notif_MacFailed", result.ErrorMessage ?? ""));
                _logger.LogWarning("MAC change failed, added {Mac} to blacklist", newMac);
            }
        }
        catch (Exception ex)
        {
            StatusMessage = $"Error: {ex.Message}";
            _logger.LogError(ex, "Unexpected error during MAC change");
        }
        finally
        {
            IsChanging = false;
        }
    }

    [RelayCommand]
    private async Task RestoreOriginalAsync()
    {
        if (SelectedAdapter is null) return;

        IsChanging = true;
        StatusMessage = "Restoring original MAC...";

        try
        {
            var result = await _macService.RestoreOriginalMacAsync(SelectedAdapter);

            if (result.Success)
            {
                SelectedAdapter.CurrentMac = result.VerifiedMac;
                CurrentMacDisplay = result.VerifiedMac.ToDisplayString();
                HasPreview = false;
                PreviewMac = "--";
                IsMacChanged = false;
                CurrentVendor = OuiLookup.GetVendor(result.VerifiedMac) ?? "Unknown";
                StatusMessage = "Original MAC restored.";
                _notificationService.Success(Loc.Get("Notif_Restored"));
            }
            else
            {
                StatusMessage = $"Restore failed: {result.ErrorMessage}";
                _notificationService.Error(Loc.Get("Notif_RestoreFailed", result.ErrorMessage ?? ""));
            }
        }
        catch (Exception ex)
        {
            StatusMessage = $"Error: {ex.Message}";
            _logger.LogError(ex, "Failed to restore original MAC");
        }
        finally
        {
            IsChanging = false;
        }
    }

    [RelayCommand]
    private async Task RefreshAdapterAsync()
    {
        if (SelectedAdapter is null) return;

        var refreshed = await _adapterService.RefreshAdapterAsync(SelectedAdapter.DeviceId);
        if (refreshed is not null)
        {
            SelectedAdapter.CurrentMac = refreshed.CurrentMac;
            SelectedAdapter.IsConnected = refreshed.IsConnected;
            SelectedAdapter.IsEnabled = refreshed.IsEnabled;
            CurrentMacDisplay = refreshed.CurrentMac.ToDisplayString();
            CurrentVendor = OuiLookup.GetVendor(refreshed.CurrentMac) ?? "Unknown";
            IsConnected = refreshed.IsConnected;
            StatusMessage = "Adapter info refreshed.";
        }
    }

    [RelayCommand]
    private void ClearHistory()
    {
        _historyService.Clear();
        RecentHistory.Clear();
        StatusMessage = "History cleared (in-memory only).";
        _logger.LogInformation("History cleared by user (file retained)");
    }

    [RelayCommand]
    private void CopyMac(string? mac)
    {
        if (string.IsNullOrEmpty(mac) || mac == "--") return;

        try
        {
            var package = new Windows.ApplicationModel.DataTransfer.DataPackage();
            package.SetText(mac);
            Windows.ApplicationModel.DataTransfer.Clipboard.SetContent(package);
            _notificationService.Info(Loc.Get("Notif_Copied", mac));
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Failed to copy MAC to clipboard");
        }
    }
}
