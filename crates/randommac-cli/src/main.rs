//! `randommac-cli` — scriptable access to RandomMac.
//!
//! Exit codes: 0 success · 1 error · 2 usage · 3 insufficient privileges / authentication ·
//! 4 adapter not found · 5 change failed or was rolled back · 6 invalid or blacklisted address.

use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand, ValueEnum};
use randommac_core::settings::LinuxMethod;
use randommac_core::{
    AppPaths, ChangeSource, GenerateRequest, GenerationMode, MacAddress, OsRandom, generate, oui,
};
use randommac_net::ops::{self, DataStore, OpError, Outcome};
use randommac_net::{
    Adapter, NetError, NetworkBackend, Stage, mock::MockBackend, resolve_adapter, system_backend,
};
use serde::Serialize;

#[derive(Parser)]
#[command(
    name = "randommac-cli",
    bin_name = "randommac-cli",
    version,
    about = "Randomize, set and restore network adapter MAC addresses (RandomMac CLI).",
    after_help = "Adapters can be referred to by name (\"Wi-Fi\", wlp2s0), id or current MAC.\n\
                  Changing addresses needs an elevated terminal on Windows; on Linux it uses\n\
                  NetworkManager or `pkexec ip` (or run with sudo)."
)]
struct Cli {
    /// Print machine-readable JSON instead of text.
    #[arg(long, global = true)]
    json: bool,

    /// Use the in-memory demo backend; no real adapter is touched.
    #[arg(long, global = true)]
    mock: bool,

    /// Keep RandomMac data (history, originals, settings) in this directory.
    #[arg(long, global = true, value_name = "DIR")]
    data_dir: Option<PathBuf>,

    /// Linux only: how to apply changes.
    #[arg(long, global = true, value_enum)]
    method: Option<MethodArg>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Clone, Copy, ValueEnum)]
enum MethodArg {
    Auto,
    #[value(name = "networkmanager", alias = "nm")]
    NetworkManager,
    Iproute2,
}

#[derive(Clone, Copy, Default, ValueEnum)]
enum ModeArg {
    /// Fully random, locally administered (works with every driver).
    #[default]
    Random,
    /// Keep the adapter's vendor prefix.
    KeepVendor,
    /// Use a prefix of the vendor given with --vendor.
    Vendor,
}

#[derive(Subcommand)]
enum Command {
    /// List network adapters (physical ones unless --all).
    List {
        /// Include virtual adapters (VPN, Hyper-V, VirtualBox, bridges, ...).
        #[arg(long)]
        all: bool,
    },
    /// Show details of one adapter.
    Show {
        /// Adapter name, id or current MAC.
        adapter: String,
    },
    /// Assign a random MAC address.
    Random {
        /// Adapter name, id or current MAC.
        adapter: String,
        /// How the new address is generated.
        #[arg(long, value_enum, default_value_t)]
        mode: ModeArg,
        /// Vendor id for --mode vendor (see `randommac-cli vendors`).
        #[arg(long)]
        vendor: Option<String>,
        /// Only print the address that would be used.
        #[arg(long)]
        dry_run: bool,
        /// Apply even if the address matches a blacklist rule.
        #[arg(long)]
        ignore_blacklist: bool,
    },
    /// Assign a specific MAC address.
    Set {
        /// Adapter name, id or current MAC.
        adapter: String,
        /// New address (AA:BB:CC:DD:EE:FF, AA-BB-CC-DD-EE-FF, AABBCCDDEEFF or AABB.CCDD.EEFF).
        mac: String,
        /// Only validate the address; change nothing.
        #[arg(long)]
        dry_run: bool,
        /// Apply even if the address matches a blacklist rule.
        #[arg(long)]
        ignore_blacklist: bool,
    },
    /// Undo RandomMac's changes and return to the original address.
    Restore {
        /// Adapter name, id or current MAC.
        adapter: String,
    },
    /// Show how a MAC address is interpreted.
    Inspect {
        /// Address in any supported notation.
        mac: String,
    },
    /// List vendors known to the OUI table.
    Vendors,
    /// Show recent changes.
    History {
        /// Maximum number of entries to show.
        #[arg(long, default_value_t = 20)]
        limit: usize,
        /// Only entries for this adapter (name or id).
        #[arg(long)]
        adapter: Option<String>,
    },
}

struct Ctx {
    json: bool,
    backend: Box<dyn NetworkBackend>,
    store: DataStore,
    method: Option<LinuxMethod>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AdapterView<'a> {
    #[serde(flatten)]
    adapter: &'a Adapter,
    vendor: Option<&'static str>,
    original_mac: Option<MacAddress>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Inspection {
    input: String,
    mac: MacAddress,
    vendor: Option<&'static str>,
    unicast: bool,
    locally_administered: bool,
    assignable: bool,
    windows_wifi_compatible: bool,
}

fn fail(code: u8, msg: impl std::fmt::Display) -> anyhow::Error {
    anyhow::Error::msg(msg.to_string()).context(ExitCodeTag(code))
}

#[derive(Debug, Clone, Copy)]
struct ExitCodeTag(u8);

impl std::fmt::Display for ExitCodeTag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "exit {}", self.0)
    }
}

fn net_exit(e: &NetError) -> u8 {
    match e {
        NetError::NotElevated
        | NetError::PermissionDenied(_)
        | NetError::AuthCancelled
        | NetError::AuthUnavailable(_) => 3,
        NetError::NotFound(_) => 4,
        NetError::InvalidInput(_) => 6,
        _ => 5,
    }
}

fn op_exit(e: &OpError) -> u8 {
    match e {
        OpError::Net(n) => net_exit(n),
        OpError::NotAssignable(_) | OpError::Blacklisted { .. } => 6,
        OpError::Busy | OpError::Storage(_) => 1,
    }
}

fn hint(e: &NetError) -> Option<&'static str> {
    match e {
        NetError::NotElevated => {
            Some("run the command from an elevated terminal (Run as administrator)")
        }
        NetError::AuthUnavailable(_) => {
            Some("no polkit agent is available; run the command with sudo")
        }
        NetError::NmNoActiveConnection => {
            Some("connect the adapter first, or use --method iproute2")
        }
        _ => None,
    }
}

impl Ctx {
    fn print<T: Serialize>(&self, value: &T, text: impl FnOnce() -> String) -> anyhow::Result<()> {
        let mut out = std::io::stdout().lock();
        if self.json {
            serde_json::to_writer_pretty(&mut out, value)?;
            writeln!(out)?;
        } else {
            writeln!(out, "{}", text())?;
        }
        Ok(())
    }

    fn adapters(&self) -> anyhow::Result<Vec<Adapter>> {
        self.backend
            .list_adapters()
            .map_err(|e| fail(net_exit(&e), e))
    }

    fn find(&self, query: &str) -> anyhow::Result<Adapter> {
        let all = self.adapters()?;
        resolve_adapter(&all, query).cloned().ok_or_else(|| {
            fail(
                4,
                format!("no unique adapter matches {query:?} (try `randommac-cli list --all`)"),
            )
        })
    }

    fn settings(&self) -> randommac_core::Settings {
        let (mut s, _) = self.store.load_settings();
        if let Some(m) = self.method {
            s.linux.method = m;
        }
        s
    }

    fn progress(&self) -> impl FnMut(Stage) + use<> {
        let quiet = self.json;
        move |stage| {
            if !quiet {
                let label = match stage {
                    Stage::Preparing => "preparing",
                    Stage::Writing => "writing the new address",
                    Stage::Restarting => "restarting the adapter",
                    Stage::Verifying => "verifying",
                    Stage::RollingBack => "rolling back",
                };
                eprintln!("... {label}");
            }
        }
    }

    fn report_outcome(&self, outcome: Outcome, verb: &str) -> anyhow::Result<()> {
        match &outcome.report {
            Ok(report) => {
                self.print(&outcome.entry, || {
                    let mac = report
                        .observed_mac
                        .or(report.requested_mac)
                        .map(|m| m.to_string())
                        .unwrap_or_else(|| "?".into());
                    let mut line = if report.status.is_success() {
                        format!(
                            "{verb} {} -> {mac} ({:?} via {})",
                            outcome.adapter.name,
                            report.status,
                            report.method.as_str()
                        )
                    } else {
                        format!(
                            "{} was not changed ({:?} via {}); it reports {mac}",
                            outcome.adapter.name,
                            report.status,
                            report.method.as_str()
                        )
                    };
                    if report.temporary {
                        line.push_str("\nnote: this change lasts until the next reboot");
                    }
                    if let Some(code) = &report.detail_code {
                        line.push_str(&format!("\ndetail: {code}"));
                    }
                    line
                })?;
                if report.status.is_success() {
                    Ok(())
                } else {
                    Err(fail(
                        5,
                        format!("the change did not take effect ({:?})", report.status),
                    ))
                }
            }
            Err(e) => {
                if self.json {
                    self.print(&outcome.entry, String::new)?;
                }
                let mut msg = e.to_string();
                if let Some(h) = hint(e) {
                    msg.push_str(&format!("\nhint: {h}"));
                }
                Err(fail(net_exit(e), msg))
            }
        }
    }
}

fn run(cli: Cli) -> anyhow::Result<()> {
    let paths = match &cli.data_dir {
        Some(dir) => AppPaths::under(dir),
        None => AppPaths::resolve()?,
    };
    let ctx = Ctx {
        json: cli.json,
        backend: if cli.mock {
            Box::new(MockBackend::from_env())
        } else {
            system_backend()
        },
        store: DataStore::new(paths),
        method: cli.method.map(|m| match m {
            MethodArg::Auto => LinuxMethod::Auto,
            MethodArg::NetworkManager => LinuxMethod::NetworkManager,
            MethodArg::Iproute2 => LinuxMethod::Iproute2,
        }),
    };

    match cli.command {
        Command::List { all } => {
            let state = ctx.store.load_state().0;
            let adapters: Vec<Adapter> = ctx
                .adapters()?
                .into_iter()
                .filter(|a| all || !a.is_virtual)
                .collect();
            let views: Vec<AdapterView> = adapters
                .iter()
                .map(|a| AdapterView {
                    adapter: a,
                    vendor: a
                        .current_mac
                        .and_then(|m| oui::lookup(m.oui()))
                        .map(|v| v.name),
                    original_mac: ops::original_mac(a, &state),
                })
                .collect();
            ctx.print(&views, || {
                let mut s = format!(
                    "{:<28} {:<9} {:<9} {:<18} {:<18}\n",
                    "NAME", "KIND", "STATE", "CURRENT MAC", "ORIGINAL MAC"
                );
                for v in &views {
                    let a = v.adapter;
                    let mac = |m: Option<MacAddress>| {
                        m.map(|m| m.to_string()).unwrap_or_else(|| "-".into())
                    };
                    let name: String = a.name.chars().take(27).collect();
                    s.push_str(&format!(
                        "{:<28} {:<9} {:<9} {:<18} {:<18}{}\n",
                        name,
                        format!("{:?}", a.kind).to_lowercase(),
                        format!("{:?}", a.state).to_lowercase(),
                        mac(a.current_mac),
                        mac(v.original_mac),
                        if a.is_virtual { "  (virtual)" } else { "" }
                    ));
                }
                if views.is_empty() {
                    s.push_str("(no adapters)\n");
                }
                s.trim_end().to_string()
            })
        }
        Command::Show { adapter } => {
            let a = ctx.find(&adapter)?;
            let state = ctx.store.load_state().0;
            let view = AdapterView {
                adapter: &a,
                vendor: a
                    .current_mac
                    .and_then(|m| oui::lookup(m.oui()))
                    .map(|v| v.name),
                original_mac: ops::original_mac(&a, &state),
            };
            ctx.print(&view, || {
                let mac = |m: Option<MacAddress>| m.map(|m| m.to_string()).unwrap_or_else(|| "-".into());
                format!(
                    "Name:          {}\nDescription:   {}\nId:            {}\nKind:          {:?}{}\nState:         {:?}{}\nCurrent MAC:   {}{}\nOriginal MAC:  {}\nMethods:       {}\nIPv4:          {}\nIPv6:          {}",
                    a.name,
                    a.description,
                    a.id,
                    a.kind,
                    if a.is_virtual { " (virtual)" } else { "" },
                    a.state,
                    if a.connected { ", connected" } else { "" },
                    mac(a.current_mac),
                    view.vendor.map(|v| format!(" ({v})")).unwrap_or_default(),
                    mac(view.original_mac),
                    a.methods.iter().map(|m| m.as_str()).collect::<Vec<_>>().join(", "),
                    a.ipv4.join(", "),
                    a.ipv6.join(", "),
                )
            })
        }
        Command::Random {
            adapter,
            mode,
            vendor,
            dry_run,
            ignore_blacklist,
        } => {
            let a = ctx.find(&adapter)?;
            let mode = match mode {
                ModeArg::Random => GenerationMode::Random,
                ModeArg::KeepVendor => GenerationMode::KeepVendor,
                ModeArg::Vendor => GenerationMode::Vendor {
                    vendor_id: vendor
                        .ok_or_else(|| fail(2, "--mode vendor needs --vendor <ID>"))?,
                },
            };
            let (blacklist, _) = ctx.store.load_blacklist();
            let avoid: Vec<MacAddress> = a.current_mac.into_iter().collect();
            let mac = generate(
                &mut OsRandom,
                &GenerateRequest {
                    mode,
                    base: a.permanent_mac.or(a.current_mac),
                    avoid: &avoid,
                    blacklist: (!ignore_blacklist).then_some(&blacklist),
                    adapter_id: Some(&a.id),
                },
            )
            .map_err(|e| fail(6, e))?;
            if dry_run {
                return ctx.print(&mac, || mac.to_string());
            }
            let settings = ctx.settings();
            let mut progress = ctx.progress();
            let outcome = ops::apply_mac(
                ctx.backend.as_ref(),
                &ctx.store,
                &settings,
                &a.id,
                mac,
                ChangeSource::Cli,
                ignore_blacklist,
                &mut progress,
            )
            .map_err(|e| fail(op_exit(&e), e))?;
            ctx.report_outcome(outcome, "changed")
        }
        Command::Set {
            adapter,
            mac,
            dry_run,
            ignore_blacklist,
        } => {
            let mac: MacAddress = mac.parse().map_err(|e| fail(6, format!("{mac:?}: {e}")))?;
            let a = ctx.find(&adapter)?;
            if dry_run {
                ops::validate(&ctx.store, &a.id, mac, ignore_blacklist)
                    .map_err(|e| fail(op_exit(&e), e))?;
                return ctx.print(&mac, || format!("{mac} can be applied to {}", a.name));
            }
            let settings = ctx.settings();
            let mut progress = ctx.progress();
            let outcome = ops::apply_mac(
                ctx.backend.as_ref(),
                &ctx.store,
                &settings,
                &a.id,
                mac,
                ChangeSource::Cli,
                ignore_blacklist,
                &mut progress,
            )
            .map_err(|e| fail(op_exit(&e), e))?;
            ctx.report_outcome(outcome, "changed")
        }
        Command::Restore { adapter } => {
            let a = ctx.find(&adapter)?;
            let settings = ctx.settings();
            let mut progress = ctx.progress();
            let outcome = ops::restore_mac(
                ctx.backend.as_ref(),
                &ctx.store,
                &settings,
                &a.id,
                ChangeSource::Cli,
                &mut progress,
            )
            .map_err(|e| fail(op_exit(&e), e))?;
            ctx.report_outcome(outcome, "restored")
        }
        Command::Inspect { mac } => {
            let parsed: MacAddress = mac.parse().map_err(|e| fail(6, format!("{mac:?}: {e}")))?;
            let info = Inspection {
                input: mac.clone(),
                mac: parsed,
                vendor: oui::lookup(parsed.oui()).map(|v| v.name),
                unicast: parsed.is_unicast(),
                locally_administered: parsed.is_locally_administered(),
                assignable: parsed.is_assignable(),
                windows_wifi_compatible: parsed.is_windows_wifi_compatible(),
            };
            ctx.print(&info, || {
                format!(
                    "{}\nvendor:                {}\nunicast:               {}\nlocally administered:  {}\nassignable:            {}\nWindows Wi-Fi ready:   {}",
                    info.mac,
                    info.vendor.unwrap_or("unknown"),
                    info.unicast,
                    info.locally_administered,
                    info.assignable,
                    info.windows_wifi_compatible
                )
            })
        }
        Command::Vendors => {
            #[derive(Serialize)]
            #[serde(rename_all = "camelCase")]
            struct V {
                id: &'static str,
                name: &'static str,
                imitable: bool,
                oui_count: u32,
            }
            let list: Vec<V> = oui::vendors()
                .iter()
                .map(|v| V {
                    id: v.id,
                    name: v.name,
                    imitable: v.imitable,
                    oui_count: v.oui_count,
                })
                .collect();
            ctx.print(&list, || {
                list.iter()
                    .map(|v| format!("{:<14} {:<22} {:>5} prefixes", v.id, v.name, v.oui_count))
                    .collect::<Vec<_>>()
                    .join("\n")
            })
        }
        Command::History { limit, adapter } => {
            let (history, _) = ctx.store.load_history();
            let filter_id = adapter
                .as_ref()
                .map(|q| ctx.find(q).map(|a| a.id).unwrap_or_else(|_| q.clone()));
            let entries: Vec<_> = history
                .search("", filter_id.as_deref())
                .take(limit)
                .cloned()
                .collect();
            ctx.print(&entries, || {
                if entries.is_empty() {
                    return "(no history)".into();
                }
                entries
                    .iter()
                    .map(|e| {
                        let mac = |m: Option<MacAddress>| {
                            m.map(|m| m.to_string()).unwrap_or_else(|| "-".into())
                        };
                        format!(
                            "{}  {:<20} {:<8} {} -> {}  {:?}",
                            e.timestamp.format("%Y-%m-%d %H:%M:%S"),
                            e.adapter_name.chars().take(20).collect::<String>(),
                            format!("{:?}", e.action).to_lowercase(),
                            mac(e.previous_mac),
                            mac(e.new_mac.or(e.requested_mac)),
                            e.status
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            })
        }
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            let code = e.downcast_ref::<ExitCodeTag>().map(|t| t.0).unwrap_or(1);
            eprintln!("error: {}", e.root_cause());
            ExitCode::from(code)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn cli_definition_is_valid() {
        Cli::command().debug_assert();
    }

    #[test]
    fn exit_codes_follow_the_documented_table() {
        let auth = [
            NetError::NotElevated,
            NetError::PermissionDenied("x".into()),
            NetError::AuthCancelled,
            NetError::AuthUnavailable("x".into()),
        ];
        assert!(auth.iter().all(|e| net_exit(e) == 3));
        assert_eq!(net_exit(&NetError::NotFound("x".into())), 4);
        assert_eq!(net_exit(&NetError::InvalidInput("x".into())), 6);
        assert_eq!(net_exit(&NetError::VerifyTimeout), 5);
        assert_eq!(net_exit(&NetError::NmNoActiveConnection), 5);

        assert_eq!(op_exit(&OpError::Net(NetError::AuthCancelled)), 3);
        assert_eq!(op_exit(&OpError::Busy), 1);
        assert_eq!(op_exit(&OpError::Storage("disk".into())), 1);
        assert_eq!(ExitCodeTag(4).to_string(), "exit 4");
    }

    #[test]
    fn hints_for_fixable_errors() {
        assert!(hint(&NetError::NotElevated).is_some_and(|h| h.contains("administrator")));
        assert!(hint(&NetError::AuthUnavailable("x".into())).is_some_and(|h| h.contains("sudo")));
        assert!(
            hint(&NetError::NmNoActiveConnection).is_some_and(|h| h.contains("--method iproute2"))
        );
        assert_eq!(hint(&NetError::VerifyTimeout), None);
    }

    #[test]
    fn exit_codes_survive_the_error_chain() {
        let e = fail(6, "bad address");
        assert_eq!(e.downcast_ref::<ExitCodeTag>().map(|t| t.0), Some(6));
        assert_eq!(e.root_cause().to_string(), "bad address");
    }
}
