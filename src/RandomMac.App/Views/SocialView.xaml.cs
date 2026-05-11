using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using System.Diagnostics;

namespace RandomMac.App.Views;

/// <summary>
/// Page hosting developer social links, Donate flyout, and Report Bug
/// shortcut. URL launching lives in code-behind (instead of a Command on
/// SocialViewModel) so MenuFlyoutItems don't depend on DataContext
/// propagation through the flyout — a known WinUI 3 fragility.
/// </summary>
public sealed partial class SocialView : UserControl
{
    public SocialView() => InitializeComponent();

    private void OnLinkClick(object sender, RoutedEventArgs e)
    {
        if (sender is FrameworkElement fe && fe.Tag is string url)
            Launch(url);
    }

    private static void Launch(string url)
    {
        if (string.IsNullOrEmpty(url)) return;
        try
        {
            Process.Start(new ProcessStartInfo { FileName = url, UseShellExecute = true });
        }
        catch
        {
            // Ignored — matches AboutViewModel.OpenUrl behavior.
        }
    }
}
