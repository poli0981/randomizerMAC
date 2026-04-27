using CommunityToolkit.Mvvm.Input;
using RandomMac.Core.Services.Interfaces;
using System.Collections.ObjectModel;
using System.Diagnostics;

namespace RandomMac.App.ViewModels;

public partial class AboutViewModel : ViewModelBase
{
    public override string Title => "About";
    public override string IconKey => "Info";

    public string AppName => "RANDOM MAC";
    public string Version { get; }
    public string Description => "A lightweight Windows utility for randomizing network adapter MAC addresses.";
    public string Developer => "poli0981";
    public string DeveloperUrl => "https://github.com/poli0981";
    public string RepositoryUrl => "https://github.com/poli0981/randomizerMAC";
    public string License => "MIT License";

    public ObservableCollection<ThirdPartyInfo> ThirdParties { get; } =
    [
        new("Microsoft.WindowsAppSDK",                     "1.8.x",   "MIT",        "WinUI 3 + Fluent Design platform",   "https://github.com/microsoft/WindowsAppSDK"),
        new("CommunityToolkit.WinUI.UI.Controls.DataGrid", "7.1.2",   "MIT",        "DataGrid for WinUI 3",               "https://github.com/CommunityToolkit/Windows"),
        new("H.NotifyIcon.WinUI",                          "2.2.x",   "MIT",        "System tray icon for WinUI 3",       "https://github.com/HavenDV/H.NotifyIcon"),
        new("CommunityToolkit.Mvvm",                       "8.4.x",   "MIT",        "MVVM source generators",             "https://github.com/CommunityToolkit/dotnet"),
        new("Serilog",                                     "4.2.x",   "Apache-2.0", "Structured logging for .NET",        "https://github.com/serilog/serilog"),
        new("Velopack",                                    "0.0.x",   "MIT",        "Auto-update and packaging",          "https://github.com/velopack/velopack"),
        new("Microsoft.Extensions.DI",                     "9.0.x",   "MIT",        "Dependency injection container",     "https://github.com/dotnet/runtime"),
    ];

    public AboutViewModel(IUpdateService updateService)
    {
        Version = updateService.CurrentVersion;
    }

    [RelayCommand]
    private void OpenUrl(string? url)
    {
        if (string.IsNullOrEmpty(url)) return;
        try
        {
            Process.Start(new ProcessStartInfo { FileName = url, UseShellExecute = true });
        }
        catch
        {
            // Ignored
        }
    }
}

public sealed record ThirdPartyInfo(
    string Name,
    string Version,
    string License,
    string Description,
    string Url);
