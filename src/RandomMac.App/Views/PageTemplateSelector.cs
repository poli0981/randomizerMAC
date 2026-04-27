using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using RandomMac.App.ViewModels;

namespace RandomMac.App.Views;

/// <summary>
/// Selects the right page <see cref="DataTemplate"/> based on the bound
/// ViewModel type. Templates are declared in <c>MainWindow.xaml</c> resources.
/// </summary>
public sealed class PageTemplateSelector : DataTemplateSelector
{
    public DataTemplate? DashboardTemplate { get; set; }
    public DataTemplate? SettingsTemplate { get; set; }
    public DataTemplate? AboutTemplate { get; set; }
    public DataTemplate? LogTemplate { get; set; }
    public DataTemplate? UpdateTemplate { get; set; }

    protected override DataTemplate? SelectTemplateCore(object item) => item switch
    {
        DashboardViewModel => DashboardTemplate,
        SettingsViewModel => SettingsTemplate,
        AboutViewModel => AboutTemplate,
        LogViewModel => LogTemplate,
        UpdateViewModel => UpdateTemplate,
        _ => null
    };

    protected override DataTemplate? SelectTemplateCore(object item, DependencyObject container)
        => SelectTemplateCore(item);
}
