using CommunityToolkit.Mvvm.ComponentModel;

namespace RandomMac.App.ViewModels;

public abstract partial class ViewModelBase : ObservableObject
{
    public abstract string Title { get; }
    public abstract string IconKey { get; }
}
