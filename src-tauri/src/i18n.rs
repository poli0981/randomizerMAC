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
    NotifyChanged,
    NotifyRestored,
    NotifyFailed,
    NotifyRolledBack,
    NotifyStartupSummary,
    NotifyUpdate,
    NotifyStillRunning,
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
    }
}
