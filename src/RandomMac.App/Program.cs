using Microsoft.UI.Dispatching;
using Microsoft.UI.Xaml;
using Velopack;

namespace RandomMac.App;

internal static class Program
{
    [STAThread]
    private static int Main(string[] args)
    {
        // Velopack must be the first thing in Main (handles --veloapp-* CLI args
        // for install/update lifecycle before any UI is created).
        VelopackApp.Build().Run();

        WinRT.ComWrappersSupport.InitializeComWrappers();

        Application.Start(p =>
        {
            var context = new DispatcherQueueSynchronizationContext(
                DispatcherQueue.GetForCurrentThread());
            SynchronizationContext.SetSynchronizationContext(context);
            new App();
        });

        return 0;
    }
}
