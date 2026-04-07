using RandomMac.Core.Models;
using Xunit;

namespace RandomMac.Tests;

public class UpdateStatusTests
{
    [Fact]
    public void DisplayMessage_Idle_ReturnsExpected()
    {
        var result = new UpdateCheckResult { Status = UpdateStatusCode.Idle };
        Assert.Contains("Check for Updates", result.DisplayMessage);
    }

    [Fact]
    public void DisplayMessage_UpdateAvailable_IncludesVersions()
    {
        var result = new UpdateCheckResult
        {
            Status = UpdateStatusCode.UpdateAvailable,
            LatestVersion = "2.0.0",
            CurrentVersion = "1.0.0"
        };
        Assert.Contains("2.0.0", result.DisplayMessage);
        Assert.Contains("1.0.0", result.DisplayMessage);
    }

    [Fact]
    public void DisplayMessage_UpToDate_IncludesVersion()
    {
        var result = new UpdateCheckResult
        {
            Status = UpdateStatusCode.UpToDate,
            CurrentVersion = "1.0.0"
        };
        Assert.Contains("latest", result.DisplayMessage);
    }

    [Fact]
    public void DisplayMessage_ConnectionError_IncludesMessage()
    {
        var result = new UpdateCheckResult
        {
            Status = UpdateStatusCode.ConnectionError,
            ErrorMessage = "Timeout"
        };
        Assert.Contains("Timeout", result.DisplayMessage);
    }

    [Fact]
    public void DisplayMessage_ReleaseNotFound_Mentions404()
    {
        var result = new UpdateCheckResult { Status = UpdateStatusCode.ReleaseNotFound };
        Assert.Contains("404", result.DisplayMessage);
    }

    [Fact]
    public void DisplayMessage_RateLimitExceeded_Mentions403()
    {
        var result = new UpdateCheckResult { Status = UpdateStatusCode.RateLimitExceeded };
        Assert.Contains("403", result.DisplayMessage);
    }

    [Fact]
    public void DisplayMessage_ServerError_IncludesHttpCode()
    {
        var result = new UpdateCheckResult
        {
            Status = UpdateStatusCode.ServerError,
            HttpStatusCode = 502
        };
        Assert.Contains("502", result.DisplayMessage);
    }

    [Theory]
    [InlineData(UpdateStatusCode.Idle)]
    [InlineData(UpdateStatusCode.Checking)]
    [InlineData(UpdateStatusCode.Downloading)]
    [InlineData(UpdateStatusCode.ReadyToInstall)]
    [InlineData(UpdateStatusCode.ParseError)]
    [InlineData(UpdateStatusCode.InstallError)]
    [InlineData(UpdateStatusCode.UnknownError)]
    public void DisplayMessage_AllCodes_ReturnNonEmpty(UpdateStatusCode code)
    {
        var result = new UpdateCheckResult
        {
            Status = code,
            CurrentVersion = "1.0.0",
            ErrorMessage = "test error"
        };
        Assert.False(string.IsNullOrEmpty(result.DisplayMessage));
    }
}
