//! External links the UI may open. A compromised or buggy webview must not be able to make the
//! (possibly elevated) app launch arbitrary URLs or programs, so only these prefixes are allowed.

const ALLOWED: &[&str] = &[
    "https://github.com/",
    "https://tauri.app",
    "https://v2.tauri.app",
    "https://svelte.dev",
    "https://www.rust-lang.org",
    "https://lucide.dev",
    "https://standards-oui.ieee.org",
    "https://opensource.org/",
    "https://ko-fi.com/skullmute",
    "https://www.buymeacoffee.com/skullmute",
    "https://www.patreon.com/skullmute",
    "https://paypal.me/DungDang212",
    "https://x.com/SkullMute0011",
    "https://youtube.com/@SkullMute",
    "https://discord.gg/",
    "https://steamcommunity.com/profiles/76561199544666292",
    "https://bsky.app/profile/skullmute0011.bsky.social",
    "https://mastodon.social/@skullmute1122",
    "https://t.me/my_skull_bot",
    "https://t.me/SkullMute0011",
    "mailto:lopop05905@proton.me",
];

pub fn is_allowed(url: &str) -> bool {
    let Ok(parsed) = url::Url::parse(url) else {
        return false;
    };
    if !matches!(parsed.scheme(), "https" | "mailto") {
        return false;
    }
    let normalized = parsed.as_str();
    ALLOWED.iter().any(|p| normalized.starts_with(p))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allowlist() {
        assert!(is_allowed(
            "https://github.com/poli0981/randomizerMAC/releases"
        ));
        assert!(is_allowed("https://ko-fi.com/skullmute"));
        assert!(is_allowed("mailto:lopop05905@proton.me"));
        assert!(!is_allowed("http://github.com/poli0981"));
        assert!(!is_allowed("https://evil.example/https://github.com/"));
        assert!(!is_allowed("file:///C:/Windows/System32/calc.exe"));
        assert!(!is_allowed("javascript:alert(1)"));
        assert!(!is_allowed("not a url"));
    }
}
