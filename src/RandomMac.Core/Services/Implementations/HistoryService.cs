using Microsoft.Extensions.Logging;
using RandomMac.Core.Models;
using RandomMac.Core.Services.Interfaces;
using System.Text.Json;

namespace RandomMac.Core.Services.Implementations;

public sealed class HistoryService : IHistoryService
{
    private readonly ILogger<HistoryService> _logger;
    private readonly string _filePath;
    private readonly List<MacHistoryEntry> _history = [];

    private static readonly JsonSerializerOptions JsonOptions = new()
    {
        WriteIndented = true
    };

    public HistoryService(ILogger<HistoryService> logger)
    {
        _logger = logger;
        _filePath = Path.Combine(
            Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData),
            "RandomMac", "history.json");
    }

    public IReadOnlyList<MacHistoryEntry> GetHistory() => _history.AsReadOnly();

    public string GetFilePath() => _filePath;

    public void Add(MacHistoryEntry entry)
    {
        _history.Insert(0, entry); // Most recent first

        // Keep max 100 entries
        if (_history.Count > 100)
            _history.RemoveRange(100, _history.Count - 100);

        _logger.LogDebug("History entry added: {Adapter} {Old} -> {New} ({Status})",
            entry.AdapterName, entry.PreviousMac, entry.NewMac, entry.Success ? "OK" : "FAIL");
    }

    public void Clear()
    {
        _history.Clear();
        _logger.LogInformation("History cleared");
    }

    public async Task LoadAsync()
    {
        try
        {
            if (!File.Exists(_filePath)) return;

            var json = await File.ReadAllTextAsync(_filePath);
            var entries = JsonSerializer.Deserialize<List<MacHistoryEntry>>(json);

            _history.Clear();
            if (entries is not null)
                _history.AddRange(entries);

            _logger.LogInformation("Loaded {Count} history entries", _history.Count);
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Failed to load history");
        }
    }

    public async Task SaveAsync()
    {
        try
        {
            var dir = Path.GetDirectoryName(_filePath)!;
            Directory.CreateDirectory(dir);

            var json = JsonSerializer.Serialize(_history, JsonOptions);
            await File.WriteAllTextAsync(_filePath, json);
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Failed to save history");
        }
    }
}
