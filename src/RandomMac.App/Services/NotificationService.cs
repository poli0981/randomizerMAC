using Avalonia.Threading;
using System.Collections.ObjectModel;

namespace RandomMac.App.Services;

public enum NotificationType
{
    Info,
    Success,
    Warning,
    Error
}

public sealed class NotificationItem
{
    public string Message { get; init; } = "";
    public NotificationType Type { get; init; }
    public DateTime Timestamp { get; init; } = DateTime.Now;
}

/// <summary>
/// Manages in-app toast notifications displayed in the main window.
/// </summary>
public sealed class NotificationService
{
    public ObservableCollection<NotificationItem> Notifications { get; } = [];

    public void Show(string message, NotificationType type = NotificationType.Info, int durationMs = 4000)
    {
        var item = new NotificationItem { Message = message, Type = type };

        Dispatcher.UIThread.Post(async () =>
        {
            Notifications.Add(item);

            // Keep max 3 visible
            while (Notifications.Count > 3)
                Notifications.RemoveAt(0);

            await Task.Delay(durationMs);

            Notifications.Remove(item);
        });
    }

    public void Success(string message) => Show(message, NotificationType.Success);
    public void Error(string message) => Show(message, NotificationType.Error, 6000);
    public void Warning(string message) => Show(message, NotificationType.Warning, 5000);
    public void Info(string message) => Show(message, NotificationType.Info);
}
