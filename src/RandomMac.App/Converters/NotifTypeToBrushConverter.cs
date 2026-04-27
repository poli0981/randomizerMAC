using Microsoft.UI;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Data;
using Microsoft.UI.Xaml.Media;
using RandomMac.App.Services;
using Windows.UI;

namespace RandomMac.App.Converters;

/// <summary>
/// Converts NotificationType to a theme-aware brush.
/// ConverterParameter: "Background" or "Stripe".
/// </summary>
public sealed class NotifTypeToBrushConverter : IValueConverter
{
    public static readonly NotifTypeToBrushConverter Instance = new();

    private static readonly SolidColorBrush Transparent = new(Color.FromArgb(0, 0, 0, 0));

    public object Convert(object value, Type targetType, object parameter, string language)
    {
        if (value is not NotificationType type) return Transparent;

        var kind = parameter as string ?? "Background";
        var key = $"Notif{TypeToString(type)}{kind}Brush";

        if (Application.Current?.Resources is { } res
            && res.TryGetValue(key, out var resource)
            && resource is Brush brush)
        {
            return brush;
        }

        return Transparent;
    }

    private static string TypeToString(NotificationType type) => type switch
    {
        NotificationType.Success => "Success",
        NotificationType.Error => "Error",
        NotificationType.Warning => "Warning",
        _ => "Info"
    };

    public object ConvertBack(object value, Type targetType, object parameter, string language)
        => throw new NotSupportedException();
}
