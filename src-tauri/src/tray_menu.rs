//! The tray's DNS submenu as plain data, so it is tested without a tray: the items it shows, what
//! each item asks for, and how a DNS configuration is named in notifications.

use randommac_core::dns::{
    Catalog, CustomDnsPreset, DnsLabel, DnsRequest, DnsTarget, EncryptionChoice,
};
use randommac_core::settings::DnsSettings;

use crate::i18n::{Key, Lang, t};

const PREFIX: &str = "dns:";
pub const AUTOMATIC_ID: &str = "dns:auto";
pub const RESTORE_ID: &str = "dns:restore";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MenuEntry {
    Item { id: String, label: String },
    Separator,
}

/// What a DNS menu item does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DnsMenuAction {
    Apply(DnsRequest),
    Restore,
}

/// The submenu: every catalog provider (its default variant), the user's presets, then
/// "Automatic" and "Restore original DNS".
pub fn dns_menu(catalog: &Catalog, presets: &[CustomDnsPreset], lang: Lang) -> Vec<MenuEntry> {
    let item = |id: String, label: String| MenuEntry::Item { id, label };
    let mut entries: Vec<MenuEntry> = catalog
        .providers
        .iter()
        .map(|p| item(format!("{PREFIX}provider:{}", p.id), p.name.clone()))
        .collect();
    if !presets.is_empty() {
        entries.push(MenuEntry::Separator);
        entries.extend(
            presets
                .iter()
                .map(|p| item(format!("{PREFIX}preset:{}", p.id), p.name.clone())),
        );
    }
    entries.push(MenuEntry::Separator);
    entries.push(item(
        AUTOMATIC_ID.to_string(),
        t(lang, Key::TrayDnsAutomatic, &[]),
    ));
    entries.push(item(
        RESTORE_ID.to_string(),
        t(lang, Key::TrayDnsRestore, &[]),
    ));
    entries
}

/// What the menu item `id` asks for; `None` when it is not a DNS item. Providers and presets are
/// encrypted when the settings prefer it and the adapter supports it.
pub fn dns_action(id: &str, settings: &DnsSettings) -> Option<DnsMenuAction> {
    let rest = id.strip_prefix(PREFIX)?;
    let encryption = if settings.prefer_encryption {
        EncryptionChoice::Preferred
    } else {
        EncryptionChoice::None
    };
    let apply = |target: DnsTarget| {
        Some(DnsMenuAction::Apply(DnsRequest {
            target,
            encryption,
            allow_fallback: settings.allow_fallback,
        }))
    };
    match rest.split_once(':') {
        Some(("provider", provider)) if !provider.is_empty() => apply(DnsTarget::Provider {
            provider: provider.to_string(),
            variant: None,
        }),
        Some(("preset", preset)) if !preset.is_empty() => apply(DnsTarget::Preset {
            id: preset.to_string(),
        }),
        None if rest == "auto" => Some(DnsMenuAction::Apply(DnsRequest {
            target: DnsTarget::Automatic,
            encryption: EncryptionChoice::None,
            allow_fallback: false,
        })),
        None if rest == "restore" => Some(DnsMenuAction::Restore),
        _ => None,
    }
}

/// How a configuration is named in a notification: the provider's or preset's name, or what it is.
pub fn label_text(lang: Lang, label: Option<&DnsLabel>, catalog: &Catalog) -> String {
    match label {
        Some(DnsLabel::Automatic) => t(lang, Key::TrayDnsAutomatic, &[]),
        Some(DnsLabel::LocalResolver) => t(lang, Key::DnsLocalResolver, &[]),
        Some(DnsLabel::Provider { provider, .. }) => catalog
            .provider(provider)
            .map_or_else(|| provider.clone(), |p| p.name.clone()),
        Some(DnsLabel::Preset { name }) => name.clone(),
        Some(DnsLabel::Custom) | None => t(lang, Key::DnsCustomServers, &[]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use randommac_core::dns::catalog;

    fn preset(id: &str, name: &str) -> CustomDnsPreset {
        CustomDnsPreset {
            id: id.into(),
            name: name.into(),
            ipv4: vec!["192.0.2.53".parse().expect("ip")],
            ipv6: Vec::new(),
            doh_template: None,
            dot_host: None,
        }
    }

    fn ids(entries: &[MenuEntry]) -> Vec<&str> {
        entries
            .iter()
            .map(|e| match e {
                MenuEntry::Item { id, .. } => id.as_str(),
                MenuEntry::Separator => "-",
            })
            .collect()
    }

    #[test]
    fn lists_providers_presets_and_the_fixed_items() {
        let c = catalog();
        let plain = dns_menu(c, &[], Lang::En);
        let plain_ids = ids(&plain);
        assert_eq!(plain_ids.len(), c.providers.len() + 3);
        assert_eq!(plain_ids[0], format!("dns:provider:{}", c.providers[0].id));
        assert_eq!(
            plain_ids[plain_ids.len() - 3..],
            ["-", AUTOMATIC_ID, RESTORE_ID]
        );
        assert!(plain.contains(&MenuEntry::Item {
            id: RESTORE_ID.into(),
            label: "Restore original DNS".into(),
        }));

        let with_presets = dns_menu(c, &[preset("0123456789abcdef", "Lab")], Lang::Vi);
        let preset_ids = ids(&with_presets);
        let at = c.providers.len();
        assert_eq!(preset_ids[at..at + 2], ["-", "dns:preset:0123456789abcdef"]);
        assert!(with_presets.contains(&MenuEntry::Item {
            id: "dns:preset:0123456789abcdef".into(),
            label: "Lab".into(),
        }));
        assert!(with_presets.contains(&MenuEntry::Item {
            id: AUTOMATIC_ID.into(),
            label: t(Lang::Vi, Key::TrayDnsAutomatic, &[]),
        }));
    }

    #[test]
    fn menu_ids_become_requests() {
        let mut settings = DnsSettings {
            prefer_encryption: true,
            allow_fallback: true,
            ..DnsSettings::default()
        };
        assert_eq!(
            dns_action("dns:provider:quad9", &settings),
            Some(DnsMenuAction::Apply(DnsRequest {
                target: DnsTarget::Provider {
                    provider: "quad9".into(),
                    variant: None,
                },
                encryption: EncryptionChoice::Preferred,
                allow_fallback: true,
            }))
        );
        settings.prefer_encryption = false;
        settings.allow_fallback = false;
        assert_eq!(
            dns_action("dns:preset:0123456789abcdef", &settings),
            Some(DnsMenuAction::Apply(DnsRequest {
                target: DnsTarget::Preset {
                    id: "0123456789abcdef".into(),
                },
                encryption: EncryptionChoice::None,
                allow_fallback: false,
            }))
        );
        assert_eq!(
            dns_action(AUTOMATIC_ID, &settings),
            Some(DnsMenuAction::Apply(DnsRequest {
                target: DnsTarget::Automatic,
                encryption: EncryptionChoice::None,
                allow_fallback: false,
            }))
        );
        assert_eq!(
            dns_action(RESTORE_ID, &settings),
            Some(DnsMenuAction::Restore)
        );
        for other in [
            "randomize",
            "dns:",
            "dns:provider:",
            "dns:preset:",
            "dns:nope",
            "dns:x:y",
        ] {
            assert_eq!(dns_action(other, &settings), None, "{other}");
        }
    }

    #[test]
    fn names_configurations() {
        let c = catalog();
        let quad9 = DnsLabel::Provider {
            provider: "quad9".into(),
            variant: "standard".into(),
        };
        assert_eq!(label_text(Lang::En, Some(&quad9), c), "Quad9");
        let unknown = DnsLabel::Provider {
            provider: "gone".into(),
            variant: "x".into(),
        };
        assert_eq!(label_text(Lang::En, Some(&unknown), c), "gone");
        assert_eq!(
            label_text(Lang::En, Some(&DnsLabel::Preset { name: "Lab".into() }), c),
            "Lab"
        );
        assert_eq!(
            label_text(Lang::Vi, Some(&DnsLabel::Automatic), c),
            t(Lang::Vi, Key::TrayDnsAutomatic, &[])
        );
        assert_eq!(
            label_text(Lang::En, Some(&DnsLabel::LocalResolver), c),
            t(Lang::En, Key::DnsLocalResolver, &[])
        );
        assert_eq!(
            label_text(Lang::En, None, c),
            t(Lang::En, Key::DnsCustomServers, &[])
        );
        assert_eq!(
            label_text(Lang::En, Some(&DnsLabel::Custom), c),
            t(Lang::En, Key::DnsCustomServers, &[])
        );
    }
}
