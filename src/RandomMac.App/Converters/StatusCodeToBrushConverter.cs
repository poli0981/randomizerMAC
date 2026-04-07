using Avalonia.Data.Converters;
using Avalonia.Media;
using RandomMac.Core.Models;
using System.Globalization;

namespace RandomMac.App.Converters;

/// <summary>
/// Maps UpdateStatusCode to a color brush for the status badge.
/// </summary>
public sealed class StatusCodeToBrushConverter : IValueConverter
{
    public static readonly StatusCodeToBrushConverter Instance = new();

    private static readonly SolidColorBrush Green = new(Color.Parse("#A6E3A1"));
    private static readonly SolidColorBrush Blue = new(Color.Parse("#89B4FA"));
    private static readonly SolidColorBrush Yellow = new(Color.Parse("#F9E2AF"));
    private static readonly SolidColorBrush Red = new(Color.Parse("#F38BA8"));
    private static readonly SolidColorBrush Gray = new(Color.Parse("#6C7086"));

    public object Convert(object? value, Type targetType, object? parameter, CultureInfo culture)
    {
        if (value is not UpdateStatusCode code) return Gray;

        return code switch
        {
            UpdateStatusCode.UpToDate or UpdateStatusCode.ReadyToInstall => Green,
            UpdateStatusCode.UpdateAvailable => Blue,
            UpdateStatusCode.Checking or UpdateStatusCode.Downloading or UpdateStatusCode.Idle => Yellow,
            _ => Red
        };
    }

    public object ConvertBack(object? value, Type targetType, object? parameter, CultureInfo culture)
        => throw new NotSupportedException();
}
