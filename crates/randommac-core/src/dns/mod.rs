//! DNS settings per adapter: the configuration model and its validation, the catalog of public
//! providers, custom presets, the snapshots behind "Restore original DNS", and the wire format of
//! the latency test. Nothing here talks to the operating system.

pub mod catalog;
pub mod model;
pub mod presets;
pub mod snapshot;
pub mod wire;

pub use catalog::{Catalog, DnscryptStamp, Provider, Variant, VariantLabel, catalog};
pub use model::{
    DnsConfig, DnsEncryption, DnsEndpoints, DnsInputError, DnsLabel, DnsRequest, DnsSummary,
    DnsTarget, EncryptionChoice, EncryptionMode, EncryptionSupport, FamilyDns, LOCAL_RESOLVER_V4,
    LOCAL_RESOLVER_V6, MAX_SERVERS_PER_FAMILY, ResolvedDns, resolve_request, validate_config,
};
pub use presets::CustomDnsPreset;
pub use snapshot::{
    DnsOriginalRecord, DnsRemembered, DnsSnapshot, DnsState, DohServerSnapshot, NmDnsProfile,
    WinFamilySnapshot,
};
