using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Logging;
using RandomMac.App.Services;
using System.Collections.ObjectModel;
using Windows.Storage.Pickers;

namespace RandomMac.App.ViewModels;

public partial class LogViewModel : ViewModelBase
{
    private readonly ILogger<LogViewModel> _logger;

    public override string Title => "Log";
    public override string IconKey => "Document";

    public ObservableCollection<string> LogLines { get; } = [];

    [ObservableProperty]
    private bool _autoScroll = true;

    [ObservableProperty]
    private string _filterText = "";

    [ObservableProperty]
    private int _logCount;

    public LogViewModel(ILogger<LogViewModel> logger)
    {
        _logger = logger;

        foreach (var entry in LogEntrySink.Instance.Entries)
            LogLines.Add(entry.Display);
        LogCount = LogLines.Count;

        LogEntrySink.Instance.SetCallback(OnNewLogEntry);
    }

    private void OnNewLogEntry(LogEntry entry)
    {
        // Marshal to UI thread via the dispatcher captured by App after MainWindow init.
        App.MainDispatcher?.TryEnqueue(() =>
        {
            var line = entry.Display;

            if (!string.IsNullOrEmpty(FilterText)
                && !line.Contains(FilterText, StringComparison.OrdinalIgnoreCase))
                return;

            LogLines.Add(line);
            LogCount = LogLines.Count;

            if (LogLines.Count > 1000)
                LogLines.RemoveAt(0);
        });
    }

    partial void OnFilterTextChanged(string value)
    {
        LogLines.Clear();
        foreach (var entry in LogEntrySink.Instance.Entries)
        {
            if (string.IsNullOrEmpty(value) || entry.Display.Contains(value, StringComparison.OrdinalIgnoreCase))
                LogLines.Add(entry.Display);
        }
        LogCount = LogLines.Count;
    }

    [RelayCommand]
    private void ClearLog()
    {
        LogLines.Clear();
        LogEntrySink.Instance.Clear();
        LogCount = 0;
        _logger.LogInformation("Log cleared by user");
    }

    [RelayCommand]
    private async Task ExportLogAsync()
    {
        try
        {
            var picker = new FileSavePicker
            {
                SuggestedStartLocation = PickerLocationId.Desktop,
                SuggestedFileName = $"randommac-log-{DateTime.Now:yyyyMMdd-HHmmss}",
                DefaultFileExtension = ".txt"
            };
            picker.FileTypeChoices.Add("Text File", [".txt"]);
            picker.FileTypeChoices.Add("All Files", [".*"]);

            var hwnd = WinRT.Interop.WindowNative.GetWindowHandle(
                App.Services.GetRequiredService<MainWindow>());
            WinRT.Interop.InitializeWithWindow.Initialize(picker, hwnd);

            var file = await picker.PickSaveFileAsync();
            if (file is not null)
            {
                var content = string.Join(Environment.NewLine, LogLines);
                await File.WriteAllTextAsync(file.Path, content);
                _logger.LogInformation("Log exported to {Path}", file.Path);
            }
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Failed to export log");
        }
    }
}
