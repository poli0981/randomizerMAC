//! Platform-independent core of RandomMac.
//!
//! Everything here is pure logic plus file persistence: MAC parsing/formatting, random address
//! generation, the vendor (OUI) table, blacklist, history, settings, crash-safe storage and the
//! RandomMac 1.x import. Operating-system specific code lives in `randommac-net`.

pub mod blacklist;
pub mod generator;
pub mod history;
pub mod legacy_v1;
pub mod lock;
pub mod mac;
pub mod oui;
pub mod paths;
pub mod random;
pub mod settings;
pub mod state;
pub mod storage;

pub use blacklist::{Blacklist, BlacklistEntry, BlockReason, MacPattern};
pub use generator::{GenerateError, GenerateRequest, GenerationMode, generate};
pub use history::{ChangeSource, ChangeStatus, History, HistoryAction, HistoryEntry};
pub use mac::{MacAddress, MacFormat, ParseMacError, Separator};
pub use paths::{APP_ID, AppPaths};
pub use random::{OsRandom, RandomSource};
pub use settings::Settings;
pub use state::{AppState, OriginalRecord, Remembered};

/// Version of the RandomMac workspace (all crates share it).
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
