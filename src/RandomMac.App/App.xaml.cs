using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Logging;
using Microsoft.UI.Dispatching;
using Microsoft.UI.Xaml;
using RandomMac.App.Services;
using RandomMac.App.ViewModels;
using RandomMac.Core.Helpers;
using RandomMac.Core.Models;
using RandomMac.Core.Services.Implementations;
using RandomMac.Core.Services.Interfaces;
using Serilog;

namespace RandomMac.App;

public partial class App : Application
{
    private ServiceProvider? _serviceProvider;
    private MainWindow? _mainWindow;

    public static IServiceProvider Services { get; private set; } = null!;

    /// <summary>
    /// Captured from <see cref="MainWindow"/> after activation. Used by
    /// background-thread services (e.g. <see cref="NotificationService"/>)
    /// to marshal back to the UI thread.
    /// </summary>
    public static DispatcherQueue MainDispatcher { get; private set; } = null!;

    public App()
    {
        InitializeComponent();
        ConfigureServices();
    }

    protected override async void OnLaunched(LaunchActivatedEventArgs args)
    {
        // Register the localization singleton as an Application resource so
        // XAML can bind via {Binding [Key], Source={StaticResource Loc}}.
        // Must happen here, NOT in the constructor — accessing Application.Resources
        // before WinUI 3 has finished wiring the COM proxy throws E_UNEXPECTED
        // (0x8000FFFF). OnLaunched is the first callback where the runtime
        // guarantees the resource dictionary is reachable.
        Resources["Loc"] = Localization.Loc.Instance;

        if (!AdminHelper.IsRunningAsAdmin())
            Log.Warning("Application is not running as Administrator. Some features may not work.");

        // Load persisted data
        await Services.GetRequiredService<ISettingsService>().LoadAsync();
        await Services.GetRequiredService<IBlacklistService>().LoadAsync();
        await Services.GetRequiredService<IHistoryService>().LoadAsync();

        // Warm up the adapter cache once. ViewModels resolved via MainWindow
        // (Dashboard + Settings) read this cache synchronously in their
        // constructors, so it MUST be loaded before MainWindow is resolved —
        // otherwise both VMs would also race to GetPhysicalAdaptersAsync()
        // and emit duplicate "Detected adapter" log blocks.
        await Services.GetRequiredService<IAdapterCacheService>().EnsureLoadedAsync();

        var settings = Services.GetRequiredService<ISettingsService>().Settings;
        Services.GetRequiredService<ThemeService>().Apply(settings.ThemeMode, settings.AccentColor);
        Localization.Loc.SetLanguage(settings.Language);

        _mainWindow = Services.GetRequiredService<MainWindow>();
        MainDispatcher = _mainWindow.DispatcherQueue;

        // Tray icon must be initialized before --minimized handling
        Services.GetRequiredService<TrayIconService>().Initialize(_mainWindow);

        // Handle --minimized (only set by Task Scheduler at OS startup)
        var cmdArgs = Environment.GetCommandLineArgs();
        var isOsStartup = cmdArgs.Contains("--minimized");
        Log.Information("Startup args: [{Args}], isOsStartup={IsOsStartup}",
            string.Join(", ", cmdArgs), isOsStartup);

        if (isOsStartup)
        {
            Log.Information("Starting minimized to system tray (window not activated)");
            // Don't Activate() — tray icon is the visible affordance.
        }
        else
        {
            Log.Information("Starting with visible window");
            _mainWindow.Activate();
        }

        // Auto-change MAC on startup (regardless of minimized state)
        if (settings.AutoChangeOnStartup && settings.AutoChangeAdapterIds.Count > 0)
        {
            _ = PerformAutoChangeAsync(settings.AutoChangeAdapterIds);
        }

        // Auto-check for updates (throttled by LastUpdateCheckedAt). Fire-and-
        // forget so it doesn't block startup; UpdateService surfaces results
        // via NotificationService and updates settings.LastUpdateCheckedAt.
        _ = AutoCheckForUpdateAsync();
    }

    private const int UpdateCheckCooldownHours = 24;

    /// <summary>
    /// Runs an update check at startup if the last check is older than
    /// <see cref="UpdateCheckCooldownHours"/> hours. Silent on
    /// up-to-date / failure (logs only); shows an info toast when a new
    /// version is available so users notice without opening the Update tab.
    /// </summary>
    private static async Task AutoCheckForUpdateAsync()
    {
        var logger = Services.GetRequiredService<ILogger<App>>();
        var settingsService = Services.GetRequiredService<ISettingsService>();
        var updateService = Services.GetRequiredService<IUpdateService>();
        var notificationService = Services.GetRequiredService<NotificationService>();

        var settings = settingsService.Settings;
        var last = settings.LastUpdateCheckedAt;
        if (last.HasValue && (DateTime.Now - last.Value).TotalHours < UpdateCheckCooldownHours)
        {
            logger.LogDebug("Auto update check: skipped (last={Last}, cooldown={Hours}h)",
                last, UpdateCheckCooldownHours);
            return;
        }

        // Tiny delay so the window finishes activating first.
        await Task.Delay(TimeSpan.FromSeconds(5));

        try
        {
            logger.LogInformation("Auto update check: running");
            var result = await updateService.CheckForUpdateAsync();

            settings.LastUpdateCheckedAt = DateTime.Now;
            await settingsService.SaveAsync();

            logger.LogInformation("Auto update check: status={Status}", result.Status);

            if (result.Status == Core.Models.UpdateStatusCode.UpdateAvailable
                && !string.IsNullOrEmpty(result.LatestVersion))
            {
                notificationService.Info(
                    Localization.Loc.Get("Notif_UpdateAvailable", result.LatestVersion));
            }
        }
        catch (Exception ex)
        {
            logger.LogWarning(ex, "Auto update check failed (non-fatal)");
        }
    }

    private static async Task PerformAutoChangeAsync(List<string> adapterPnpIds)
    {
        var logger = Services.GetRequiredService<ILogger<App>>();
        var cache = Services.GetRequiredService<IAdapterCacheService>();
        var macService = Services.GetRequiredService<IMacAddressService>();
        var blacklistService = Services.GetRequiredService<IBlacklistService>();
        var historyService = Services.GetRequiredService<IHistoryService>();
        var notificationService = Services.GetRequiredService<NotificationService>();

        logger.LogInformation("Auto-change MAC: starting for {Count} adapter(s)", adapterPnpIds.Count);

        try
        {
            // Cache is already warmed up by EnsureLoadedAsync in OnLaunched.
            var targets = cache.Adapters.Where(a => adapterPnpIds.Contains(a.PnpDeviceId)).ToList();

            var successCount = 0;
            var failCount = 0;

            foreach (var adapter in targets)
            {
                if (string.IsNullOrEmpty(adapter.RegistrySubKey))
                {
                    logger.LogWarning("Auto-change: skipping {Name} (no registry subkey)", adapter.Name);
                    failCount++;
                    continue;
                }

                var blacklist = blacklistService.GetEffectiveBlacklist(adapter.PnpDeviceId);
                var newMac = MacAddressGenerator.Generate(blacklist);
                if (newMac is null)
                {
                    logger.LogWarning("Auto-change: could not generate MAC for {Name}", adapter.Name);
                    failCount++;
                    continue;
                }

                var result = await macService.ChangeMacAsync(adapter, newMac.Value);

                historyService.Add(new MacHistoryEntry
                {
                    AdapterName = adapter.Name,
                    AdapterDeviceId = adapter.DeviceId,
                    PreviousMac = result.PreviousMac.ToDisplayString(),
                    NewMac = result.NewMac.ToDisplayString(),
                    Success = result.Success,
                    ErrorMessage = result.ErrorMessage,
                    Timestamp = result.Timestamp
                });

                if (result.Success)
                {
                    successCount++;
                    logger.LogInformation("Auto-change: {Name} -> {Mac}", adapter.Name, result.VerifiedMac);
                }
                else
                {
                    failCount++;
                    blacklistService.AddToAdapter(adapter.PnpDeviceId, newMac.Value);
                    logger.LogWarning("Auto-change failed for {Name}: {Error}", adapter.Name, result.ErrorMessage);
                }
            }

            await historyService.SaveAsync();
            await blacklistService.SaveAsync();

            if (successCount > 0)
                notificationService.Success(Localization.Loc.Get("Notif_AutoChangeOk", successCount));
            if (failCount > 0)
                notificationService.Warning(Localization.Loc.Get("Notif_AutoChangeFail", failCount));

            logger.LogInformation("Auto-change completed: {Success} success, {Fail} failed", successCount, failCount);
        }
        catch (Exception ex)
        {
            logger.LogError(ex, "Auto-change MAC failed");
            notificationService.Error(Localization.Loc.Get("Notif_AutoChangeError", ex.Message));
        }
    }

    private void ConfigureServices()
    {
        var services = new ServiceCollection();

        // Logging
        Log.Logger = new LoggerConfiguration()
            .MinimumLevel.Debug()
            .WriteTo.Sink(LogEntrySink.Instance)
            .WriteTo.File(
                path: Path.Combine(
                    Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData),
                    "RandomMac", "logs", "log-.txt"),
                rollingInterval: RollingInterval.Day,
                retainedFileCountLimit: 7,
                outputTemplate: "{Timestamp:yyyy-MM-dd HH:mm:ss.fff} [{Level:u3}] {Message:lj}{NewLine}{Exception}")
            .CreateLogger();

        services.AddLogging(builder => builder.AddSerilog(Log.Logger, dispose: true));

        // Core services
        services.AddSingleton<INetworkAdapterService, NetworkAdapterService>();
        services.AddSingleton<IAdapterCacheService, AdapterCacheService>();
        services.AddSingleton<IMacAddressService, MacAddressService>();
        services.AddSingleton<IBlacklistService, BlacklistService>();
        services.AddSingleton<IHistoryService, HistoryService>();
        services.AddSingleton<ISettingsService, SettingsService>();
        services.AddSingleton<IUpdateService, UpdateService>();

        // ViewModels
        services.AddSingleton<MainWindowViewModel>();
        services.AddTransient<DashboardViewModel>();
        services.AddTransient<SettingsViewModel>();
        services.AddTransient<AboutViewModel>();
        services.AddTransient<UpdateViewModel>();
        services.AddTransient<LogViewModel>();

        // App services
        services.AddSingleton<ThemeService>();
        services.AddSingleton<TrayIconService>();
        services.AddSingleton<NotificationService>();

        // Views
        services.AddSingleton<MainWindow>();

        _serviceProvider = services.BuildServiceProvider();
        Services = _serviceProvider;
    }
}
