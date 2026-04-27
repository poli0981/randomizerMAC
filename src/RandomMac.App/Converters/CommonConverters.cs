using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Data;

namespace RandomMac.App.Converters;

public sealed class BoolToVisibilityConverter : IValueConverter
{
    public object Convert(object value, Type targetType, object parameter, string language)
        => (value is true) ? Visibility.Visible : Visibility.Collapsed;

    public object ConvertBack(object value, Type targetType, object parameter, string language)
        => value is Visibility v && v == Visibility.Visible;
}

public sealed class InverseBoolConverter : IValueConverter
{
    public object Convert(object value, Type targetType, object parameter, string language)
        => value is not true;

    public object ConvertBack(object value, Type targetType, object parameter, string language)
        => value is not true;
}

public sealed class NullToBoolConverter : IValueConverter
{
    /// <summary>
    /// Returns true when value is non-null. Pass parameter "invert" to flip.
    /// </summary>
    public object Convert(object value, Type targetType, object parameter, string language)
    {
        var hasValue = value is not null;
        return parameter is string p && p.Equals("invert", StringComparison.OrdinalIgnoreCase)
            ? !hasValue
            : hasValue;
    }

    public object ConvertBack(object value, Type targetType, object parameter, string language)
        => throw new NotSupportedException();
}
