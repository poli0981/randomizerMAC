using Avalonia;
using Avalonia.Data.Converters;
using Avalonia.Media;
using RandomMac.App.Services;
using System.Globalization;

namespace RandomMac.App.Converters;

/// <summary>
/// Converts NotificationType to a theme-aware brush.
/// ConverterParameter: "Background" or "Stripe".
/// </summary>
public sealed class NotifTypeToBrushConverter : IValueConverter
{
    public static readonly NotifTypeToBrushConverter Instance = new();

    public object Convert(object? value, Type targetType, object? parameter, CultureInfo culture)
    {
        if (value is not NotificationType type) return Brushes.Transparent;

        var kind = parameter as string ?? "Background";
        var key = $"Notif{TypeToString(type)}{kind}Brush";

        if (Application.Current?.Resources.TryGetResource(key, Application.Current.ActualThemeVariant, out var resource) == true
            && resource is IBrush brush)
        {
            return brush;
        }

        return Brushes.Transparent;
    }

    private static string TypeToString(NotificationType type) => type switch
    {
        NotificationType.Success => "Success",
        NotificationType.Error => "Error",
        NotificationType.Warning => "Warning",
        _ => "Info"
    };

    public object ConvertBack(object? value, Type targetType, object? parameter, CultureInfo culture)
        => throw new NotSupportedException();
}
