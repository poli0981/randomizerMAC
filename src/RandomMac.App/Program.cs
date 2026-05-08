using Microsoft.Extensions.DependencyInjection;
using Microsoft.UI.Dispatching;
using Microsoft.UI.Xaml;
using Microsoft.Windows.AppLifecycle;
using RandomMac.App.Services;
using Velopack;

namespace RandomMac.App;

internal static class Program
{
    [STAThread]
    private static int Main(string[] args)
    {
        // Velopack must be the first thing in Main (handles --veloapp-* CLI args
        // for install/update lifecycle before any UI is created). Velopack may
        // Environment.Exit before reaching the single-instance gate below.
        VelopackApp.Build().Run();

        WinRT.ComWrappersSupport.InitializeComWrappers();

        // Single-instance gate. If another RandomMac is already running (e.g.
        // resident in the tray after MinimizeToTray), redirect this activation
        // to it and exit so duplicate processes don't accumulate.
        var keyInstance = AppInstance.FindOrRegisterForKey("RandomMac.SingleInstance");
        if (!keyInstance.IsCurrent)
        {
            var activatedArgs = AppInstance.GetCurrent().GetActivatedEventArgs();
            keyInstance.RedirectActivationToAsync(activatedArgs)
                       .AsTask().GetAwaiter().GetResult();
            return 0;
        }

        // We are the primary. Subscribe BEFORE Application.Start so a fast
        // second-launch redirected during our own bootstrap is queued by the
        // AppLifecycle plumbing rather than dropped.
        AppInstance.GetCurrent().Activated += OnActivated;

        Application.Start(p =>
        {
            var context = new DispatcherQueueSynchronizationContext(
                DispatcherQueue.GetForCurrentThread());
            SynchronizationContext.SetSynchronizationContext(context);
            new App();
        });

        return 0;
    }

    private static void OnActivated(object? sender, AppActivationArguments e)
    {
        // Fires on the threadpool. App.MainDispatcher is null until OnLaunched
        // has run far enough to capture it from MainWindow — bail silently if
        // a redirect arrives during our own startup.
        var dispatcher = App.MainDispatcher;
        if (dispatcher is null) return;

        dispatcher.TryEnqueue(() =>
        {
            try
            {
                App.Services.GetRequiredService<TrayIconService>().ShowMainWindow();
            }
            catch
            {
                // Non-fatal: the redirect was consumed regardless.
            }
        });
    }
}
