using Avalonia.Controls;
using Avalonia.Platform.Storage;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using Microsoft.Extensions.Logging;
using RandomMac.App.Localization;
using RandomMac.App.Services;
using RandomMac.Core.Helpers;
using RandomMac.Core.Services.Interfaces;
using System.Collections.ObjectModel;
using System.Diagnostics;

namespace RandomMac.App.ViewModels;

public partial class SettingsViewModel : ViewModelBase
{
    private readonly ISettingsService _settingsService;
    private readonly IBlacklistService _blacklistService;
    private readonly INetworkAdapterService _adapterService;
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
    private string _selectedAccentColor = "Blue";

    [ObservableProperty]
    private string _statusMessage = "";

    // Auto-change on startup
    [ObservableProperty]
    private bool _autoChangeOnStartup;

    public ObservableCollection<AutoChangeAdapterItem> AutoChangeAdapters { get; } = [];

    public string[] AvailableLanguages { get; } = ["en", "vi"];
    public string[] AvailableModes => ThemeService.AvailableModes;
    public string[] AvailableAccentColors => ThemeService.AvailableAccentColors;

    public SettingsViewModel(
        ISettingsService settingsService,
        IBlacklistService blacklistService,
        INetworkAdapterService adapterService,
        ThemeService themeService,
        ILogger<SettingsViewModel> logger)
    {
        _settingsService = settingsService;
        _blacklistService = blacklistService;
        _adapterService = adapterService;
        _themeService = themeService;
        _logger = logger;

        _isLoading = true;
        LoadFromSettings();
        _isLoading = false;
        LoadAutoChangeAdaptersCommand.ExecuteAsync(null);
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
        SelectedAccentColor = s.AccentColor;
        AutoChangeOnStartup = s.AutoChangeOnStartup;
    }

    [RelayCommand]
    private async Task LoadAutoChangeAdaptersAsync()
    {
        try
        {
            var adapters = await _adapterService.GetPhysicalAdaptersAsync();
            var configuredIds = _settingsService.Settings.AutoChangeAdapterIds;

            AutoChangeAdapters.Clear();
            foreach (var adapter in adapters)
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
        s.AccentColor = SelectedAccentColor;
        s.AutoChangeOnStartup = AutoChangeOnStartup;
        s.AutoChangeAdapterIds = AutoChangeAdapters
            .Where(a => a.IsSelected)
            .Select(a => a.PnpDeviceId)
            .ToList();

        // Apply theme + accent color immediately
        _themeService.Apply(SelectedThemeMode, SelectedAccentColor);

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
            var topLevel = TopLevel.GetTopLevel(
                (App.Services.GetService(typeof(Views.MainWindow)) as Window)!);
            if (topLevel is null) return;

            var file = await topLevel.StorageProvider.SaveFilePickerAsync(new FilePickerSaveOptions
            {
                Title = "Export Settings",
                DefaultExtension = "json",
                FileTypeChoices =
                [
                    new FilePickerFileType("JSON") { Patterns = ["*.json"] }
                ],
                SuggestedFileName = "randommac-settings.json"
            });

            if (file is not null)
            {
                await _settingsService.ExportAsync(file.Path.LocalPath);
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
            var topLevel = TopLevel.GetTopLevel(
                (App.Services.GetService(typeof(Views.MainWindow)) as Window)!);
            if (topLevel is null) return;

            var files = await topLevel.StorageProvider.OpenFilePickerAsync(new FilePickerOpenOptions
            {
                Title = "Import Settings",
                AllowMultiple = false,
                FileTypeFilter =
                [
                    new FilePickerFileType("JSON") { Patterns = ["*.json"] }
                ]
            });

            if (files.Count > 0)
            {
                await _settingsService.ImportAsync(files[0].Path.LocalPath);
                LoadFromSettings();
                _themeService.Apply(SelectedThemeMode, SelectedAccentColor);
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
