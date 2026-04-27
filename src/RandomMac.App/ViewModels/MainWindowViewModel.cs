using CommunityToolkit.Mvvm.ComponentModel;
using RandomMac.App.Localization;
using System.Collections.ObjectModel;

namespace RandomMac.App.ViewModels;

public partial class MainWindowViewModel : ObservableObject
{
    [ObservableProperty]
    private ViewModelBase _currentPage;

    [ObservableProperty]
    private NavItem? _selectedNav;

    public ObservableCollection<NavItem> NavItems { get; }

    public MainWindowViewModel(
        DashboardViewModel dashboard,
        SettingsViewModel settings,
        AboutViewModel about,
        UpdateViewModel update,
        LogViewModel log)
    {
        // Glyphs are Segoe Fluent Icons codepoints (https://aka.ms/SegoeFluentIcons).
        NavItems =
        [
            new NavItem("Nav_Dashboard", "", dashboard),  // Home
            new NavItem("Nav_Settings",  "", settings),   // Settings (gear)
            new NavItem("Nav_Log",       "", log),        // Document
            new NavItem("Nav_Update",    "", update),     // Refresh / Sync
            new NavItem("Nav_About",     "", about),      // Info
        ];

        _currentPage = dashboard;
        _selectedNav = NavItems[0];

        // Refresh nav labels when language changes
        Loc.Instance.PropertyChanged += (_, _) =>
        {
            foreach (var item in NavItems)
                item.RefreshLabel();
        };
    }

    partial void OnSelectedNavChanged(NavItem? value)
    {
        if (value is not null)
            CurrentPage = value.ViewModel;
    }
}

public partial class NavItem : ObservableObject
{
    public string LabelKey { get; }
    public string Icon { get; }
    public ViewModelBase ViewModel { get; }

    [ObservableProperty]
    private string _label;

    public NavItem(string labelKey, string icon, ViewModelBase viewModel)
    {
        LabelKey = labelKey;
        Icon = icon;
        ViewModel = viewModel;
        _label = Loc.Get(labelKey);
    }

    public void RefreshLabel() => Label = Loc.Get(LabelKey);
}
