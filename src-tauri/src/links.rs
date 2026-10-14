//! External links the UI may open. A compromised or buggy webview must not be able to make the
//! (possibly elevated) app launch arbitrary URLs or programs, so only these destinations are
//! allowed: same scheme, exactly the same host, and a path at or below the listed one.

const ALLOWED: &[&str] = &[
    "https://github.com/",
    "https://tauri.app/",
    "https://v2.tauri.app/",
    "https://svelte.dev/",
    "https://www.rust-lang.org/",
    "https://lucide.dev/",
    "https://standards-oui.ieee.org/",
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

fn matches(url: &url::Url, entry: &str) -> bool {
    let Ok(entry) = url::Url::parse(entry) else {
        return false;
    };
    if url.scheme() != entry.scheme() {
        return false;
    }
    if url.scheme() == "mailto" {
        return url.path() == entry.path();
    }
    if url.host_str() != entry.host_str() || url.port() != entry.port() {
        return false;
    }
    let (path, base) = (url.path(), entry.path());
    path == base
        || (base.ends_with('/') && path.starts_with(base))
        || path
            .strip_prefix(base)
            .is_some_and(|rest| rest.starts_with('/'))
}

/// Returns the normalized URL to open, or `None` when it is not allowed. Callers must open the
/// returned string, not their input.
pub fn allowed(url: &str) -> Option<String> {
    let parsed = url::Url::parse(url).ok()?;
    if !matches!(parsed.scheme(), "https" | "mailto") || !parsed.username().is_empty() {
        return None;
    }
    ALLOWED
        .iter()
        .any(|entry| matches(&parsed, entry))
        .then(|| parsed.as_str().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allowlist() {
        assert!(allowed("https://github.com/poli0981/randomizerMAC/releases").is_some());
        assert!(allowed("https://ko-fi.com/skullmute").is_some());
        assert!(allowed("https://svelte.dev").is_some());
        assert!(allowed("mailto:lopop05905@proton.me").is_some());
        assert!(allowed("http://github.com/poli0981").is_none());
        assert!(allowed("https://evil.example/https://github.com/").is_none());
        assert!(allowed("file:///C:/Windows/System32/calc.exe").is_none());
        assert!(allowed("javascript:alert(1)").is_none());
        assert!(allowed("not a url").is_none());
    }

    #[test]
    fn hosts_and_paths_must_match_exactly() {
        assert!(allowed("https://svelte.dev.attacker.example/").is_none());
        assert!(allowed("https://github.com.attacker.example/").is_none());
        assert!(allowed("https://ko-fi.com/skullmute-impostor").is_none());
        assert!(allowed("https://ko-fi.com/skullmute/posts").is_some());
        assert!(allowed("https://user@github.com/").is_none());
        assert!(allowed("mailto:someone@else.example").is_none());
        // The normalized form is what gets opened.
        assert_eq!(
            allowed("https://GITHUB.com/poli0981").as_deref(),
            Some("https://github.com/poli0981")
        );
    }
}
