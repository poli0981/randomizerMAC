using Avalonia.Controls;
using Avalonia.Platform.Storage;
using Avalonia.Threading;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using Microsoft.Extensions.Logging;
using RandomMac.App.Services;
using System.Collections.ObjectModel;

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

        // Load existing entries
        foreach (var entry in LogEntrySink.Instance.Entries)
            LogLines.Add(entry.Display);
        LogCount = LogLines.Count;

        // Subscribe to new entries
        LogEntrySink.Instance.SetCallback(OnNewLogEntry);
    }

    private void OnNewLogEntry(LogEntry entry)
    {
        Dispatcher.UIThread.Post(() =>
        {
            var line = entry.Display;

            // Apply filter
            if (!string.IsNullOrEmpty(FilterText)
                && !line.Contains(FilterText, StringComparison.OrdinalIgnoreCase))
                return;

            LogLines.Add(line);
            LogCount = LogLines.Count;

            // Keep max 1000 lines in UI
            if (LogLines.Count > 1000)
                LogLines.RemoveAt(0);
        });
    }

    partial void OnFilterTextChanged(string value)
    {
        // Re-filter all entries
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
            var topLevel = TopLevel.GetTopLevel(
                (App.Services.GetService(typeof(Views.MainWindow)) as Window)!);
            if (topLevel is null) return;

            var file = await topLevel.StorageProvider.SaveFilePickerAsync(new FilePickerSaveOptions
            {
                Title = "Export Log",
                DefaultExtension = "txt",
                FileTypeChoices =
                [
                    new FilePickerFileType("Text File") { Patterns = ["*.txt"] },
                    new FilePickerFileType("All Files") { Patterns = ["*.*"] }
                ],
                SuggestedFileName = $"randommac-log-{DateTime.Now:yyyyMMdd-HHmmss}.txt"
            });

            if (file is not null)
            {
                var content = string.Join(Environment.NewLine, LogLines);
                await File.WriteAllTextAsync(file.Path.LocalPath, content);
                _logger.LogInformation("Log exported to {Path}", file.Path.LocalPath);
            }
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Failed to export log");
        }
    }
}
