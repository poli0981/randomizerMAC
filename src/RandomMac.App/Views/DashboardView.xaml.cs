using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using RandomMac.App.Localization;
using System.ComponentModel;

namespace RandomMac.App.Views;

public sealed partial class DashboardView : UserControl
{
    private static readonly string[] HistoryHeaderKeys =
    [
        "Dashboard_Time",
        "Dashboard_Adapter",
        "Dashboard_Previous",
        "Dashboard_New",
        "Dashboard_Status",
    ];

    public DashboardView()
    {
        InitializeComponent();
        Loaded += OnLoaded;
        Unloaded += OnUnloaded;
    }

    private void OnLoaded(object sender, RoutedEventArgs e)
    {
        ApplyHistoryHeaders();
        Loc.Instance.PropertyChanged += OnLocChanged;
    }

    private void OnUnloaded(object sender, RoutedEventArgs e)
    {
        Loc.Instance.PropertyChanged -= OnLocChanged;
    }

    private void OnLocChanged(object? sender, PropertyChangedEventArgs e)
    {
        // Loc raises only "Item[]" on culture change.
        ApplyHistoryHeaders();
    }

    /// <summary>
    /// DataGridColumn isn't a FrameworkElement — XAML bindings on its Header
    /// don't resolve. Set the localized strings here so they update when the
    /// language changes.
    /// </summary>
    private void ApplyHistoryHeaders()
    {
        if (HistoryGrid?.Columns is not { Count: > 0 } cols) return;

        for (var i = 0; i < cols.Count && i < HistoryHeaderKeys.Length; i++)
            cols[i].Header = Loc.Get(HistoryHeaderKeys[i]);
    }
}
