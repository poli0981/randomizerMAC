using Microsoft.Extensions.Logging;
using RandomMac.Core.Models;
using RandomMac.Core.Services.Interfaces;
using System.Reflection;
using Velopack;
using Velopack.Sources;

namespace RandomMac.Core.Services.Implementations;

public sealed class UpdateService : IUpdateService
{
    private readonly ILogger<UpdateService> _logger;
    private readonly UpdateManager _updateManager;
    private UpdateInfo? _lastUpdateInfo;

    private const string GitHubRepoUrl = "https://github.com/poli0981/randomizerMAC";

    public string CurrentVersion { get; }

    public UpdateService(ILogger<UpdateService> logger)
    {
        _logger = logger;
        CurrentVersion = Assembly.GetEntryAssembly()?.GetName().Version?.ToString(3) ?? "0.0.1";

        var source = new GithubSource(GitHubRepoUrl, accessToken: null, prerelease: false);
        _updateManager = new UpdateManager(source);
    }

    public async Task<UpdateCheckResult> CheckForUpdateAsync()
    {
        _logger.LogInformation("Checking for updates via Velopack (GitHub: {Url})", GitHubRepoUrl);

        try
        {
            if (!_updateManager.IsInstalled)
            {
                _logger.LogWarning("App is not installed via Velopack (running from IDE/debug). Falling back to GitHub API check.");
                return await CheckViaGitHubApiAsync();
            }

            var updateInfo = await _updateManager.CheckForUpdatesAsync();

            if (updateInfo is null)
            {
                _logger.LogInformation("No updates available, already on latest version");
                return new UpdateCheckResult
                {
                    Status = UpdateStatusCode.UpToDate,
                    CurrentVersion = CurrentVersion,
                    HttpStatusCode = 200
                };
            }

            _lastUpdateInfo = updateInfo;
            var latestVersion = updateInfo.TargetFullRelease.Version.ToString();

            _logger.LogInformation("Update available: v{Latest} (current: v{Current})", latestVersion, CurrentVersion);

            return new UpdateCheckResult
            {
                Status = UpdateStatusCode.UpdateAvailable,
                LatestVersion = latestVersion,
                CurrentVersion = CurrentVersion,
                HtmlUrl = $"{GitHubRepoUrl}/releases/tag/v{latestVersion}",
                HttpStatusCode = 200
            };
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Velopack update check failed, falling back to GitHub API");
            return await CheckViaGitHubApiAsync();
        }
    }

    public async Task<UpdateCheckResult> DownloadAndApplyAsync(UpdateCheckResult updateInfo)
    {
        _logger.LogInformation("Downloading update v{Version} via Velopack", updateInfo.LatestVersion);

        try
        {
            if (_updateManager.IsInstalled && _lastUpdateInfo is not null)
            {
                await _updateManager.DownloadUpdatesAsync(_lastUpdateInfo);

                _logger.LogInformation("Update downloaded. Ready to apply and restart.");
                return new UpdateCheckResult
                {
                    Status = UpdateStatusCode.ReadyToInstall,
                    CurrentVersion = CurrentVersion,
                    LatestVersion = updateInfo.LatestVersion,
                    ReleaseNotes = updateInfo.ReleaseNotes
                };
            }

            // Fallback: not installed via Velopack, just provide download URL
            return new UpdateCheckResult
            {
                Status = UpdateStatusCode.ReadyToInstall,
                CurrentVersion = CurrentVersion,
                LatestVersion = updateInfo.LatestVersion,
                DownloadUrl = updateInfo.HtmlUrl,
                ReleaseNotes = updateInfo.ReleaseNotes
            };
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Failed to download update via Velopack");
            return new UpdateCheckResult
            {
                Status = UpdateStatusCode.InstallError,
                CurrentVersion = CurrentVersion,
                LatestVersion = updateInfo.LatestVersion,
                ErrorMessage = ex.Message
            };
        }
    }

    /// <summary>
    /// Apply downloaded update and restart the application.
    /// Only works when app is installed via Velopack.
    /// </summary>
    public void ApplyAndRestart()
    {
        if (_updateManager.IsInstalled && _lastUpdateInfo is not null)
        {
            _logger.LogInformation("Applying update and restarting...");
            _updateManager.ApplyUpdatesAndRestart(_lastUpdateInfo.TargetFullRelease);
        }
    }

    /// <summary>
    /// Fallback: check GitHub API directly when not installed via Velopack (e.g. debug/dev).
    /// </summary>
    private async Task<UpdateCheckResult> CheckViaGitHubApiAsync()
    {
        try
        {
            using var http = new HttpClient();
            http.DefaultRequestHeaders.UserAgent.ParseAdd($"RandomMac/{CurrentVersion}");
            http.DefaultRequestHeaders.Accept.ParseAdd("application/vnd.github+json");
            http.Timeout = TimeSpan.FromSeconds(15);

            var apiUrl = $"https://api.github.com/repos/poli0981/randomizerMAC/releases/latest";
            var response = await http.GetAsync(apiUrl);
            var code = (int)response.StatusCode;

            if (!response.IsSuccessStatusCode)
            {
                var status = code switch
                {
                    404 => UpdateStatusCode.ReleaseNotFound,
                    403 => UpdateStatusCode.RateLimitExceeded,
                    >= 500 => UpdateStatusCode.ServerError,
                    _ => UpdateStatusCode.UnknownError
                };
                return new UpdateCheckResult
                {
                    Status = status,
                    CurrentVersion = CurrentVersion,
                    HttpStatusCode = code,
                    ErrorMessage = $"HTTP {code}"
                };
            }

            var json = await response.Content.ReadAsStringAsync();
            var doc = System.Text.Json.JsonDocument.Parse(json);
            var root = doc.RootElement;

            var tagName = root.GetProperty("tag_name").GetString()?.TrimStart('v', 'V') ?? "0.0.0";
            var htmlUrl = root.GetProperty("html_url").GetString();
            var body = root.TryGetProperty("body", out var bodyProp) ? bodyProp.GetString() : null;
            var publishedAt = root.TryGetProperty("published_at", out var pubProp) ? pubProp.GetDateTime() : (DateTime?)null;

            var isNewer = Version.TryParse(tagName, out var latestVer)
                          && Version.TryParse(CurrentVersion, out var currentVer)
                          && latestVer > currentVer;

            return new UpdateCheckResult
            {
                Status = isNewer ? UpdateStatusCode.UpdateAvailable : UpdateStatusCode.UpToDate,
                LatestVersion = tagName,
                CurrentVersion = CurrentVersion,
                ReleaseNotes = body,
                HtmlUrl = htmlUrl,
                PublishedAt = publishedAt,
                HttpStatusCode = code
            };
        }
        catch (HttpRequestException ex)
        {
            return new UpdateCheckResult
            {
                Status = UpdateStatusCode.ConnectionError,
                CurrentVersion = CurrentVersion,
                ErrorMessage = ex.Message
            };
        }
        catch (TaskCanceledException)
        {
            return new UpdateCheckResult
            {
                Status = UpdateStatusCode.ConnectionError,
                CurrentVersion = CurrentVersion,
                ErrorMessage = "Request timed out."
            };
        }
        catch (Exception ex)
        {
            return new UpdateCheckResult
            {
                Status = UpdateStatusCode.UnknownError,
                CurrentVersion = CurrentVersion,
                ErrorMessage = ex.Message
            };
        }
    }
}
