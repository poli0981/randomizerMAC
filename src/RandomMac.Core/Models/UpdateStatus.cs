namespace RandomMac.Core.Models;

/// <summary>
/// Represents the outcome of an update check or download operation.
/// </summary>
public enum UpdateStatusCode
{
    /// <summary>Idle / not checked yet.</summary>
    Idle,

    /// <summary>Currently checking for updates.</summary>
    Checking,

    /// <summary>200 – A newer release was found.</summary>
    UpdateAvailable,

    /// <summary>200 – Already running the latest version.</summary>
    UpToDate,

    /// <summary>Currently downloading the update.</summary>
    Downloading,

    /// <summary>Download complete, ready to install.</summary>
    ReadyToInstall,

    /// <summary>Network error – could not reach GitHub.</summary>
    ConnectionError,

    /// <summary>404 – No releases found for the repository.</summary>
    ReleaseNotFound,

    /// <summary>403 – GitHub API rate limit exceeded.</summary>
    RateLimitExceeded,

    /// <summary>5xx – GitHub server error.</summary>
    ServerError,

    /// <summary>Response could not be parsed.</summary>
    ParseError,

    /// <summary>Download or install failed.</summary>
    InstallError,

    /// <summary>An unexpected error occurred.</summary>
    UnknownError
}

/// <summary>
/// Result of an update check.
/// </summary>
public sealed class UpdateCheckResult
{
    public required UpdateStatusCode Status { get; init; }
    public string? LatestVersion { get; init; }
    public string? CurrentVersion { get; init; }
    public string? ReleaseNotes { get; init; }
    public string? DownloadUrl { get; init; }
    public string? HtmlUrl { get; init; }
    public DateTime? PublishedAt { get; init; }
    public string? ErrorMessage { get; init; }
    public int? HttpStatusCode { get; init; }

    /// <summary>Human-readable status message.</summary>
    public string DisplayMessage => Status switch
    {
        UpdateStatusCode.Idle => "Click 'Check for Updates' to get started.",
        UpdateStatusCode.Checking => "Checking for updates...",
        UpdateStatusCode.UpdateAvailable => $"Update available: v{LatestVersion} (current: v{CurrentVersion})",
        UpdateStatusCode.UpToDate => $"You are running the latest version (v{CurrentVersion}).",
        UpdateStatusCode.Downloading => "Downloading update...",
        UpdateStatusCode.ReadyToInstall => "Update downloaded. Restart to apply.",
        UpdateStatusCode.ConnectionError => $"Connection error: {ErrorMessage}",
        UpdateStatusCode.ReleaseNotFound => "No releases found for this repository (HTTP 404).",
        UpdateStatusCode.RateLimitExceeded => "GitHub API rate limit exceeded (HTTP 403). Try again later.",
        UpdateStatusCode.ServerError => $"GitHub server error (HTTP {HttpStatusCode}). Try again later.",
        UpdateStatusCode.ParseError => $"Failed to parse update response: {ErrorMessage}",
        UpdateStatusCode.InstallError => $"Install failed: {ErrorMessage}",
        UpdateStatusCode.UnknownError => $"Unexpected error: {ErrorMessage}",
        _ => "Unknown status."
    };
}
