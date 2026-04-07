using Avalonia.Data.Converters;
using RandomMac.App.Services;
using System.Globalization;

namespace RandomMac.App.Converters;

public sealed class NotifTypeToIconConverter : IValueConverter
{
    public static readonly NotifTypeToIconConverter Instance = new();

    public object Convert(object? value, Type targetType, object? parameter, CultureInfo culture)
    {
        return value is NotificationType type ? type switch
        {
            NotificationType.Success => "OK",
            NotificationType.Error => "ERR",
            NotificationType.Warning => "WARN",
            _ => "INFO"
        } : "INFO";
    }

    public object ConvertBack(object? value, Type targetType, object? parameter, CultureInfo culture)
        => throw new NotSupportedException();
}
