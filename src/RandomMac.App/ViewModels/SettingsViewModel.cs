using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Logging;
using RandomMac.App.Helpers;
using RandomMac.App.Localization;
using RandomMac.App.Services;
using RandomMac.Core.Helpers;
using RandomMac.Core.Services.Interfaces;
using System.Collections.ObjectModel;
using System.ComponentModel;
using System.Diagnostics;
using System.IO.Compression;

namespace RandomMac.App.ViewModels;

public partial class SettingsViewModel : ViewModelBase
{
    /// <summary>
    /// Debounce window before SaveAsync runs after the last property change.
    /// </summary>
    private const int SaveDebounceMs = 500;

    private readonly ISettingsService _settingsService;
    private readonly IBlacklistService _blacklistService;
    private readonly IHistoryService _historyService;
    private readonly IAdapterCacheService _cache;
    private readonly ThemeService _themeService;
    private readonly ILogger<SettingsViewModel> _logger;

    private bool _isLoading;
    private CancellationTokenSource? _saveCts;

    public override string Title => "Settings";
    public override string IconKey => "Settings";

    [ObservableProperty]
    private string _selectedLanguage = "en";

    [ObservableProperty]
    private bool _runAtStartup;

    [ObservableProperty]
    private bool _minimizeToTray = true;

    [ObservableProperty]
    private bool _startMinimized;

    [ObservableProperty]
    private bool _showNotifications = true;

    [ObservableProperty]
    private string _selectedThemeMode = "Dark";

    [ObservableProperty]
    private string _statusMessage = "";

    [ObservableProperty]
    private bool _autoChangeOnStartup;

    public ObservableCollection<AutoChangeAdapterItem> AutoChangeAdapters { get; } = [];

    public string[] AvailableLanguages { get; } = ["en", "vi"];
    public string[] AvailableModes => ThemeService.AvailableModes;

    public SettingsViewModel(
        ISettingsService settingsService,
        IBlacklistService blacklistService,
        IHistoryService historyService,
        IAdapterCacheService cache,
        ThemeService themeService,
        ILogger<SettingsViewModel> logger)
    {
        _settingsService = settingsService;
        _blacklistService = blacklistService;
        _historyService = historyService;
        _cache = cache;
        _themeService = themeService;
        _logger = logger;

        _isLoading = true;
        LoadFromSettings();
        _isLoading = false;

        PopulateAutoChangeAdaptersFromCache();
        _cache.AdaptersRefreshed += OnCacheRefreshed;
    }

    private void OnCacheRefreshed(object? sender, EventArgs e)
    {
        // Marshal to UI thread — cache fires this from threadpool because
        // RefreshAsync uses ConfigureAwait(false) internally.
        var dispatcher = App.MainDispatcher;
        if (dispatcher is null || dispatcher.HasThreadAccess)
            PopulateAutoChangeAdaptersFromCache();
        else
            dispatcher.TryEnqueue(PopulateAutoChangeAdaptersFromCache);
    }

    private void LoadFromSettings()
    {
        var s = _settingsService.Settings;
        SelectedLanguage = s.Language;
        RunAtStartup = s.RunAtStartup;
        MinimizeToTray = s.MinimizeToTray;
        StartMinimized = s.StartMinimized;
        ShowNotifications = s.ShowNotifications;
        SelectedThemeMode = s.ThemeMode;
        AutoChangeOnStartup = s.AutoChangeOnStartup;
    }

    private void PopulateAutoChangeAdaptersFromCache()
    {
        // Detach old subscriptions before clearing.
        foreach (var existing in AutoChangeAdapters)
            existing.PropertyChanged -= OnAutoChangeItemChanged;

        var configuredIds = _settingsService.Settings.AutoChangeAdapterIds;
        AutoChangeAdapters.Clear();
        foreach (var adapter in _cache.Adapters)
        {
            var item = new AutoChangeAdapterItem
            {
                PnpDeviceId = adapter.PnpDeviceId,
                Name = adapter.Name,
                Type = adapter.Type.ToString(),
                IsSelected = configuredIds.Contains(adapter.PnpDeviceId)
            };
            item.PropertyChanged += OnAutoChangeItemChanged;
            AutoChangeAdapters.Add(item);
        }
    }

    private void OnAutoChangeItemChanged(object? sender, PropertyChangedEventArgs e)
    {
        if (_isLoading) return;
        if (e.PropertyName != nameof(AutoChangeAdapterItem.IsSelected)) return;

        _settingsService.Settings.AutoChangeAdapterIds = AutoChangeAdapters
            .Where(a => a.IsSelected)
            .Select(a => a.PnpDeviceId)
            .ToList();
        ScheduleSave();
    }

    // ---------- Auto-apply property change handlers ----------

    partial void OnSelectedLanguageChanged(string value)
    {
        if (_isLoading || string.IsNullOrEmpty(value)) return;

        Loc.SetLanguage(value);
        _settingsService.Settings.Language = value;
        ScheduleSave();
    }

    partial void OnSelectedThemeModeChanged(string value)
    {
        if (_isLoading || string.IsNullOrEmpty(value)) return;

        _themeService.ApplyThemeMode(value);
        _settingsService.Settings.ThemeMode = value;
        ScheduleSave();
    }

    partial void OnShowNotificationsChanged(bool value)
    {
        if (_isLoading) return;
        _settingsService.Settings.ShowNotifications = value;
        ScheduleSave();
    }

    partial void OnRunAtStartupChanged(bool value)
    {
        if (_isLoading) return;
        _settingsService.Settings.RunAtStartup = value;

        var exePath = Environment.ProcessPath;
        if (!RegistryHelper.SetStartup(value, exePath, StartMinimized))
            _logger.LogWarning("Failed to update startup task (RunAtStartup={Value})", value);

        ScheduleSave();
    }

    partial void OnStartMinimizedChanged(bool value)
    {
        if (_isLoading) return;
        _settingsService.Settings.StartMinimized = value;

        // Re-register the scheduled task with the new --minimized flag, but
        // only if startup is enabled (otherwise the change has nothing to apply yet).
        if (RunAtStartup)
        {
            var exePath = Environment.ProcessPath;
            RegistryHelper.SetStartup(true, exePath, value);
        }

        ScheduleSave();
    }

    partial void OnMinimizeToTrayChanged(bool value)
    {
        if (_isLoading) return;
        _settingsService.Settings.MinimizeToTray = value;
        ScheduleSave();
    }

    partial void OnAutoChangeOnStartupChanged(bool value)
    {
        if (_isLoading) return;
        _settingsService.Settings.AutoChangeOnStartup = value;
        ScheduleSave();
    }

    // ---------- Debounced persistence ----------

    /// <summary>
    /// Schedules an asynchronous SaveAsync after <see cref="SaveDebounceMs"/>.
    /// Subsequent calls within that window cancel the previous schedule, so a
    /// burst of property changes (e.g. flipping multiple toggles) collapses
    /// into a single disk write.
    /// </summary>
    private void ScheduleSave()
    {
        _saveCts?.Cancel();
        _saveCts = new CancellationTokenSource();
        _ = SaveAfterDelayAsync(_saveCts.Token);
    }

    private async Task SaveAfterDelayAsync(CancellationToken token)
    {
        try
        {
            await Task.Delay(SaveDebounceMs, token);
        }
        catch (OperationCanceledException)
        {
            return;
        }

        try
        {
            await _settingsService.SaveAsync();
            _logger.LogDebug("Settings auto-saved");
        }
        catch (Exception ex)
        {
            StatusMessage = $"Save failed: {ex.Message}";
            _logger.LogError(ex, "Settings auto-save failed");
        }
    }

    // ---------- Refresh / Export / Import / Open ----------

    [RelayCommand]
    private async Task LoadAutoChangeAdaptersAsync()
    {
        try
        {
            await _cache.RefreshAsync();
            // PopulateAutoChangeAdaptersFromCache runs via AdaptersRefreshed event.
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Failed to load adapters for auto-change config");
        }
    }

    [RelayCommand]
    private async Task ExportSettingsAsync()
    {
        try
        {
            var path = Win32FileDialog.PickSave(
                MainWindowHwnd(),
                "Export Settings",
                "randommac-settings.json",
                ".json",
                ("JSON files (*.json)", "*.json"));

            if (string.IsNullOrEmpty(path)) return;

            await _settingsService.ExportAsync(path);
            StatusMessage = "Settings exported.";
        }
        catch (Exception ex)
        {
            StatusMessage = $"Export failed: {ex.Message}";
            _logger.LogError(ex, "Failed to export settings");
        }
    }

    /// <summary>
    /// Export settings + blacklist + history into a single ZIP — convenient
    /// for backing up the full app state or moving between machines.
    /// </summary>
    [RelayCommand]
    private async Task ExportBundleAsync()
    {
        try
        {
            var path = Win32FileDialog.PickSave(
                MainWindowHwnd(),
                "Export bundle (settings + blacklist + history)",
                $"randommac-bundle-{DateTime.Now:yyyyMMdd-HHmmss}.zip",
                ".zip",
                ("ZIP archive (*.zip)", "*.zip"));

            if (string.IsNullOrEmpty(path)) return;

            var sources = new (string label, string path)[]
            {
                ("settings.json",  _settingsService.GetFilePath()),
                ("blacklist.json", _blacklistService.GetFilePath()),
                ("history.json",   _historyService.GetFilePath()),
            };

            // FileMode.Create truncates if the path already exists.
            await using var stream = new FileStream(
                path, FileMode.Create, FileAccess.Write, FileShare.None);
            using var zip = new ZipArchive(stream, ZipArchiveMode.Create, leaveOpen: false);
            foreach (var (label, src) in sources)
            {
                if (!File.Exists(src))
                {
                    _logger.LogDebug("Export bundle: {Label} missing at {Path}, skipped", label, src);
                    continue;
                }

                var entry = zip.CreateEntry(label, CompressionLevel.Optimal);
                using var entryStream = entry.Open();
                await using var srcStream = File.OpenRead(src);
                await srcStream.CopyToAsync(entryStream);
            }

            StatusMessage = $"Bundle exported to {Path.GetFileName(path)}.";
        }
        catch (Exception ex)
        {
            StatusMessage = $"Export failed: {ex.Message}";
            _logger.LogError(ex, "Failed to export bundle");
        }
    }

    [RelayCommand]
    private async Task ImportSettingsAsync()
    {
        try
        {
            var path = Win32FileDialog.PickOpen(
                MainWindowHwnd(),
                "Import Settings",
                ".json",
                ("JSON files (*.json)", "*.json"));

            if (string.IsNullOrEmpty(path)) return;

            await _settingsService.ImportAsync(path);

            _isLoading = true;
            LoadFromSettings();
            _isLoading = false;

            _themeService.Apply(SelectedThemeMode);
            Loc.SetLanguage(SelectedLanguage);
            PopulateAutoChangeAdaptersFromCache();
            StatusMessage = "Settings imported.";
        }
        catch (Exception ex)
        {
            StatusMessage = $"Import failed: {ex.Message}";
            _logger.LogError(ex, "Failed to import settings");
        }
    }

    private static IntPtr MainWindowHwnd() =>
        WinRT.Interop.WindowNative.GetWindowHandle(
            App.Services.GetRequiredService<MainWindow>());

    [RelayCommand]
    private void OpenSettingsFile() => OpenInExplorer(_settingsService.GetFilePath());

    [RelayCommand]
    private void OpenBlacklistFile() => OpenInExplorer(_blacklistService.GetFilePath());

    private void OpenInExplorer(string filePath)
    {
        try
        {
            var dir = Path.GetDirectoryName(filePath);
            if (dir is not null && Directory.Exists(dir))
            {
                Process.Start(new ProcessStartInfo
                {
                    FileName = "explorer.exe",
                    Arguments = File.Exists(filePath)
                        ? $"/select,\"{filePath}\""
                        : $"\"{dir}\"",
                    UseShellExecute = false
                });
            }
            else
            {
                StatusMessage = "File or directory not found.";
            }
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Failed to open explorer for {Path}", filePath);
        }
    }
}

public partial class AutoChangeAdapterItem : ObservableObject
{
    public string PnpDeviceId { get; init; } = "";
    public string Name { get; init; } = "";
    public string Type { get; init; } = "";

    [ObservableProperty]
    private bool _isSelected;
}
