using Microsoft.UI;
using Microsoft.UI.Xaml.Data;
using Microsoft.UI.Xaml.Media;
using Windows.UI;

namespace RandomMac.App.Converters;

/// <summary>
/// Converts a boolean to a color brush. True = green (connected), False = red (disconnected).
/// </summary>
public sealed class BoolToColorConverter : IValueConverter
{
    public static readonly BoolToColorConverter Instance = new();

    private static readonly SolidColorBrush GreenBrush = new(Color.FromArgb(0xFF, 0xA6, 0xE3, 0xA1));
    private static readonly SolidColorBrush RedBrush = new(Color.FromArgb(0xFF, 0xF3, 0x8B, 0xA8));

    public object Convert(object value, Type targetType, object parameter, string language)
        => value is true ? GreenBrush : RedBrush;

    public object ConvertBack(object value, Type targetType, object parameter, string language)
        => throw new NotSupportedException();
}

/// <summary>
/// Converts a boolean to a status string. True = "OK", False = "FAIL".
/// </summary>
public sealed class BoolToStatusConverter : IValueConverter
{
    public static readonly BoolToStatusConverter Instance = new();

    public object Convert(object value, Type targetType, object parameter, string language)
        => value is true ? "OK" : "FAIL";

    public object ConvertBack(object value, Type targetType, object parameter, string language)
        => throw new NotSupportedException();
}
