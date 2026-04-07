using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using Microsoft.Extensions.Logging;
using RandomMac.App.Localization;
using RandomMac.App.Services;
using RandomMac.Core.Models;
using RandomMac.Core.Services.Interfaces;
using System.Diagnostics;

namespace RandomMac.App.ViewModels;

public partial class UpdateViewModel : ViewModelBase
{
    private readonly IUpdateService _updateService;
    private readonly NotificationService _notificationService;
    private readonly ILogger<UpdateViewModel> _logger;

    private UpdateCheckResult? _lastCheckResult;

    public override string Title => "Update";
    public override string IconKey => "ArrowSync";

    [ObservableProperty]
    private string _currentVersion = "";

    [ObservableProperty]
    private string _statusMessage = "Click 'Check for Updates' to get started.";

    [ObservableProperty]
    private UpdateStatusCode _statusCode = UpdateStatusCode.Idle;

    [ObservableProperty]
    private bool _isChecking;

    [ObservableProperty]
    private bool _isDownloading;

    [ObservableProperty]
    private bool _hasUpdate;

    [ObservableProperty]
    private string _latestVersion = "";

    public string CurrentVersionDisplay => Loc.Get("Update_VersionFormat", CurrentVersion);

    [ObservableProperty]
    private string _releaseNotes = "";

    [ObservableProperty]
    private string _publishedDate = "";

    public string PublishedDateDisplay => string.IsNullOrEmpty(PublishedDate) ? "" : Loc.Get("Update_PublishedFormat", PublishedDate);

    [ObservableProperty]
    private string _releaseUrl = "";

    [ObservableProperty]
    private string _statusBadge = "";

    public UpdateViewModel(
        IUpdateService updateService,
        NotificationService notificationService,
        ILogger<UpdateViewModel> logger)
    {
        _updateService = updateService;
        _notificationService = notificationService;
        _logger = logger;

        CurrentVersion = _updateService.CurrentVersion;
    }

    [RelayCommand]
    private async Task CheckForUpdateAsync()
    {
        IsChecking = true;
        HasUpdate = false;
        StatusCode = UpdateStatusCode.Checking;
        StatusBadge = "CHECKING";
        StatusMessage = "Checking for updates...";

        try
        {
            var result = await _updateService.CheckForUpdateAsync();
            _lastCheckResult = result;

            StatusCode = result.Status;
            StatusMessage = result.DisplayMessage;
            StatusBadge = FormatBadge(result.Status, result.HttpStatusCode);

            if (result.Status == UpdateStatusCode.UpdateAvailable)
            {
                HasUpdate = true;
                LatestVersion = result.LatestVersion ?? "";
                ReleaseNotes = result.ReleaseNotes ?? "No release notes.";
                PublishedDate = result.PublishedAt?.ToString("yyyy-MM-dd HH:mm") ?? "";
                ReleaseUrl = result.HtmlUrl ?? "";
                _notificationService.Info(Loc.Get("Notif_UpdateAvailable", result.LatestVersion ?? ""));
            }
            else if (result.Status == UpdateStatusCode.UpToDate)
            {
                _notificationService.Success(Loc.Get("Notif_UpToDate"));
            }
            else
            {
                _notificationService.Warning(result.DisplayMessage);
            }

            _logger.LogInformation("Update check result: {Status} (HTTP {Code})",
                result.Status, result.HttpStatusCode);
        }
        catch (Exception ex)
        {
            StatusCode = UpdateStatusCode.UnknownError;
            StatusMessage = $"Unexpected error: {ex.Message}";
            StatusBadge = "ERROR";
            _logger.LogError(ex, "Update check failed unexpectedly");
        }
        finally
        {
            IsChecking = false;
        }
    }

    [RelayCommand]
    private async Task DownloadUpdateAsync()
    {
        if (_lastCheckResult is null || _lastCheckResult.Status != UpdateStatusCode.UpdateAvailable)
            return;

        IsDownloading = true;
        StatusCode = UpdateStatusCode.Downloading;
        StatusBadge = "DOWNLOADING";
        StatusMessage = "Downloading update...";

        try
        {
            var result = await _updateService.DownloadAndApplyAsync(_lastCheckResult);

            StatusCode = result.Status;
            StatusMessage = result.DisplayMessage;
            StatusBadge = FormatBadge(result.Status, result.HttpStatusCode);

            if (result.Status == UpdateStatusCode.ReadyToInstall)
            {
                _notificationService.Success(Loc.Get("Notif_UpdateDownloaded"));
            }
            else
            {
                _notificationService.Error(result.DisplayMessage);
            }
        }
        catch (Exception ex)
        {
            StatusCode = UpdateStatusCode.InstallError;
            StatusMessage = $"Download failed: {ex.Message}";
            StatusBadge = "ERROR";
            _logger.LogError(ex, "Update download failed");
        }
        finally
        {
            IsDownloading = false;
        }
    }

    [RelayCommand]
    private void OpenReleasePage()
    {
        if (string.IsNullOrEmpty(ReleaseUrl)) return;

        try
        {
            Process.Start(new ProcessStartInfo
            {
                FileName = ReleaseUrl,
                UseShellExecute = true
            });
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Failed to open release page");
        }
    }

    private static string FormatBadge(UpdateStatusCode status, int? httpCode) => status switch
    {
        UpdateStatusCode.Idle => "IDLE",
        UpdateStatusCode.Checking => "CHECKING",
        UpdateStatusCode.UpdateAvailable => $"200 UPDATE",
        UpdateStatusCode.UpToDate => "200 OK",
        UpdateStatusCode.Downloading => "DOWNLOADING",
        UpdateStatusCode.ReadyToInstall => "READY",
        UpdateStatusCode.ConnectionError => "CONN ERR",
        UpdateStatusCode.ReleaseNotFound => "404",
        UpdateStatusCode.RateLimitExceeded => "403 LIMIT",
        UpdateStatusCode.ServerError => $"{httpCode} SERVER",
        UpdateStatusCode.ParseError => "PARSE ERR",
        UpdateStatusCode.InstallError => "INSTALL ERR",
        UpdateStatusCode.UnknownError => "ERROR",
        _ => "UNKNOWN"
    };
}
