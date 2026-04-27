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
    private DateTime? _lastCheckedAt;

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

    [ObservableProperty]
    private string _releaseNotes = "";

    [ObservableProperty]
    private string _publishedDate = "";

    [ObservableProperty]
    private string _releaseUrl = "";

    [ObservableProperty]
    private string _lastCheckedDisplay = "";

    public string CurrentVersionDisplay => Loc.Get("Update_VersionFormat", CurrentVersion);

    public string PublishedDateDisplay
        => string.IsNullOrEmpty(PublishedDate) ? "" : Loc.Get("Update_PublishedFormat", PublishedDate);

    /// <summary>True when status reflects "no update needed".</summary>
    public bool IsUpToDateState => StatusCode == UpdateStatusCode.UpToDate;

    /// <summary>True for any non-success terminal status.</summary>
    public bool IsErrorState => StatusCode is
        UpdateStatusCode.ConnectionError or
        UpdateStatusCode.ReleaseNotFound or
        UpdateStatusCode.RateLimitExceeded or
        UpdateStatusCode.ServerError or
        UpdateStatusCode.ParseError or
        UpdateStatusCode.InstallError or
        UpdateStatusCode.UnknownError;

    /// <summary>True when no check has run yet (initial state).</summary>
    public bool IsIdle => StatusCode == UpdateStatusCode.Idle;

    /// <summary>True after at least one check has completed.</summary>
    public bool HasLastChecked => _lastCheckedAt.HasValue;

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

    partial void OnStatusCodeChanged(UpdateStatusCode value)
    {
        OnPropertyChanged(nameof(IsUpToDateState));
        OnPropertyChanged(nameof(IsErrorState));
        OnPropertyChanged(nameof(IsIdle));
    }

    [RelayCommand]
    private async Task CheckForUpdateAsync()
    {
        IsChecking = true;
        HasUpdate = false;
        StatusCode = UpdateStatusCode.Checking;
        StatusMessage = "Checking for updates...";

        try
        {
            var result = await _updateService.CheckForUpdateAsync();
            _lastCheckResult = result;
            _lastCheckedAt = DateTime.Now;

            StatusCode = result.Status;
            StatusMessage = result.DisplayMessage;

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

            UpdateLastCheckedDisplay();

            _logger.LogInformation("Update check result: {Status} (HTTP {Code})",
                result.Status, result.HttpStatusCode);
        }
        catch (Exception ex)
        {
            StatusCode = UpdateStatusCode.UnknownError;
            StatusMessage = $"Unexpected error: {ex.Message}";
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
        StatusMessage = "Downloading update...";

        try
        {
            var result = await _updateService.DownloadAndApplyAsync(_lastCheckResult);

            StatusCode = result.Status;
            StatusMessage = result.DisplayMessage;

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

    private void UpdateLastCheckedDisplay()
    {
        if (_lastCheckedAt is null)
        {
            LastCheckedDisplay = "";
        }
        else
        {
            var delta = DateTime.Now - _lastCheckedAt.Value;
            LastCheckedDisplay = "Last checked: " + FormatRelative(delta, _lastCheckedAt.Value);
        }
        OnPropertyChanged(nameof(HasLastChecked));
    }

    private static string FormatRelative(TimeSpan delta, DateTime when)
    {
        if (delta.TotalSeconds < 60) return "just now";
        if (delta.TotalMinutes < 60) return $"{(int)delta.TotalMinutes} min ago";
        if (delta.TotalHours < 24) return $"{(int)delta.TotalHours} h ago";
        return when.ToString("yyyy-MM-dd HH:mm");
    }
}
