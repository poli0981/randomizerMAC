using Avalonia;
using Avalonia.Controls;
using Avalonia.Controls.ApplicationLifetimes;
using RandomMac.Core.Services.Interfaces;

namespace RandomMac.App.Services;

/// <summary>
/// Manages the system tray icon and its context menu.
/// </summary>
public sealed class TrayIconService : IDisposable
{
    private TrayIcon? _trayIcon;
    private readonly ISettingsService _settingsService;
    private bool _isExiting;

    public TrayIconService(ISettingsService settingsService)
    {
        _settingsService = settingsService;
    }

    public void Initialize(Window mainWindow)
    {
        _trayIcon = new TrayIcon
        {
            ToolTipText = "RANDOM MAC",
            IsVisible = true,
            Menu = CreateMenu(mainWindow)
        };

        _trayIcon.Clicked += (_, _) => ShowWindow(mainWindow);

        // Handle minimize to tray (intercept close → hide, unless exiting)
        mainWindow.Closing += (_, e) =>
        {
            if (_isExiting) return; // Allow close when exiting

            if (_settingsService.Settings.MinimizeToTray)
            {
                e.Cancel = true;
                mainWindow.Hide();
            }
        };
    }

    private NativeMenu CreateMenu(Window mainWindow)
    {
        var menu = new NativeMenu();

        var showItem = new NativeMenuItem("Show");
        showItem.Click += (_, _) => ShowWindow(mainWindow);
        menu.Add(showItem);

        menu.Add(new NativeMenuItemSeparator());

        var exitItem = new NativeMenuItem("Exit");
        exitItem.Click += (_, _) => ExitApp(mainWindow);
        menu.Add(exitItem);

        return menu;
    }

    private void ExitApp(Window mainWindow)
    {
        _isExiting = true; // Bypass Closing handler
        Dispose();

        if (Application.Current?.ApplicationLifetime is IClassicDesktopStyleApplicationLifetime desktop)
        {
            desktop.Shutdown();
        }
    }

    private static void ShowWindow(Window window)
    {
        window.Show();
        window.WindowState = WindowState.Normal;
        window.Activate();
    }

    public void Dispose()
    {
        _trayIcon?.Dispose();
        _trayIcon = null;
    }
}
