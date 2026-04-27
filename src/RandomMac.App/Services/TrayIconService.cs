using CommunityToolkit.Mvvm.Input;
using H.NotifyIcon;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using RandomMac.Core.Services.Interfaces;

namespace RandomMac.App.Services;

/// <summary>
/// Manages the system tray icon and its context menu. Backed by
/// <see cref="H.NotifyIcon.TaskbarIcon"/> (WinUI 3 port of WPF's NotifyIcon).
/// </summary>
public sealed class TrayIconService : IDisposable
{
    private TaskbarIcon? _trayIcon;
    private MainWindow? _window;
    private bool _isExiting;

    private readonly ISettingsService _settingsService;

    public TrayIconService(ISettingsService settingsService)
    {
        _settingsService = settingsService;
    }

    public void Initialize(MainWindow mainWindow)
    {
        _window = mainWindow;

        _trayIcon = new TaskbarIcon
        {
            ToolTipText = "RANDOM MAC",
            ContextFlyout = BuildMenu(),
            LeftClickCommand = new RelayCommand(ShowMainWindow),
        };
        _trayIcon.ForceCreate();

        // Intercept close → hide (unless exiting from tray menu)
        if (mainWindow.AppWindow is not null)
        {
            mainWindow.AppWindow.Closing += OnAppWindowClosing;
        }
    }

    private void OnAppWindowClosing(
        Microsoft.UI.Windowing.AppWindow sender,
        Microsoft.UI.Windowing.AppWindowClosingEventArgs args)
    {
        if (_isExiting) return;

        if (_settingsService.Settings.MinimizeToTray)
        {
            args.Cancel = true;
            _window?.AppWindow?.Hide();
        }
    }

    private MenuFlyout BuildMenu()
    {
        var menu = new MenuFlyout();

        var showItem = new MenuFlyoutItem { Text = "Show" };
        showItem.Click += (_, _) => ShowMainWindow();
        menu.Items.Add(showItem);

        menu.Items.Add(new MenuFlyoutSeparator());

        var exitItem = new MenuFlyoutItem { Text = "Exit" };
        exitItem.Click += (_, _) => ExitApp();
        menu.Items.Add(exitItem);

        return menu;
    }

    private void ShowMainWindow()
    {
        if (_window?.AppWindow is null) return;

        _window.AppWindow.Show();
        _window.Activate();
    }

    private void ExitApp()
    {
        _isExiting = true;
        Dispose();
        Application.Current?.Exit();
    }

    public void Dispose()
    {
        _trayIcon?.Dispose();
        _trayIcon = null;
    }
}
