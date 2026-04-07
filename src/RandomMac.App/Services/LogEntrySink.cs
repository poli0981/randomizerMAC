using Serilog.Core;
using Serilog.Events;

namespace RandomMac.App.Services;

/// <summary>
/// A Serilog sink that buffers log events and raises a callback for real-time UI display.
/// </summary>
public sealed class LogEntrySink : ILogEventSink
{
    private readonly object _lock = new();
    private readonly List<LogEntry> _entries = [];
    private Action<LogEntry>? _onEntry;

    public static LogEntrySink Instance { get; } = new();

    public IReadOnlyList<LogEntry> Entries
    {
        get { lock (_lock) return [.. _entries]; }
    }

    public void SetCallback(Action<LogEntry> callback) => _onEntry = callback;

    public void Emit(LogEvent logEvent)
    {
        var entry = new LogEntry
        {
            Timestamp = logEvent.Timestamp.LocalDateTime,
            Level = logEvent.Level.ToString()[..3].ToUpperInvariant(),
            Message = logEvent.RenderMessage(),
            Exception = logEvent.Exception?.Message
        };

        lock (_lock)
        {
            _entries.Add(entry);
            if (_entries.Count > 500)
                _entries.RemoveAt(0);
        }

        _onEntry?.Invoke(entry);
    }

    public void Clear()
    {
        lock (_lock) _entries.Clear();
    }
}

public sealed class LogEntry
{
    public DateTime Timestamp { get; init; }
    public string Level { get; init; } = "";
    public string Message { get; init; } = "";
    public string? Exception { get; init; }

    public string Display => Exception is null
        ? $"[{Timestamp:HH:mm:ss}] [{Level}] {Message}"
        : $"[{Timestamp:HH:mm:ss}] [{Level}] {Message} | {Exception}";
}
