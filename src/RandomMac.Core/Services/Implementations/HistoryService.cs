using Microsoft.Extensions.Logging;
using RandomMac.Core.Models;
using RandomMac.Core.Services.Interfaces;
using System.Text.Json;

namespace RandomMac.Core.Services.Implementations;

public sealed class HistoryService : IHistoryService
{
    /// <summary>Entries older than this are pruned on load and on every Add.</summary>
    private static readonly TimeSpan RetentionWindow = TimeSpan.FromDays(30);

    /// <summary>Hard cap on total entries (prevents unbounded growth on busy machines).</summary>
    private const int MaxEntries = 100;

    private readonly ILogger<HistoryService> _logger;
    private readonly string _filePath;
    private readonly List<MacHistoryEntry> _history = [];

    /// <summary>
    /// Set true when LoadAsync ran successfully (file missing or parsed OK).
    /// SaveAsync refuses to overwrite while this is false to prevent the
    /// "load failed silently → next save wipes the user's history" bug.
    /// </summary>
    private bool _loadCompleted;

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
        Prune();

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
        if (!File.Exists(_filePath))
        {
            _loadCompleted = true; // No file is a valid empty state
            return;
        }

        try
        {
            var json = await File.ReadAllTextAsync(_filePath);
            if (string.IsNullOrWhiteSpace(json))
            {
                _loadCompleted = true;
                return;
            }

            var entries = JsonSerializer.Deserialize<List<MacHistoryEntry>>(json);

            _history.Clear();
            if (entries is not null)
                _history.AddRange(entries);

            Prune();
            _loadCompleted = true;
            _logger.LogInformation("Loaded {Count} history entries", _history.Count);
        }
        catch (Exception ex)
        {
            // Corrupt or unreadable. Back up so the user's data isn't
            // silently overwritten on the next SaveAsync.
            var backup = $"{_filePath}.bak.{DateTime.Now:yyyyMMddHHmmss}";
            try { File.Copy(_filePath, backup, overwrite: true); } catch { /* best effort */ }

            _logger.LogError(ex,
                "History file unreadable at {Path}; backed up to {Backup}. SaveAsync will be blocked until the next successful load to avoid data loss.",
                _filePath, backup);
            _history.Clear();
            // Leave _loadCompleted == false → SaveAsync will refuse.
        }
    }

    public async Task SaveAsync()
    {
        if (!_loadCompleted)
        {
            _logger.LogWarning("History save skipped: load did not complete (refusing to overwrite a possibly-corrupt file)");
            return;
        }

        try
        {
            var dir = Path.GetDirectoryName(_filePath)!;
            Directory.CreateDirectory(dir);

            // Atomic write: serialize to a sibling .tmp, then move into place.
            // If the process is killed mid-write, the original file is untouched.
            var tmpPath = _filePath + ".tmp";
            var json = JsonSerializer.Serialize(_history, JsonOptions);
            await File.WriteAllTextAsync(tmpPath, json);
            File.Move(tmpPath, _filePath, overwrite: true);
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Failed to save history");
        }
    }

    /// <summary>
    /// Drops entries older than <see cref="RetentionWindow"/> and caps the
    /// total at <see cref="MaxEntries"/>. Caller is responsible for awaiting
    /// SaveAsync if they want the change persisted.
    /// </summary>
    private void Prune()
    {
        var cutoff = DateTime.UtcNow - RetentionWindow;

        var removed = _history.RemoveAll(e =>
            ToUtc(e.Timestamp) < cutoff);

        if (_history.Count > MaxEntries)
        {
            var overflow = _history.Count - MaxEntries;
            _history.RemoveRange(MaxEntries, overflow);
            removed += overflow;
        }

        if (removed > 0)
            _logger.LogDebug("History pruned: dropped {Count} entries (>{Days}d or >{Max} cap)",
                removed, (int)RetentionWindow.TotalDays, MaxEntries);
    }

    private static DateTime ToUtc(DateTime dt) => dt.Kind switch
    {
        DateTimeKind.Utc         => dt,
        DateTimeKind.Local       => dt.ToUniversalTime(),
        DateTimeKind.Unspecified => DateTime.SpecifyKind(dt, DateTimeKind.Utc),
        _                        => dt,
    };
}
