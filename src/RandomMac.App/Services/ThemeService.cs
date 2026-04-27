using Microsoft.Extensions.DependencyInjection;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Media;
using Windows.UI;

namespace RandomMac.App.Services;

/// <summary>
/// Manages runtime theme mode (Dark/Light). Accent color is fixed to the
/// brand blue — the v1.0.0 multi-color picker was removed in v1.1.x to
/// simplify the Settings UI.
/// </summary>
public sealed class ThemeService
{
    public static readonly string[] AvailableModes = ["Dark", "Light"];

    /// <summary>
    /// Hardcoded brand accent (#61AFEF) — same color as the v1.0.0 default.
    /// </summary>
    private static readonly Color BrandAccent = Color.FromArgb(0xFF, 0x61, 0xAF, 0xEF);

    public void Apply(string mode)
    {
        ApplyThemeMode(mode);
        ApplyBrandAccent();
    }

    public void ApplyThemeMode(string mode)
    {
        var theme = mode == "Light" ? ElementTheme.Light : ElementTheme.Dark;

        // WinUI 3: Application.Current.RequestedTheme can only be set before
        // the first window. After that, walk to the root FrameworkElement and
        // set RequestedTheme there — propagates to ThemeResource consumers.
        try
        {
            var window = App.Services?.GetService<MainWindow>();
            if (window?.Content is FrameworkElement root)
                root.RequestedTheme = theme;
        }
        catch
        {
            // Window not yet created — initial Apply() in OnLaunched runs
            // before the window exists; that's fine, ElementTheme defaults
            // to Default and inherits from Application later.
        }
    }

    private static void ApplyBrandAccent()
    {
        if (Application.Current is null) return;

        var color = BrandAccent;
        var brush = new SolidColorBrush(color);
        var hoverBrush = new SolidColorBrush(Color.FromArgb(0xCC, color.R, color.G, color.B));

        var res = Application.Current.Resources;
        res["SystemAccentColor"] = color;
        res["SystemAccentColorDark1"] = color;
        res["SystemAccentColorDark2"] = DarkenColor(color, 0.2);
        res["SystemAccentColorDark3"] = DarkenColor(color, 0.4);
        res["SystemAccentColorLight1"] = LightenColor(color, 0.2);
        res["SystemAccentColorLight2"] = LightenColor(color, 0.35);
        res["SystemAccentColorLight3"] = LightenColor(color, 0.5);
        res["AccentFillColorDefaultBrush"] = brush;
        res["AccentFillColorSecondaryBrush"] = hoverBrush;
    }

    private static Color DarkenColor(Color c, double amount)
    {
        var factor = 1.0 - amount;
        return Color.FromArgb(c.A, (byte)(c.R * factor), (byte)(c.G * factor), (byte)(c.B * factor));
    }

    private static Color LightenColor(Color c, double amount)
        => Color.FromArgb(c.A,
            (byte)(c.R + (255 - c.R) * amount),
            (byte)(c.G + (255 - c.G) * amount),
            (byte)(c.B + (255 - c.B) * amount));
}
