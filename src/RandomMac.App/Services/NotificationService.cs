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
/// In-app toast notifications shown by <c>NotificationPopup</c>. Marshals
/// background-thread requests onto <see cref="App.MainDispatcher"/> so the
/// observable collection is mutated on the UI thread.
/// </summary>
public sealed class NotificationService
{
    public ObservableCollection<NotificationItem> Notifications { get; } = [];

    public void Show(string message, NotificationType type = NotificationType.Info, int durationMs = 4000)
    {
        var item = new NotificationItem { Message = message, Type = type };

        var dispatcher = App.MainDispatcher;
        if (dispatcher is null)
        {
            // Pre-launch: drop silently. Auto-change before MainWindow Activate
            // is the only known caller and its outcome is logged to Serilog anyway.
            return;
        }

        dispatcher.TryEnqueue(() => AddAndExpire(item, durationMs));
    }

    private async void AddAndExpire(NotificationItem item, int durationMs)
    {
        Notifications.Add(item);

        while (Notifications.Count > 3)
            Notifications.RemoveAt(0);

        await Task.Delay(durationMs);

        Notifications.Remove(item);
    }

    public void Success(string message) => Show(message, NotificationType.Success);
    public void Error(string message) => Show(message, NotificationType.Error, 6000);
    public void Warning(string message) => Show(message, NotificationType.Warning, 5000);
    public void Info(string message) => Show(message, NotificationType.Info);
}
