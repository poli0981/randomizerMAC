namespace RandomMac.Core.Models;

/// <summary>
/// Application settings, serialized to JSON.
/// </summary>
public sealed class AppSettings
{
    public string Language { get; set; } = "en";
    public bool RunAtStartup { get; set; }
    public bool MinimizeToTray { get; set; } = true;
    public bool StartMinimized { get; set; }
    public bool ShowNotifications { get; set; } = true;
    public string ThemeMode { get; set; } = "Dark";
    public string AccentColor { get; set; } = "Blue";

    // Auto-change MAC on OS startup
    public bool AutoChangeOnStartup { get; set; }
    public List<string> AutoChangeAdapterIds { get; set; } = [];
}
