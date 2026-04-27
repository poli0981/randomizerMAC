using CommunityToolkit.Mvvm.Input;
using H.NotifyIcon;
using Microsoft.Extensions.Logging;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using RandomMac.App.Localization;
using RandomMac.Core.Helpers;
using RandomMac.Core.Models;
using RandomMac.Core.Services.Interfaces;
using System.Runtime.InteropServices;

namespace RandomMac.App.Services;

/// <summary>
/// Manages the system tray icon and its context menu. Backed by
/// <see cref="H.NotifyIcon.TaskbarIcon"/> (WinUI 3 port of WPF's NotifyIcon).
/// </summary>
public sealed class TrayIconService : IDisposable
{
    private TaskbarIcon? _trayIcon;
    private MenuFlyout? _menuFlyout;
    private MainWindow? _window;
    private bool _isExiting;

    private readonly ISettingsService _settingsService;
    private readonly IAdapterCacheService _adapterCache;
    private readonly IMacAddressService _macService;
    private readonly IBlacklistService _blacklistService;
    private readonly IHistoryService _historyService;
    private readonly NotificationService _notificationService;
    private readonly ILogger<TrayIconService> _logger;

    public TrayIconService(
        ISettingsService settingsService,
        IAdapterCacheService adapterCache,
        IMacAddressService macService,
        IBlacklistService blacklistService,
        IHistoryService historyService,
        NotificationService notificationService,
        ILogger<TrayIconService> logger)
    {
        _settingsService = settingsService;
        _adapterCache = adapterCache;
        _macService = macService;
        _blacklistService = blacklistService;
        _historyService = historyService;
        _notificationService = notificationService;
        _logger = logger;
    }

    public void Initialize(MainWindow mainWindow)
    {
        _window = mainWindow;
        _menuFlyout = BuildMenu();

        var showCmd = new RelayCommand(ShowMainWindow);

        _trayIcon = new TaskbarIcon
        {
            ToolTipText = "RANDOM MAC",
            ContextFlyout = _menuFlyout,
            LeftClickCommand = showCmd,
            DoubleClickCommand = showCmd,
            NoLeftClickDelay = true,
        };
        _trayIcon.ForceCreate();

        // ContextFlyout needs a XamlRoot to display its popup. The window's
        // XamlRoot becomes available once the window has activated at least
        // once — wire that up lazily so even --minimized startups eventually
        // attach a root when the window is first shown.
        mainWindow.Activated += OnWindowActivatedFirstTime;

        if (mainWindow.AppWindow is not null)
            mainWindow.AppWindow.Closing += OnAppWindowClosing;
    }

    private void OnWindowActivatedFirstTime(object sender, WindowActivatedEventArgs args)
    {
        if (_window is null) return;

        if (_window.Content?.XamlRoot is { } root && _menuFlyout is not null)
            _menuFlyout.XamlRoot = root;

        _window.Activated -= OnWindowActivatedFirstTime;
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

        var showItem = new MenuFlyoutItem
        {
            Text = "Show",
            Icon = new FontIcon { FontFamily = SegoeFluent, Glyph = "" }, // Window
        };
        showItem.Click += (_, _) => ShowMainWindow();
        menu.Items.Add(showItem);

        var randomizeItem = new MenuFlyoutItem
        {
            Text = "Randomize Active Adapter",
            Icon = new FontIcon { FontFamily = SegoeFluent, Glyph = "" }, // Shuffle
        };
        randomizeItem.Click += (_, _) => _ = RandomizeActiveAdapterAsync();
        menu.Items.Add(randomizeItem);

        menu.Items.Add(new MenuFlyoutSeparator());

        var exitItem = new MenuFlyoutItem
        {
            Text = "Exit",
            Icon = new FontIcon { FontFamily = SegoeFluent, Glyph = "" }, // ChromeClose
        };
        exitItem.Click += (_, _) => ExitApp();
        menu.Items.Add(exitItem);

        return menu;
    }

    private static readonly Microsoft.UI.Xaml.Media.FontFamily SegoeFluent =
        new("Segoe Fluent Icons");

    /// <summary>
    /// One-click MAC randomization on the first connected physical adapter.
    /// No UI required — for users who keep the window minimized and just
    /// want a fresh MAC.
    /// </summary>
    private async Task RandomizeActiveAdapterAsync()
    {
        var active = _adapterCache.Adapters.FirstOrDefault(a => a.IsConnected)
                  ?? _adapterCache.Adapters.FirstOrDefault(a => a.IsEnabled);

        if (active is null)
        {
            _notificationService.Warning("No active adapter found.");
            _logger.LogWarning("Tray Randomize: no active adapter in cache");
            return;
        }

        if (string.IsNullOrEmpty(active.RegistrySubKey))
        {
            _notificationService.Error($"{active.Name}: no registry subkey.");
            return;
        }

        var blacklist = _blacklistService.GetEffectiveBlacklist(active.PnpDeviceId);
        var newMac = MacAddressGenerator.Generate(blacklist);
        if (newMac is null)
        {
            _notificationService.Error("Could not generate a non-blacklisted MAC.");
            return;
        }

        try
        {
            var result = await _macService.ChangeMacAsync(active, newMac.Value);

            _historyService.Add(new MacHistoryEntry
            {
                AdapterName = active.Name,
                AdapterDeviceId = active.DeviceId,
                PreviousMac = result.PreviousMac.ToDisplayString(),
                NewMac = result.NewMac.ToDisplayString(),
                Success = result.Success,
                ErrorMessage = result.ErrorMessage,
                Timestamp = result.Timestamp,
            });
            await _historyService.SaveAsync();

            if (result.Success)
            {
                _notificationService.Success(Loc.Get("Notif_MacChanged", result.VerifiedMac));
                _logger.LogInformation("Tray Randomize: {Adapter} -> {Mac}", active.Name, result.VerifiedMac);
            }
            else
            {
                _blacklistService.AddToAdapter(active.PnpDeviceId, newMac.Value);
                await _blacklistService.SaveAsync();
                _notificationService.Error(Loc.Get("Notif_MacFailed", result.ErrorMessage ?? ""));
            }
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Tray Randomize failed");
            _notificationService.Error(ex.Message);
        }
    }

    private void ShowMainWindow()
    {
        if (_window?.AppWindow is null) return;

        _window.AppWindow.Show();

        // AppWindow.Show + WinUI Activate together don't reliably bring the
        // window to the foreground if it was hidden via AppWindow.Hide.
        // Use Win32 ShowWindow(SW_RESTORE) + SetForegroundWindow to force it.
        try
        {
            var hwnd = WinRT.Interop.WindowNative.GetWindowHandle(_window);
            ShowWindow(hwnd, SW_RESTORE);
            SetForegroundWindow(hwnd);
        }
        catch (Exception ex)
        {
            _logger.LogDebug(ex, "Win32 SetForegroundWindow failed (non-fatal)");
        }

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

    [DllImport("user32.dll")]
    private static extern bool SetForegroundWindow(IntPtr hWnd);

    [DllImport("user32.dll")]
    private static extern bool ShowWindow(IntPtr hWnd, int nCmdShow);

    private const int SW_RESTORE = 9;
}
