using System.ComponentModel;
using System.Globalization;
using System.Resources;

namespace RandomMac.App.Localization;

/// <summary>
/// Provides localized string access. Change culture via SetLanguage().
/// Stores the active culture internally to avoid thread-culture drift.
///
/// Bound from XAML as <c>{Binding [Key], Source={StaticResource Loc}}</c> —
/// the singleton is registered in <c>App.OnLaunched</c> as
/// <c>Resources["Loc"] = Loc.Instance</c>.
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

    public static string Get(string key) => Instance[key];

    public static string Get(string key, params object[] args)
        => string.Format(Instance[key], args);

    /// <summary>
    /// Change the UI language and notify all bindings.
    /// </summary>
    public static void SetLanguage(string cultureCode)
    {
        var culture = new CultureInfo(cultureCode);

        Instance._culture = culture;

        CultureInfo.CurrentUICulture = culture;
        CultureInfo.CurrentCulture = culture;
        Thread.CurrentThread.CurrentUICulture = culture;
        Thread.CurrentThread.CurrentCulture = culture;

        // "Item[]" is the WinUI/WPF indexer-change marker
        Instance.PropertyChanged?.Invoke(Instance,
            new PropertyChangedEventArgs("Item[]"));
    }

    public static string CurrentLanguage => Instance._culture.TwoLetterISOLanguageName;
}
