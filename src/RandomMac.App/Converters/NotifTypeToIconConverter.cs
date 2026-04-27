using Microsoft.UI.Xaml.Data;
using RandomMac.App.Services;

namespace RandomMac.App.Converters;

public sealed class NotifTypeToIconConverter : IValueConverter
{
    public static readonly NotifTypeToIconConverter Instance = new();

    public object Convert(object value, Type targetType, object parameter, string language)
    {
        return value is NotificationType type ? type switch
        {
            NotificationType.Success => "OK",
            NotificationType.Error => "ERR",
            NotificationType.Warning => "WARN",
            _ => "INFO"
        } : "INFO";
    }

    public object ConvertBack(object value, Type targetType, object parameter, string language)
        => throw new NotSupportedException();
}
