using Avalonia.Controls;
using Avalonia.Data.Converters;
using Avalonia.Interactivity;
using Avalonia.Media;
using Microsoft.Extensions.DependencyInjection;
using RandomMac.App.Services;
using RandomMac.App.ViewModels;
using System.Globalization;

namespace RandomMac.App.Views;

public partial class MainWindow : Window
{
    public static readonly IValueConverter IconConverter = new NavIconConverter();

    public MainWindow()
    {
        InitializeComponent();
        DataContext = App.Services.GetRequiredService<MainWindowViewModel>();

        // Wire notification overlay
        var notifService = App.Services.GetRequiredService<NotificationService>();
        NotificationOverlay.DataContext = notifService;
    }

    private void OnMinimizeClick(object? sender, RoutedEventArgs e)
    {
        WindowState = WindowState.Minimized;
    }

    private void OnCloseClick(object? sender, RoutedEventArgs e)
    {
        Close();
    }

    private sealed class NavIconConverter : IValueConverter
    {
        private static readonly Dictionary<string, string> IconPaths = new()
        {
            ["Home"] = "M12 3L4 9V21H9V14H15V21H20V9L12 3Z",
            ["Settings"] = "M12 15.5A3.5 3.5 0 1 0 12 8.5A3.5 3.5 0 0 0 12 15.5M19.43 12.97C19.47 12.65 19.5 12.33 19.5 12C19.5 11.67 19.47 11.34 19.43 11L21.54 9.37C21.73 9.22 21.78 8.95 21.66 8.73L19.66 5.27C19.54 5.05 19.27 4.96 19.05 5.05L16.56 6.05C16.04 5.66 15.5 5.32 14.87 5.07L14.5 2.42C14.46 2.18 14.25 2 14 2H10C9.75 2 9.54 2.18 9.5 2.42L9.13 5.07C8.5 5.32 7.96 5.66 7.44 6.05L4.95 5.05C4.73 4.96 4.46 5.05 4.34 5.27L2.34 8.73C2.21 8.95 2.27 9.22 2.46 9.37L4.57 11C4.53 11.34 4.5 11.67 4.5 12C4.5 12.33 4.53 12.65 4.57 12.97L2.46 14.63C2.27 14.78 2.21 15.05 2.34 15.27L4.34 18.73C4.46 18.95 4.73 19.04 4.95 18.95L7.44 17.94C7.96 18.34 8.5 18.68 9.13 18.93L9.5 21.58C9.54 21.82 9.75 22 10 22H14C14.25 22 14.46 21.82 14.5 21.58L14.87 18.93C15.5 18.67 16.04 18.34 16.56 17.94L19.05 18.95C19.27 19.04 19.54 18.95 19.66 18.73L21.66 15.27C21.78 15.05 21.73 14.78 21.54 14.63L19.43 12.97Z",
            ["Document"] = "M6 2C4.89 2 4 2.89 4 4V20C4 21.11 4.89 22 6 22H18C19.11 22 20 21.11 20 20V8L14 2H6M13 3.5L18.5 9H13V3.5M8 11H16V13H8V11M8 15H14V17H8V15Z",
            ["ArrowSync"] = "M12 4V1L8 5L12 9V6C15.31 6 18 8.69 18 12C18 13.01 17.75 13.97 17.3 14.8L18.76 16.26C19.54 15.03 20 13.57 20 12C20 7.58 16.42 4 12 4M12 18C8.69 18 6 15.31 6 12C6 10.99 6.25 10.03 6.7 9.2L5.24 7.74C4.46 8.97 4 10.43 4 12C4 16.42 7.58 20 12 20V23L16 19L12 15V18Z",
            ["Info"] = "M12 2C6.48 2 2 6.48 2 12S6.48 22 12 22S22 17.52 22 12S17.52 2 12 2M13 17H11V11H13V17M13 9H11V7H13V9Z",
        };

        public object? Convert(object? value, Type targetType, object? parameter, CultureInfo culture)
        {
            if (value is string iconName && IconPaths.TryGetValue(iconName, out var path))
            {
                return Geometry.Parse(path);
            }
            return null;
        }

        public object? ConvertBack(object? value, Type targetType, object? parameter, CultureInfo culture)
            => throw new NotSupportedException();
    }
}
