using Avalonia.Data.Converters;
using Avalonia.Media;
using System.Globalization;

namespace RandomMac.App.Converters;

/// <summary>
/// Converts a boolean to a color brush. True = green (connected), False = red (disconnected).
/// </summary>
public sealed class BoolToColorConverter : IValueConverter
{
    public static readonly BoolToColorConverter Instance = new();

    private static readonly SolidColorBrush GreenBrush = new(Color.Parse("#A6E3A1"));
    private static readonly SolidColorBrush RedBrush = new(Color.Parse("#F38BA8"));

    public object Convert(object? value, Type targetType, object? parameter, CultureInfo culture)
        => value is true ? GreenBrush : RedBrush;

    public object ConvertBack(object? value, Type targetType, object? parameter, CultureInfo culture)
        => throw new NotSupportedException();
}

/// <summary>
/// Converts a boolean to a status string. True = checkmark, False = X.
/// </summary>
public sealed class BoolToStatusConverter : IValueConverter
{
    public static readonly BoolToStatusConverter Instance = new();

    public object Convert(object? value, Type targetType, object? parameter, CultureInfo culture)
        => value is true ? "OK" : "FAIL";

    public object ConvertBack(object? value, Type targetType, object? parameter, CultureInfo culture)
        => throw new NotSupportedException();
}
