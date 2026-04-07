using Microsoft.Extensions.Logging;
using RandomMac.Core.Models;
using RandomMac.Core.Services.Interfaces;
using System.Text.Json;

namespace RandomMac.Core.Services.Implementations;

public sealed class SettingsService : ISettingsService
{
    private readonly ILogger<SettingsService> _logger;
    private readonly string _filePath;

    private static readonly JsonSerializerOptions JsonOptions = new()
    {
        WriteIndented = true
    };

    public AppSettings Settings { get; private set; } = new();

    public SettingsService(ILogger<SettingsService> logger)
    {
        _logger = logger;
        _filePath = Path.Combine(
            Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData),
            "RandomMac", "settings.json");
    }

    public async Task LoadAsync()
    {
        try
        {
            if (!File.Exists(_filePath))
            {
                _logger.LogDebug("Settings file not found, using defaults");
                return;
            }

            var json = await File.ReadAllTextAsync(_filePath);
            var settings = JsonSerializer.Deserialize<AppSettings>(json);
            if (settings is not null)
                Settings = settings;

            _logger.LogInformation("Settings loaded from {Path}", _filePath);
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Failed to load settings, using defaults");
            Settings = new AppSettings();
        }
    }

    public async Task SaveAsync()
    {
        try
        {
            var dir = Path.GetDirectoryName(_filePath)!;
            Directory.CreateDirectory(dir);

            var json = JsonSerializer.Serialize(Settings, JsonOptions);
            await File.WriteAllTextAsync(_filePath, json);

            _logger.LogDebug("Settings saved");
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Failed to save settings");
        }
    }

    public async Task ExportAsync(string filePath)
    {
        var json = JsonSerializer.Serialize(Settings, JsonOptions);
        await File.WriteAllTextAsync(filePath, json);
        _logger.LogInformation("Settings exported to {Path}", filePath);
    }

    public async Task ImportAsync(string filePath)
    {
        var json = await File.ReadAllTextAsync(filePath);
        var settings = JsonSerializer.Deserialize<AppSettings>(json);
        if (settings is not null)
        {
            Settings = settings;
            await SaveAsync();
            _logger.LogInformation("Settings imported from {Path}", filePath);
        }
    }

    public string GetFilePath() => _filePath;
}
