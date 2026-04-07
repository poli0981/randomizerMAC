using Avalonia.Data;
using Avalonia.Markup.Xaml;
using System.ComponentModel;
using System.Globalization;
using System.Resources;

namespace RandomMac.App.Localization;

/// <summary>
/// Provides localized string access. Change culture via SetLanguage().
/// Stores the active culture internally to avoid thread-culture drift.
/// </summary>
public sealed class Loc : INotifyPropertyChanged
{
    private static readonly Lazy<Loc> _instance = new(() => new Loc());
    public static Loc Instance => _instance.Value;

    private readonly ResourceManager _rm;
    private CultureInfo _culture = CultureInfo.CurrentUICulture;

    public event PropertyChangedEventHandler? PropertyChanged;

    private Loc()
    {
        _rm = new ResourceManager("RandomMac.App.Localization.Lang", typeof(Loc).Assembly);
    }

    /// <summary>
    /// Indexer for binding. Uses internally stored culture (not thread culture).
    /// </summary>
    public string this[string key] =>
        _rm.GetString(key, _culture) ?? $"[{key}]";

    /// <summary>
    /// Get a localized string by key.
    /// </summary>
    public static string Get(string key) => Instance[key];

    /// <summary>
    /// Get a localized string with format args.
    /// </summary>
    public static string Get(string key, params object[] args)
        => string.Format(Instance[key], args);

    /// <summary>
    /// Change the UI language and notify all bindings.
    /// </summary>
    public static void SetLanguage(string cultureCode)
    {
        var culture = new CultureInfo(cultureCode);

        // Store internally (thread-safe, no drift)
        Instance._culture = culture;

        // Also set thread/app culture for other .NET APIs
        CultureInfo.CurrentUICulture = culture;
        CultureInfo.CurrentCulture = culture;
        Thread.CurrentThread.CurrentUICulture = culture;
        Thread.CurrentThread.CurrentCulture = culture;

        // Notify all bindings to re-read from indexer
        Instance.PropertyChanged?.Invoke(Instance,
            new PropertyChangedEventArgs("Item"));
        Instance.PropertyChanged?.Invoke(Instance,
            new PropertyChangedEventArgs("Item[]"));
    }

    /// <summary>
    /// Current culture code (e.g. "en", "vi").
    /// </summary>
    public static string CurrentLanguage => Instance._culture.TwoLetterISOLanguageName;
}

/// <summary>
/// Markup extension for localized strings in XAML.
/// Usage: Text="{loc:L Nav_Dashboard}"
/// </summary>
public class L : MarkupExtension
{
    public string Key { get; set; } = "";

    public L() { }
    public L(string key) => Key = key;

    public override object ProvideValue(IServiceProvider serviceProvider)
    {
        return new Binding
        {
            Source = Loc.Instance,
            Path = $"[{Key}]",
            Mode = BindingMode.OneWay
        };
    }
}
