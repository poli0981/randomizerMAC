using RandomMac.Core.Models;

namespace RandomMac.Core.Services.Interfaces;

public interface IHistoryService
{
    IReadOnlyList<MacHistoryEntry> GetHistory();
    void Add(MacHistoryEntry entry);
    void Clear();
    Task LoadAsync();
    Task SaveAsync();
}
