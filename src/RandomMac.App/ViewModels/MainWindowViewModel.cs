using CommunityToolkit.Mvvm.ComponentModel;
using RandomMac.App.Localization;
using System.Collections.ObjectModel;

namespace RandomMac.App.ViewModels;

public partial class MainWindowViewModel : ObservableObject
{
    [ObservableProperty]
    private ViewModelBase _currentPage;

    [ObservableProperty]
    private int _selectedNavIndex;

    public ObservableCollection<NavItem> NavItems { get; }

    public MainWindowViewModel(
        DashboardViewModel dashboard,
        SettingsViewModel settings,
        AboutViewModel about,
        UpdateViewModel update,
        LogViewModel log)
    {
        NavItems =
        [
            new NavItem("Nav_Dashboard", "Home", dashboard),
            new NavItem("Nav_Settings", "Settings", settings),
            new NavItem("Nav_Log", "Document", log),
            new NavItem("Nav_Update", "ArrowSync", update),
            new NavItem("Nav_About", "Info", about),
        ];

        _currentPage = dashboard;
        _selectedNavIndex = 0;

        // Update nav labels when language changes
        Loc.Instance.PropertyChanged += (_, _) =>
        {
            foreach (var item in NavItems)
                item.RefreshLabel();
        };
    }

    partial void OnSelectedNavIndexChanged(int value)
    {
        if (value >= 0 && value < NavItems.Count)
        {
            CurrentPage = NavItems[value].ViewModel;
        }
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
