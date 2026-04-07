using RandomMac.Core.Models;

namespace RandomMac.Core.Services.Interfaces;

public interface IUpdateService
{
    string CurrentVersion { get; }
    Task<UpdateCheckResult> CheckForUpdateAsync();
    Task<UpdateCheckResult> DownloadAndApplyAsync(UpdateCheckResult updateInfo);
}
