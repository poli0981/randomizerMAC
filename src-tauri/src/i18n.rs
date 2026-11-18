//! The few strings produced on the Rust side (tray menu, OS notifications, file dialogs).
//! Everything else is translated in the frontend (`src/lib/i18n`).

use randommac_core::settings::Language;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    En,
    Vi,
}

impl Lang {
    pub fn resolve(setting: Language) -> Self {
        match setting {
            Language::En => Lang::En,
            Language::Vi => Lang::Vi,
            Language::System => {
                let locale = sys_locale::get_locale()
                    .unwrap_or_default()
                    .to_ascii_lowercase();
                if locale.starts_with("vi") {
                    Lang::Vi
                } else {
                    Lang::En
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    TrayShow,
    TrayRandomize,
    TrayRestore,
    TrayRestoreAll,
    TrayCheckUpdates,
    TrayQuit,
    TrayNoAdapter,
    TrayDns,
    TrayDnsAutomatic,
    TrayDnsRestore,
    DnsLocalResolver,
    DnsCustomServers,
    NotifyChanged,
    NotifyRestored,
    NotifyFailed,
    NotifyRolledBack,
    NotifyStartupSummary,
    NotifyUpdate,
    NotifyStillRunning,
    NotifyDnsChanged,
    NotifyDnsRestored,
    NotifyDnsFailed,
    NotifyDnsRolledBack,
    NotifyDnsNothingToRestore,
    NotifyBusy,
    NotifyActionFailed,
    DialogExport,
    DialogImport,
}

/// Returns the template for `key`; `{0}`, `{1}` are replaced by [`t`].
fn template(lang: Lang, key: Key) -> &'static str {
    use Key::*;
    match (lang, key) {
        (Lang::En, TrayShow) => "Show RandomMac",
        (Lang::Vi, TrayShow) => "Mở RandomMac",
        (Lang::En, TrayRandomize) => "Randomize {0}",
        (Lang::Vi, TrayRandomize) => "Đổi MAC ngẫu nhiên: {0}",
        (Lang::En, TrayRestore) => "Restore {0}",
        (Lang::Vi, TrayRestore) => "Khôi phục: {0}",
        (Lang::En, TrayRestoreAll) => "Restore all adapters",
        (Lang::Vi, TrayRestoreAll) => "Khôi phục tất cả adapter",
        (Lang::En, TrayCheckUpdates) => "Check for updates",
        (Lang::Vi, TrayCheckUpdates) => "Kiểm tra cập nhật",
        (Lang::En, TrayQuit) => "Quit",
        (Lang::Vi, TrayQuit) => "Thoát",
        (Lang::En, TrayNoAdapter) => "No adapter selected",
        (Lang::Vi, TrayNoAdapter) => "Chưa chọn adapter",
        (Lang::En, NotifyChanged) => "{0}: MAC address changed to {1}",
        (Lang::Vi, NotifyChanged) => "{0}: đã đổi địa chỉ MAC thành {1}",
        (Lang::En, NotifyRestored) => "{0}: original MAC address restored",
        (Lang::Vi, NotifyRestored) => "{0}: đã khôi phục địa chỉ MAC gốc",
        (Lang::En, NotifyFailed) => "{0}: MAC change failed ({1})",
        (Lang::Vi, NotifyFailed) => "{0}: đổi MAC thất bại ({1})",
        (Lang::En, NotifyRolledBack) => {
            "{0}: the driver ignored the new address; nothing was changed"
        }
        (Lang::Vi, NotifyRolledBack) => "{0}: driver bỏ qua địa chỉ mới; không có gì thay đổi",
        (Lang::En, NotifyStartupSummary) => "Changed at startup: {0} succeeded, {1} failed",
        (Lang::Vi, NotifyStartupSummary) => "Đổi MAC khi khởi động: {0} thành công, {1} thất bại",
        (Lang::En, NotifyUpdate) => "RandomMac {0} is available",
        (Lang::Vi, NotifyUpdate) => "Đã có RandomMac {0}",
        (Lang::En, NotifyStillRunning) => "RandomMac keeps running in the notification area",
        (Lang::Vi, NotifyStillRunning) => "RandomMac vẫn chạy ở khay hệ thống",
        (Lang::En, TrayDns) => "DNS for {0}",
        (Lang::Vi, TrayDns) => "DNS cho {0}",
        (Lang::En, TrayDnsAutomatic) => "Automatic (from the network)",
        (Lang::Vi, TrayDnsAutomatic) => "Tự động (theo mạng)",
        (Lang::En, TrayDnsRestore) => "Restore original DNS",
        (Lang::Vi, TrayDnsRestore) => "Khôi phục DNS ban đầu",
        (Lang::En, DnsLocalResolver) => "the local resolver",
        (Lang::Vi, DnsLocalResolver) => "trình phân giải cục bộ",
        (Lang::En, DnsCustomServers) => "custom servers",
        (Lang::Vi, DnsCustomServers) => "máy chủ tùy chỉnh",
        (Lang::En, NotifyDnsChanged) => "{0}: DNS set to {1}",
        (Lang::Vi, NotifyDnsChanged) => "{0}: đã đặt DNS thành {1}",
        (Lang::En, NotifyDnsRestored) => "{0}: original DNS settings restored",
        (Lang::Vi, NotifyDnsRestored) => "{0}: đã khôi phục cài đặt DNS ban đầu",
        (Lang::En, NotifyDnsFailed) => "{0}: DNS change failed ({1})",
        (Lang::Vi, NotifyDnsFailed) => "{0}: đổi DNS thất bại ({1})",
        (Lang::En, NotifyDnsRolledBack) => {
            "{0}: the system did not keep the new DNS settings; nothing was changed"
        }
        (Lang::Vi, NotifyDnsRolledBack) => {
            "{0}: hệ thống không giữ cài đặt DNS mới; không có gì thay đổi"
        }
        (Lang::En, NotifyDnsNothingToRestore) => "{0}: no DNS change to undo",
        (Lang::Vi, NotifyDnsNothingToRestore) => "{0}: không có thay đổi DNS nào để hoàn tác",
        (Lang::En, NotifyBusy) => "Another change is in progress; try again in a moment",
        (Lang::Vi, NotifyBusy) => "Đang có một thay đổi khác; hãy thử lại sau giây lát",
        (Lang::En, NotifyActionFailed) => "The action could not be completed (error {0})",
        (Lang::Vi, NotifyActionFailed) => "Không thực hiện được thao tác (lỗi {0})",
        (Lang::En, DialogExport) => "Export",
        (Lang::Vi, DialogExport) => "Xuất",
        (Lang::En, DialogImport) => "Import",
        (Lang::Vi, DialogImport) => "Nhập",
    }
}

pub fn t(lang: Lang, key: Key, args: &[&str]) -> String {
    let mut s = template(lang, key).to_string();
    for (i, a) in args.iter().enumerate() {
        s = s.replace(&format!("{{{i}}}"), a);
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_both_languages() {
        assert_eq!(
            t(
                Lang::En,
                Key::NotifyChanged,
                &["Wi-Fi", "02:00:00:00:00:01"]
            ),
            "Wi-Fi: MAC address changed to 02:00:00:00:00:01"
        );
        assert!(t(Lang::Vi, Key::TrayRandomize, &["Ethernet"]).contains("Ethernet"));
        assert_eq!(Lang::resolve(Language::Vi), Lang::Vi);
        assert_eq!(Lang::resolve(Language::En), Lang::En);
        // The system language is either of the two.
        assert!(matches!(
            Lang::resolve(Language::System),
            Lang::En | Lang::Vi
        ));
    }

    #[test]
    fn every_key_is_translated_with_the_same_placeholders() {
        use Key::*;
        let keys = [
            TrayShow,
            TrayRandomize,
            TrayRestore,
            TrayRestoreAll,
            TrayCheckUpdates,
            TrayQuit,
            TrayNoAdapter,
            NotifyChanged,
            NotifyRestored,
            NotifyFailed,
            NotifyRolledBack,
            NotifyStartupSummary,
            NotifyUpdate,
            NotifyStillRunning,
            TrayDns,
            TrayDnsAutomatic,
            TrayDnsRestore,
            DnsLocalResolver,
            DnsCustomServers,
            NotifyDnsChanged,
            NotifyDnsRestored,
            NotifyDnsFailed,
            NotifyDnsRolledBack,
            NotifyDnsNothingToRestore,
            NotifyBusy,
            NotifyActionFailed,
            DialogExport,
            DialogImport,
        ];
        let placeholders = |s: &str| (0..3).filter(|i| s.contains(&format!("{{{i}}}"))).count();
        for key in keys {
            let (en, vi) = (template(Lang::En, key), template(Lang::Vi, key));
            assert!(!en.is_empty() && !vi.is_empty(), "{key:?}");
            assert_ne!(en, vi, "{key:?} is not translated");
            assert_eq!(placeholders(en), placeholders(vi), "{key:?}");
            // Every placeholder is replaced when arguments are given.
            assert!(!t(Lang::Vi, key, &["a", "b"]).contains('{'), "{key:?}");
        }
    }
}
