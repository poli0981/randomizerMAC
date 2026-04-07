using RandomMac.Core.Models;

namespace RandomMac.Core.Services.Interfaces;

public interface ISettingsService
{
    AppSettings Settings { get; }
    Task LoadAsync();
    Task SaveAsync();
    Task ExportAsync(string filePath);
    Task ImportAsync(string filePath);
    string GetFilePath();
}
