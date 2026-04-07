using System.Runtime.Versioning;
using System.Security.Principal;

namespace RandomMac.Core.Helpers;

/// <summary>
/// Helper to check if the current process is running with administrator privileges.
/// </summary>
[SupportedOSPlatform("windows")]
public static class AdminHelper
{
    public static bool IsRunningAsAdmin()
    {
        using var identity = WindowsIdentity.GetCurrent();
        var principal = new WindowsPrincipal(identity);
        return principal.IsInRole(WindowsBuiltInRole.Administrator);
    }
}
