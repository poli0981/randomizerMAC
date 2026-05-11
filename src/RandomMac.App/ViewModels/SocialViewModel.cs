using System.Collections.ObjectModel;

namespace RandomMac.App.ViewModels;

/// <summary>
/// Surfaces the developer's social presence + Donate / Report Bug shortcuts.
/// All URL launching is handled by <see cref="Views.SocialView"/> code-behind
/// (Click handlers) to keep DataContext propagation through MenuFlyout
/// items robust — WinUI 3 flyouts can lose DataContext for Command bindings.
/// </summary>
public partial class SocialViewModel : ViewModelBase
{
    public override string Title => "Social";
    public override string IconKey => "People";

    public ObservableCollection<SocialLink> SocialLinks { get; } =
    [
        new("X (Twitter)",    "@SkullMute0011",  "https://x.com/SkullMute0011"),
        new("YouTube",        "@SkullMute",      "https://youtube.com/@SkullMute"),
        new("Discord (repo)", "Invite",          "https://discord.gg/2aNR3aVt"),
        new("Discord (game)", "Invite",          "https://discord.gg/kDM9GMu5vm"),
        new("Patreon",        "skullmute",       "https://www.patreon.com/skullmute"),
        new("Ko-fi",          "skullmute",       "https://ko-fi.com/skullmute"),
        new("Steam",          "Profile",         "https://steamcommunity.com/profiles/76561199544666292/"),
        new("Bluesky",        "@skullmute0011",  "https://bsky.app/profile/skullmute0011.bsky.social"),
        new("Mastodon",       "@skullmute1122",  "https://mastodon.social/@skullmute1122"),
        new("Telegram bot",   "@my_skull_bot",   "https://t.me/my_skull_bot"),
        new("Telegram",       "@SkullMute0011",  "https://t.me/SkullMute0011"),
    ];
}

public sealed record SocialLink(string Platform, string Handle, string Url);
