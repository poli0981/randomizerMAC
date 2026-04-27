using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Logging;
using RandomMac.App.Localization;
using RandomMac.App.Services;
using RandomMac.Core.Helpers;
using RandomMac.Core.Services.Interfaces;
using System.Collections.ObjectModel;
using System.Diagnostics;
using Windows.Storage.Pickers;

namespace RandomMac.App.ViewModels;

public partial class SettingsViewModel : ViewModelBase
{
    private readonly ISettingsService _settingsService;
    private readonly IBlacklistService _blacklistService;
    private readonly IAdapterCacheService _cache;
    private readonly ThemeService _themeService;
    private readonly ILogger<SettingsViewModel> _logger;

    private bool _isLoading;

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

    // Auto-change on startup
    [ObservableProperty]
    private bool _autoChangeOnStartup;

    public ObservableCollection<AutoChangeAdapterItem> AutoChangeAdapters { get; } = [];

    public string[] AvailableLanguages { get; } = ["en", "vi"];
    public string[] AvailableModes => ThemeService.AvailableModes;

    public SettingsViewModel(
        ISettingsService settingsService,
        IBlacklistService blacklistService,
        IAdapterCacheService cache,
        ThemeService themeService,
        ILogger<SettingsViewModel> logger)
    {
        _settingsService = settingsService;
        _blacklistService = blacklistService;
        _cache = cache;
        _themeService = themeService;
        _logger = logger;

        _isLoading = true;
        LoadFromSettings();
        _isLoading = false;

        // Read warmed-up cache synchronously (App.OnLaunched calls
        // EnsureLoadedAsync before this VM is constructed).
        PopulateAutoChangeAdaptersFromCache();
        _cache.AdaptersRefreshed += (_, _) => PopulateAutoChangeAdaptersFromCache();
    }

    private void PopulateAutoChangeAdaptersFromCache()
    {
        var configuredIds = _settingsService.Settings.AutoChangeAdapterIds;
        AutoChangeAdapters.Clear();
        foreach (var adapter in _cache.Adapters)
        {
            AutoChangeAdapters.Add(new AutoChangeAdapterItem
            {
                PnpDeviceId = adapter.PnpDeviceId,
                Name = adapter.Name,
                Type = adapter.Type.ToString(),
                IsSelected = configuredIds.Contains(adapter.PnpDeviceId)
            });
        }
    }

    /// <summary>
    /// Apply language immediately when ComboBox selection changes (not during loading).
    /// </summary>
    partial void OnSelectedLanguageChanged(string value)
    {
        if (!_isLoading && !string.IsNullOrEmpty(value))
        {
            Loc.SetLanguage(value);
        }
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
    private async Task SaveSettingsAsync()
    {
        var s = _settingsService.Settings;
        s.Language = SelectedLanguage;
        s.RunAtStartup = RunAtStartup;
        s.MinimizeToTray = MinimizeToTray;
        s.StartMinimized = StartMinimized;
        s.ShowNotifications = ShowNotifications;
        s.ThemeMode = SelectedThemeMode;
        s.AutoChangeOnStartup = AutoChangeOnStartup;
        s.AutoChangeAdapterIds = AutoChangeAdapters
            .Where(a => a.IsSelected)
            .Select(a => a.PnpDeviceId)
            .ToList();

        // Apply theme immediately. Accent is fixed brand color (ignored arg).
        _themeService.Apply(SelectedThemeMode, "");

        // Ensure language is applied (may already be applied via OnSelectedLanguageChanged)
        if (Loc.CurrentLanguage != SelectedLanguage)
            Loc.SetLanguage(SelectedLanguage);

        // Apply startup setting
        var exePath = Environment.ProcessPath;
        if (!RegistryHelper.SetStartup(RunAtStartup, exePath, StartMinimized))
            _logger.LogWarning("Failed to write startup registry entry");

        await _settingsService.SaveAsync();
        StatusMessage = Loc.Get("Settings_Saved");
        _logger.LogInformation("Settings saved");
    }

    [RelayCommand]
    private async Task ExportSettingsAsync()
    {
        try
        {
            var picker = new FileSavePicker
            {
                SuggestedStartLocation = PickerLocationId.Desktop,
                SuggestedFileName = "randommac-settings",
                DefaultFileExtension = ".json"
            };
            picker.FileTypeChoices.Add("JSON", [".json"]);

            InitWithMainWindow(picker);

            var file = await picker.PickSaveFileAsync();
            if (file is not null)
            {
                await _settingsService.ExportAsync(file.Path);
                StatusMessage = "Settings exported.";
            }
        }
        catch (Exception ex)
        {
            StatusMessage = $"Export failed: {ex.Message}";
            _logger.LogError(ex, "Failed to export settings");
        }
    }

    [RelayCommand]
    private async Task ImportSettingsAsync()
    {
        try
        {
            var picker = new FileOpenPicker
            {
                SuggestedStartLocation = PickerLocationId.Desktop,
            };
            picker.FileTypeFilter.Add(".json");

            InitWithMainWindow(picker);

            var file = await picker.PickSingleFileAsync();
            if (file is not null)
            {
                await _settingsService.ImportAsync(file.Path);
                LoadFromSettings();
                _themeService.Apply(SelectedThemeMode, "");
                Loc.SetLanguage(SelectedLanguage);
                StatusMessage = "Settings imported.";
            }
        }
        catch (Exception ex)
        {
            StatusMessage = $"Import failed: {ex.Message}";
            _logger.LogError(ex, "Failed to import settings");
        }
    }

    private static void InitWithMainWindow(object pickerOrDialog)
    {
        var hwnd = WinRT.Interop.WindowNative.GetWindowHandle(
            App.Services.GetRequiredService<MainWindow>());
        WinRT.Interop.InitializeWithWindow.Initialize(pickerOrDialog, hwnd);
    }

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
