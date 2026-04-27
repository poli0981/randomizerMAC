using Microsoft.Extensions.Logging;
using RandomMac.Core.Models;
using RandomMac.Core.Services.Interfaces;
using System.Text.Encodings.Web;
using System.Text.Json;
using System.Text.Json.Serialization;

namespace RandomMac.Core.Services.Implementations;

public sealed class BlacklistService : IBlacklistService
{
    private readonly ILogger<BlacklistService> _logger;
    private readonly string _filePath;
    private readonly HashSet<MacAddress> _blacklist = [];
    private readonly Dictionary<string, HashSet<MacAddress>> _adapterBlacklists = [];

    private static readonly JsonSerializerOptions JsonOptions = new()
    {
        WriteIndented = true,
        DefaultIgnoreCondition = JsonIgnoreCondition.WhenWritingNull,
        Encoder = JavaScriptEncoder.UnsafeRelaxedJsonEscaping,
    };

    private static readonly MacAddress[] ReservedMacs =
    [
        MacAddress.Parse("00:00:00:00:00:00"),
        MacAddress.Parse("FF:FF:FF:FF:FF:FF"),
        MacAddress.Parse("AA:AA:AA:AA:AA:AA"),
    ];

    public BlacklistService(ILogger<BlacklistService> logger)
    {
        _logger = logger;
        _filePath = Path.Combine(
            Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData),
            "RandomMac", "blacklist.json");

        EnsureReserved();
    }

    // --- Global blacklist ---

    public IReadOnlySet<MacAddress> GetBlacklist() => _blacklist;

    public void Add(MacAddress mac)
    {
        if (_blacklist.Add(mac))
            _logger.LogInformation("Added {Mac} to global blacklist", mac);
    }

    public void Remove(MacAddress mac)
    {
        if (IsReserved(mac))
        {
            _logger.LogWarning("Cannot remove reserved MAC {Mac}", mac);
            return;
        }
        if (_blacklist.Remove(mac))
            _logger.LogInformation("Removed {Mac} from global blacklist", mac);
    }

    public void Clear()
    {
        _blacklist.Clear();
        EnsureReserved();
        _logger.LogInformation("Global blacklist cleared (reserved retained)");
    }

    // --- Per-adapter blacklist ---

    public IReadOnlySet<MacAddress> GetAdapterBlacklist(string pnpDeviceId)
    {
        if (_adapterBlacklists.TryGetValue(pnpDeviceId, out var set))
            return set;
        return new HashSet<MacAddress>();
    }

    public void AddToAdapter(string pnpDeviceId, MacAddress mac)
    {
        if (!_adapterBlacklists.TryGetValue(pnpDeviceId, out var set))
        {
            set = [];
            _adapterBlacklists[pnpDeviceId] = set;
        }
        if (set.Add(mac))
            _logger.LogInformation("Added {Mac} to adapter blacklist [{Adapter}]", mac, ShortenId(pnpDeviceId));
    }

    public void RemoveFromAdapter(string pnpDeviceId, MacAddress mac)
    {
        if (_adapterBlacklists.TryGetValue(pnpDeviceId, out var set) && set.Remove(mac))
            _logger.LogInformation("Removed {Mac} from adapter blacklist [{Adapter}]", mac, ShortenId(pnpDeviceId));
    }

    public void ClearAdapter(string pnpDeviceId)
    {
        if (_adapterBlacklists.Remove(pnpDeviceId))
            _logger.LogInformation("Cleared adapter blacklist [{Adapter}]", ShortenId(pnpDeviceId));
    }

    // --- Combined ---

    public IReadOnlySet<MacAddress> GetEffectiveBlacklist(string pnpDeviceId)
    {
        var combined = new HashSet<MacAddress>(_blacklist);
        if (_adapterBlacklists.TryGetValue(pnpDeviceId, out var adapterSet))
        {
            foreach (var mac in adapterSet)
                combined.Add(mac);
        }
        return combined;
    }

    // --- Persistence ---

    public async Task LoadAsync()
    {
        var needsSave = false;

        try
        {
            if (!File.Exists(_filePath))
            {
                _logger.LogInformation("Blacklist file not found, creating with defaults");
                needsSave = true;
            }
            else
            {
                var json = await File.ReadAllTextAsync(_filePath);

                // Try new nested format first
                if (TryLoadNestedFormat(json))
                {
                    _logger.LogDebug("Loaded blacklist in nested format");
                }
                // Fallback: old flat format (migration)
                else if (TryLoadFlatFormat(json))
                {
                    _logger.LogInformation("Migrated blacklist from flat to nested format");
                    needsSave = true;
                }
                else
                {
                    _logger.LogWarning("Failed to parse blacklist, resetting to defaults");
                    _blacklist.Clear();
                    needsSave = true;
                }

                // Ensure reserved MACs present
                var countBefore = _blacklist.Count;
                EnsureReserved();
                if (_blacklist.Count > countBefore)
                    needsSave = true;
            }
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Failed to load blacklist, recreating with defaults");
            _blacklist.Clear();
            _adapterBlacklists.Clear();
            needsSave = true;
        }

        EnsureReserved();

        if (needsSave)
        {
            await SaveAsync();
            _logger.LogInformation("Blacklist file saved with {Global} global + {Adapters} adapter entries",
                _blacklist.Count, _adapterBlacklists.Count);
        }
    }

    public async Task SaveAsync()
    {
        try
        {
            var dir = Path.GetDirectoryName(_filePath)!;
            Directory.CreateDirectory(dir);

            var data = new BlacklistData
            {
                Global = _blacklist.Select(m => m.ToDisplayString()).ToList(),
                Adapters = _adapterBlacklists.ToDictionary(
                    kvp => kvp.Key,
                    kvp => kvp.Value.Select(m => m.ToDisplayString()).ToList())
            };

            var json = JsonSerializer.Serialize(data, JsonOptions);
            await File.WriteAllTextAsync(_filePath, json);

            _logger.LogDebug("Saved blacklist: {Global} global, {Adapters} adapter categories",
                _blacklist.Count, _adapterBlacklists.Count);
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Failed to save blacklist to {Path}", _filePath);
        }
    }

    public string GetFilePath() => _filePath;

    // --- Private helpers ---

    private void EnsureReserved()
    {
        foreach (var mac in ReservedMacs)
            _blacklist.Add(mac);
    }

    private static bool IsReserved(MacAddress mac)
        => ReservedMacs.Any(r => r == mac);

    private bool TryLoadNestedFormat(string json)
    {
        try
        {
            var data = JsonSerializer.Deserialize<BlacklistData>(json);
            if (data?.Global is null) return false;

            _blacklist.Clear();
            foreach (var entry in data.Global)
            {
                if (MacAddress.TryParse(entry, out var mac))
                    _blacklist.Add(mac);
            }

            _adapterBlacklists.Clear();
            if (data.Adapters is not null)
            {
                foreach (var (adapterId, entries) in data.Adapters)
                {
                    var set = new HashSet<MacAddress>();
                    foreach (var entry in entries)
                    {
                        if (MacAddress.TryParse(entry, out var mac))
                            set.Add(mac);
                    }
                    if (set.Count > 0)
                        _adapterBlacklists[adapterId] = set;
                }
            }

            return true;
        }
        catch
        {
            return false;
        }
    }

    private bool TryLoadFlatFormat(string json)
    {
        try
        {
            var entries = JsonSerializer.Deserialize<List<string>>(json);
            if (entries is null) return false;

            _blacklist.Clear();
            foreach (var entry in entries)
            {
                if (MacAddress.TryParse(entry, out var mac))
                    _blacklist.Add(mac);
            }
            return true;
        }
        catch
        {
            return false;
        }
    }

    private static string ShortenId(string pnpDeviceId)
        => pnpDeviceId.Length > 20 ? pnpDeviceId[..20] + "..." : pnpDeviceId;

    private sealed class BlacklistData
    {
        [JsonPropertyName("global")]
        public List<string> Global { get; set; } = [];

        [JsonPropertyName("adapters")]
        public Dictionary<string, List<string>>? Adapters { get; set; }
    }
}
