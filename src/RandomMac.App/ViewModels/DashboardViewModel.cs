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

    [ObservableProperty]
    private string _statusMessage = "Select an adapter to get started.";

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
        IMacAddressService macService,
        IBlacklistService blacklistService,
        IHistoryService historyService,
        NotificationService notificationService,
        ILogger<DashboardViewModel> logger)
    {
        _adapterService = adapterService;
        _macService = macService;
        _blacklistService = blacklistService;
        _historyService = historyService;
        _notificationService = notificationService;
        _logger = logger;

        LoadAdaptersCommand.ExecuteAsync(null);
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
            StatusMessage = "Select an adapter to get started.";
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
            var adapters = await _adapterService.GetPhysicalAdaptersAsync();
            Adapters.Clear();
            foreach (var a in adapters)
                Adapters.Add(a);

            StatusMessage = $"Found {adapters.Count} adapter(s).";
            _logger.LogInformation("Loaded {Count} physical adapters", adapters.Count);
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

        // Load recent history
        RecentHistory.Clear();
        foreach (var entry in _historyService.GetHistory())
            RecentHistory.Add(entry);
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
    private async Task CopyMacAsync(string? mac)
    {
        if (string.IsNullOrEmpty(mac) || mac == "--") return;

        try
        {
            var topLevel = Avalonia.Controls.TopLevel.GetTopLevel(
                (App.Services.GetService(typeof(Views.MainWindow)) as Avalonia.Controls.Window)!);
            if (topLevel?.Clipboard is not null)
            {
                await topLevel.Clipboard.SetTextAsync(mac);
                _notificationService.Info(Loc.Get("Notif_Copied", mac));
            }
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Failed to copy MAC to clipboard");
        }
    }
}
