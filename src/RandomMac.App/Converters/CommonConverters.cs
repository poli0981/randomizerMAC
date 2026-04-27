using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Data;
using RandomMac.Core.Models;

namespace RandomMac.App.Converters;

public sealed class BoolToVisibilityConverter : IValueConverter
{
    public object Convert(object value, Type targetType, object parameter, string language)
    {
        var visible = value is true;
        if (parameter is string p && p.Equals("invert", StringComparison.OrdinalIgnoreCase))
            visible = !visible;
        return visible ? Visibility.Visible : Visibility.Collapsed;
    }

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

/// <summary>
/// Formats a <see cref="DateTime"/> as a relative phrase ("just now",
/// "5 min ago", "3 h ago", "yesterday", "2026-04-25 14:32").
/// </summary>
public sealed class RelativeTimeConverter : IValueConverter
{
    public object Convert(object value, Type targetType, object parameter, string language)
    {
        if (value is not DateTime when) return string.Empty;

        var delta = DateTime.Now - when;
        if (delta.TotalSeconds < 0)            return when.ToString("HH:mm");
        if (delta.TotalSeconds < 60)           return "just now";
        if (delta.TotalMinutes < 60)           return $"{(int)delta.TotalMinutes} min ago";
        if (delta.TotalHours   < 24)           return $"{(int)delta.TotalHours} h ago";
        if (delta.TotalHours   < 48)           return "yesterday " + when.ToString("HH:mm");
        return when.ToString("yyyy-MM-dd HH:mm");
    }

    public object ConvertBack(object value, Type targetType, object parameter, string language)
        => throw new NotSupportedException();
}

/// <summary>
/// Maps <see cref="AdapterType"/> to a Segoe Fluent Icons glyph string.
/// </summary>
public sealed class AdapterTypeToGlyphConverter : IValueConverter
{
    public object Convert(object value, Type targetType, object parameter, string language)
        => value is AdapterType t ? t switch
        {
            AdapterType.WiFi     => "", // Network bars (wireless)
            AdapterType.Ethernet => "", // Ethernet
            _                    => "", // DeviceLaptopNoPic (generic)
        } : "";

    public object ConvertBack(object value, Type targetType, object parameter, string language)
        => throw new NotSupportedException();
}
