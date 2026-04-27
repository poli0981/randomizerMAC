using Microsoft.UI;
using Microsoft.UI.Composition.SystemBackdrops;
using Microsoft.UI.Windowing;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Media;
using RandomMac.App.Services;
using RandomMac.App.ViewModels;
using Serilog;
using Windows.Graphics;

namespace RandomMac.App;

/// <summary>
/// Application root window. Fixed 880x600, non-resizable, no maximize.
/// Custom titlebar dragable region driven by <c>SetTitleBar(AppTitleBar)</c>.
/// Mica backdrop on Win11; falls back to acrylic/solid on Win10.
/// </summary>
public sealed partial class MainWindow : Window
{
    private const int FixedWidth = 880;
    private const int FixedHeight = 600;

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
}

internal static class PInvoke
{
    internal static class User32
    {
        [System.Runtime.InteropServices.DllImport("user32.dll")]
        public static extern uint GetDpiForWindow(IntPtr hwnd);
    }
}
