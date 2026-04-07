using Avalonia;
using Avalonia.Media;
using Avalonia.Styling;

namespace RandomMac.App.Services;

/// <summary>
/// Manages runtime theme mode (Dark/Light) and accent color switching.
/// </summary>
public sealed class ThemeService
{
    public static readonly string[] AvailableModes = ["Dark", "Light"];
    public static readonly string[] AvailableAccentColors = ["Blue", "Red", "Green", "Purple", "Orange", "Teal"];

    private static readonly Dictionary<string, Color> AccentColors = new()
    {
        ["Blue"] = Color.Parse("#61AFEF"),
        ["Red"] = Color.Parse("#E06C75"),
        ["Green"] = Color.Parse("#98C379"),
        ["Purple"] = Color.Parse("#C678DD"),
        ["Orange"] = Color.Parse("#E5C07B"),
        ["Teal"] = Color.Parse("#56B6C2"),
    };

    public void Apply(string mode, string accentColor)
    {
        ApplyThemeMode(mode);
        ApplyAccentColor(accentColor);
    }

    public void ApplyThemeMode(string mode)
    {
        if (Application.Current is null) return;

        Application.Current.RequestedThemeVariant = mode switch
        {
            "Light" => ThemeVariant.Light,
            _ => ThemeVariant.Dark
        };
    }

    public void ApplyAccentColor(string colorName)
    {
        if (Application.Current is null) return;

        if (!AccentColors.TryGetValue(colorName, out var color))
            color = AccentColors["Blue"];

        var brush = new SolidColorBrush(color);
        var hoverBrush = new SolidColorBrush(Color.FromArgb(204, color.R, color.G, color.B));

        // Override Fluent accent resource keys
        Application.Current.Resources["SystemAccentColor"] = color;
        Application.Current.Resources["SystemAccentColorDark1"] = color;
        Application.Current.Resources["SystemAccentColorDark2"] = DarkenColor(color, 0.2);
        Application.Current.Resources["SystemAccentColorDark3"] = DarkenColor(color, 0.4);
        Application.Current.Resources["SystemAccentColorLight1"] = LightenColor(color, 0.2);
        Application.Current.Resources["SystemAccentColorLight2"] = LightenColor(color, 0.35);
        Application.Current.Resources["SystemAccentColorLight3"] = LightenColor(color, 0.5);
        Application.Current.Resources["AccentFillColorDefaultBrush"] = brush;
        Application.Current.Resources["AccentFillColorSecondaryBrush"] = hoverBrush;
    }

    private static Color DarkenColor(Color c, double amount)
    {
        var factor = 1.0 - amount;
        return Color.FromArgb(c.A, (byte)(c.R * factor), (byte)(c.G * factor), (byte)(c.B * factor));
    }

    private static Color LightenColor(Color c, double amount)
    {
        return Color.FromArgb(c.A,
            (byte)(c.R + (255 - c.R) * amount),
            (byte)(c.G + (255 - c.G) * amount),
            (byte)(c.B + (255 - c.B) * amount));
    }
}
