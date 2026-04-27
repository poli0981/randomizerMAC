using Microsoft.UI;
using Microsoft.UI.Composition.SystemBackdrops;
using Microsoft.UI.Windowing;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Input;
using Microsoft.UI.Xaml.Media;
using RandomMac.App.Services;
using RandomMac.App.ViewModels;
using Serilog;
using Windows.Graphics;
using Windows.System;

namespace RandomMac.App;

/// <summary>
/// Application root window. Fixed 880x600, non-resizable, no maximize.
/// Custom titlebar dragable region driven by <c>SetTitleBar(AppTitleBar)</c>.
/// Mica backdrop on Win11; falls back to acrylic/solid on Win10.
/// </summary>
public sealed partial class MainWindow : Window
{
    private const int FixedWidth = 1024;
    private const int FixedHeight = 680;

    public MainWindow(MainWindowViewModel viewModel, NotificationService notificationService)
    {
        InitializeComponent();

        RootGrid.DataContext = viewModel;
        NotificationOverlay.DataContext = notificationService;

        ConfigureTitleBar();
        ConfigurePresenter();
        ConfigureBackdrop();
    }

    private void ConfigureTitleBar()
    {
        ExtendsContentIntoTitleBar = true;
        SetTitleBar(AppTitleBar);
    }

    private void ConfigurePresenter()
    {
        if (AppWindow is null) return;

        if (AppWindow.Presenter is OverlappedPresenter presenter)
        {
            presenter.IsResizable = false;
            presenter.IsMaximizable = false;
        }

        // Resize to 880x600 in physical pixels accounting for current DPI.
        // PerMonitorV2 in app.manifest ensures the OS gives us scaled device pixels.
        var dpi = GetDpiForWindow();
        var width = (int)(FixedWidth * dpi / 96.0);
        var height = (int)(FixedHeight * dpi / 96.0);

        var displayArea = DisplayArea.GetFromWindowId(AppWindow.Id, DisplayAreaFallback.Primary);
        var centerX = displayArea.WorkArea.X + (displayArea.WorkArea.Width - width) / 2;
        var centerY = displayArea.WorkArea.Y + (displayArea.WorkArea.Height - height) / 2;
        AppWindow.MoveAndResize(new RectInt32(centerX, centerY, width, height));
    }

    private void ConfigureBackdrop()
    {
        try
        {
            if (MicaController.IsSupported())
            {
                SystemBackdrop = new MicaBackdrop();
                Log.Debug("Window backdrop: Mica");
            }
            else if (DesktopAcrylicController.IsSupported())
            {
                SystemBackdrop = new DesktopAcrylicBackdrop();
                Log.Debug("Window backdrop: Acrylic (Win10 fallback)");
            }
            else
            {
                Log.Debug("Window backdrop: solid (no controller supported)");
            }
        }
        catch (Exception ex)
        {
            Log.Warning(ex, "Failed to apply system backdrop, falling back to solid");
        }
    }

    private uint GetDpiForWindow()
    {
        var hwnd = WinRT.Interop.WindowNative.GetWindowHandle(this);
        return PInvoke.User32.GetDpiForWindow(hwnd);
    }

    /// <summary>
    /// Ctrl+1..5 jumps to the corresponding nav item in
    /// <see cref="MainWindowViewModel.NavItems"/>.
    /// </summary>
    private void OnNavShortcut(KeyboardAccelerator sender, KeyboardAcceleratorInvokedEventArgs args)
    {
        var index = sender.Key switch
        {
            VirtualKey.Number1 => 0,
            VirtualKey.Number2 => 1,
            VirtualKey.Number3 => 2,
            VirtualKey.Number4 => 3,
            VirtualKey.Number5 => 4,
            _ => -1,
        };

        if (index >= 0
            && RootGrid.DataContext is MainWindowViewModel vm
            && index < vm.NavItems.Count)
        {
            vm.SelectedNav = vm.NavItems[index];
            args.Handled = true;
        }
    }
}

internal static class PInvoke
{
    internal static class User32
    {
        [System.Runtime.InteropServices.DllImport("user32.dll")]
        public static extern uint GetDpiForWindow(IntPtr hwnd);
    }
}
