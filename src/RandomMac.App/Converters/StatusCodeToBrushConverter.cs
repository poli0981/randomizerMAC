using Microsoft.UI.Xaml.Data;
using Microsoft.UI.Xaml.Media;
using RandomMac.Core.Models;
using Windows.UI;

namespace RandomMac.App.Converters;

/// <summary>
/// Maps UpdateStatusCode to a color brush for the status badge.
/// </summary>
public sealed class StatusCodeToBrushConverter : IValueConverter
{
    public static readonly StatusCodeToBrushConverter Instance = new();

    private static readonly SolidColorBrush Green = new(Color.FromArgb(0xFF, 0xA6, 0xE3, 0xA1));
    private static readonly SolidColorBrush Blue = new(Color.FromArgb(0xFF, 0x89, 0xB4, 0xFA));
    private static readonly SolidColorBrush Yellow = new(Color.FromArgb(0xFF, 0xF9, 0xE2, 0xAF));
    private static readonly SolidColorBrush Red = new(Color.FromArgb(0xFF, 0xF3, 0x8B, 0xA8));
    private static readonly SolidColorBrush Gray = new(Color.FromArgb(0xFF, 0x6C, 0x70, 0x86));

    public object Convert(object value, Type targetType, object parameter, string language)
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

    public object ConvertBack(object value, Type targetType, object parameter, string language)
        => throw new NotSupportedException();
}
