//! `randommac-cli dns …`: DNS servers per adapter.

use std::net::IpAddr;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

use clap::{Args, Subcommand, ValueEnum};
use randommac_core::dns::{
    self, DnsEndpoints, DnsLabel, DnsRequest, DnsTarget, EncryptionChoice, catalog,
};
use randommac_core::{ChangeSource, HistoryEntry};
use randommac_net::dns_ops::{self, DnsOutcome};
use randommac_net::dns_probe::{self, ProbeOptions, ProbeOutcome, ProbeTarget};
use randommac_net::{DnsOptions, Stage};
use serde::Serialize;

use crate::{Ctx, fail, net_exit, op_exit};

#[derive(Subcommand)]
pub(crate) enum DnsCommand {
    /// List the DNS providers RandomMac knows, with every endpoint (IPv4, IPv6, DoH, DoT and the
    /// DNSCrypt stamps for a DNSCrypt client).
    Providers {
        /// Only this provider, with all details.
        #[arg(long)]
        provider: Option<String>,
    },
    /// Show an adapter's DNS settings and what can be changed.
    Show {
        /// Adapter name, id or current MAC.
        adapter: String,
    },
    /// Set the DNS servers of one adapter, or of every connected physical adapter (--all).
    Set(SetArgs),
    /// Use the DNS servers from the network again (DHCP).
    Auto(TargetArgs),
    /// Put back the DNS settings from before RandomMac's first DNS change.
    Restore(TargetArgs),
    /// Measure how fast DNS servers answer from this computer (plain DNS over UDP).
    Test(TestArgs),
    /// List your custom DNS presets (made in the app's DNS page).
    Presets,
}

#[derive(Args)]
#[group(required = true, multiple = false)]
pub(crate) struct TargetArgs {
    /// Adapter name, id or current MAC.
    adapter: Option<String>,
    /// Every connected physical adapter.
    #[arg(long)]
    all: bool,
}

#[derive(Args)]
#[group(required = true, multiple = false)]
pub(crate) struct ServerArgs {
    /// A provider from `dns providers`, optionally with a variant: `quad9`, `quad9:ecs`.
    #[arg(long, value_name = "ID[:VARIANT]")]
    provider: Option<String>,
    /// One of your custom presets (name or id).
    #[arg(long)]
    preset: Option<String>,
    /// 127.0.0.1 and ::1: a DNS client such as dnscrypt-proxy on this computer.
    #[arg(long)]
    local: bool,
    /// Your own servers, comma-separated (up to two IPv4 and two IPv6 addresses).
    #[arg(long, value_delimiter = ',', value_name = "IP")]
    servers: Vec<String>,
}

#[derive(Clone, Copy, Default, ValueEnum)]
pub(crate) enum EncryptionArg {
    /// DNS over HTTPS where supported, else DNS over TLS, else none.
    #[default]
    Auto,
    None,
    Doh,
    Dot,
}

#[derive(Args)]
pub(crate) struct SetArgs {
    #[command(flatten)]
    target: TargetArgs,
    #[command(flatten)]
    servers: ServerArgs,
    /// How DNS traffic is protected.
    #[arg(long, value_enum, default_value_t)]
    encryption: EncryptionArg,
    /// DNS-over-HTTPS template for --servers (https://host/path).
    #[arg(long)]
    doh_template: Option<String>,
    /// DNS-over-TLS server name for --servers.
    #[arg(long)]
    dot_host: Option<String>,
    /// Let encrypted DNS fall back to unencrypted queries when the server cannot be reached.
    #[arg(long)]
    allow_fallback: bool,
    /// Only print the configuration that would be applied.
    #[arg(long)]
    dry_run: bool,
    /// Switch to plain DNS servers even if none of them answers from this computer.
    #[arg(long)]
    no_check: bool,
}

#[derive(Args)]
pub(crate) struct TestArgs {
    /// Providers to test (`ID[:VARIANT]`); default: every provider's default variant.
    #[arg(long = "provider", value_name = "ID[:VARIANT]")]
    providers: Vec<String>,
    /// Servers to test.
    #[arg(long = "server", value_name = "IP")]
    servers: Vec<String>,
    /// Also test the providers' IPv6 servers.
    #[arg(long)]
    ipv6: bool,
    /// Timed queries per server.
    #[arg(long, default_value_t = 3)]
    attempts: u8,
    /// How long to wait for each answer.
    #[arg(long, default_value_t = 1500)]
    timeout_ms: u64,
    /// Query this port instead of 53 (tests).
    #[arg(long, default_value_t = 53, hide = true)]
    port: u16,
}

fn parse_provider(spec: &str) -> DnsTarget {
    let (provider, variant) = match spec.split_once(':') {
        Some((p, v)) => (p.to_string(), Some(v.to_string())),
        None => (spec.to_string(), None),
    };
    DnsTarget::Provider { provider, variant }
}

fn parse_ips(list: &[String]) -> anyhow::Result<Vec<IpAddr>> {
    list.iter()
        .filter(|s| !s.trim().is_empty())
        .map(|s| {
            s.trim()
                .parse()
                .map_err(|_| fail(6, format!("{s:?} is not an IP address")))
        })
        .collect()
}

fn request(args: &SetArgs) -> anyhow::Result<DnsRequest> {
    let s = &args.servers;
    let target = if let Some(p) = &s.provider {
        parse_provider(p)
    } else if let Some(name) = &s.preset {
        DnsTarget::Preset { id: name.clone() }
    } else if s.local {
        DnsTarget::LocalResolver
    } else {
        let ips = parse_ips(&s.servers)?;
        DnsTarget::Custom(DnsEndpoints {
            ipv4: ips
                .iter()
                .filter_map(|ip| match ip {
                    IpAddr::V4(a) => Some(*a),
                    IpAddr::V6(_) => None,
                })
                .collect(),
            ipv6: ips
                .iter()
                .filter_map(|ip| match ip {
                    IpAddr::V6(a) => Some(*a),
                    IpAddr::V4(_) => None,
                })
                .collect(),
            doh: args.doh_template.clone(),
            dot: args.dot_host.clone(),
        })
    };
    let encryption = match args.encryption {
        EncryptionArg::Auto => EncryptionChoice::Preferred,
        EncryptionArg::None => EncryptionChoice::None,
        EncryptionArg::Doh => EncryptionChoice::Doh,
        EncryptionArg::Dot => EncryptionChoice::Dot,
    };
    Ok(DnsRequest {
        target,
        encryption,
        allow_fallback: args.allow_fallback,
    })
}

fn label_text(label: &DnsLabel) -> String {
    match label {
        DnsLabel::Automatic => "automatic (from the network)".into(),
        DnsLabel::LocalResolver => "local resolver".into(),
        DnsLabel::Provider { provider, variant } => {
            let name = catalog()
                .provider(provider)
                .map_or(provider.as_str(), |p| p.name.as_str());
            format!("{name} ({variant})")
        }
        DnsLabel::Preset { name } => format!("preset {name}"),
        DnsLabel::Custom => "custom servers".into(),
    }
}

fn progress(ctx: &Ctx) -> impl FnMut(Stage) + use<> {
    let quiet = ctx.json;
    move |stage| {
        if !quiet {
            let label = match stage {
                Stage::Preparing => "preparing",
                Stage::Writing => "writing the DNS settings",
                Stage::Restarting => "applying them to the connection",
                Stage::Verifying => "verifying",
                Stage::RollingBack => "rolling back",
            };
            eprintln!("... {label}");
        }
    }
}

fn outcome_line(outcome: &DnsOutcome) -> String {
    let name = &outcome.adapter.name;
    match &outcome.report {
        Ok(r) => {
            let what = r
                .observed
                .as_ref()
                .or(r.requested.as_ref())
                .map(ToString::to_string)
                .unwrap_or_default();
            let mut line = if r.status.is_success() {
                format!(
                    "dns {name} -> {what} ({:?} via {})",
                    r.status,
                    r.method.as_str()
                )
            } else {
                format!(
                    "dns of {name} was not changed ({:?} via {}); it uses {what}",
                    r.status,
                    r.method.as_str()
                )
            };
            if let Some(code) = &r.detail_code {
                line.push_str(&format!("\ndetail: {code}"));
            }
            for w in &r.warnings {
                line.push_str(&format!("\nnote: {w}"));
            }
            line
        }
        Err(e) => format!("dns of {name} was not changed: {e}"),
    }
}

/// Exit code for a finished change: 0 success, 3 authorization, else 5.
fn outcome_exit(outcome: &DnsOutcome) -> u8 {
    match &outcome.report {
        Ok(r) if r.status.is_success() => 0,
        Ok(_) => 5,
        Err(e) => net_exit(e),
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AllResult {
    adapter: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    entry: Option<HistoryEntry>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

/// Runs a change on one adapter or on every connected one, prints the outcome(s) and maps them
/// to the exit code.
fn change(
    ctx: &Ctx,
    target: &TargetArgs,
    run: &dyn Fn(&Ctx, &str) -> Result<DnsOutcome, randommac_net::ops::OpError>,
) -> anyhow::Result<()> {
    if !target.all {
        let query = target.adapter.as_deref().unwrap_or_default();
        let adapter = ctx.find(query)?;
        let outcome = run(ctx, &adapter.id).map_err(|e| fail(op_exit(&e), e))?;
        let code = outcome_exit(&outcome);
        ctx.print(&outcome.entry, || outcome_line(&outcome))?;
        return if code == 0 {
            Ok(())
        } else {
            Err(fail(code, "the DNS change did not take effect"))
        };
    }
    let targets = dns_ops::dns_targets(ctx.backend.as_ref()).map_err(|e| fail(net_exit(&e), e))?;
    if targets.is_empty() {
        return Err(fail(4, "no connected adapter can change its DNS settings"));
    }
    let mut results = Vec::new();
    let mut lines = Vec::new();
    let mut codes = Vec::new();
    for adapter in targets {
        match run(ctx, &adapter.id) {
            Ok(outcome) => {
                codes.push(outcome_exit(&outcome));
                lines.push(outcome_line(&outcome));
                results.push(AllResult {
                    adapter: adapter.name.clone(),
                    entry: outcome.entry,
                    error: None,
                });
            }
            Err(e) => {
                codes.push(op_exit(&e));
                lines.push(format!("dns of {} was not changed: {e}", adapter.name));
                results.push(AllResult {
                    adapter: adapter.name.clone(),
                    entry: None,
                    error: Some(e.to_string()),
                });
            }
        }
    }
    ctx.print(&results, || lines.join("\n"))?;
    let failed: Vec<u8> = codes.into_iter().filter(|c| *c != 0).collect();
    match failed.as_slice() {
        [] => Ok(()),
        all if all.iter().all(|c| *c == 3) => {
            Err(fail(3, "not authorized to change the DNS settings"))
        }
        _ => Err(fail(
            5,
            "the DNS settings of some adapters were not changed",
        )),
    }
}

/// Before switching to plain DNS: at least one of the servers must answer from here.
fn check_servers(ctx: &Ctx, request: &DnsRequest) -> anyhow::Result<()> {
    if request.encryption != EncryptionChoice::None || ctx.backend.capabilities().platform == "mock"
    {
        return Ok(());
    }
    let settings = ctx.settings();
    let all = dns::EncryptionSupport {
        doh: false,
        dot: false,
    };
    let Ok(resolved) = dns::resolve_request(request, catalog(), &settings.dns.presets, all) else {
        return Ok(()); // the change itself reports the problem
    };
    let targets: Vec<ProbeTarget> = resolved
        .config
        .servers()
        .into_iter()
        .map(|addr| ProbeTarget {
            label: addr.to_string(),
            addr,
        })
        .collect();
    if targets.is_empty() {
        return Ok(());
    }
    let opts = ProbeOptions {
        attempts: 1,
        ..ProbeOptions::default()
    };
    let mut answered = false;
    dns_probe::probe_all(&targets, &opts, &AtomicBool::new(false), &mut |_, o| {
        answered |= matches!(o, ProbeOutcome::Answered { .. });
    });
    if answered {
        Ok(())
    } else {
        Err(fail(
            5,
            "none of these DNS servers answered from this computer (use --no-check to switch anyway)",
        ))
    }
}

fn providers(ctx: &Ctx, only: Option<&str>) -> anyhow::Result<()> {
    let c = catalog();
    let list: Vec<&dns::Provider> = match only {
        Some(id) => vec![
            c.provider(id)
                .ok_or_else(|| fail(6, format!("unknown DNS provider {id:?}")))?,
        ],
        None => c.providers.iter().collect(),
    };
    ctx.print(&list, || {
        let mut out = Vec::new();
        for p in &list {
            out.push(format!("{} ({})", p.name, p.id));
            for v in &p.variants {
                let join = |l: Vec<String>| {
                    if l.is_empty() {
                        "-".to_string()
                    } else {
                        l.join(", ")
                    }
                };
                out.push(format!(
                    "  {}{}  [{}]\n    IPv4 {}\n    IPv6 {}\n    DoH  {}\n    DoT  {}",
                    v.id,
                    if v.default { " (default)" } else { "" },
                    serde_json::to_value(v.label)
                        .ok()
                        .and_then(|l| l.as_str().map(str::to_owned))
                        .unwrap_or_default(),
                    join(v.ipv4.iter().map(ToString::to_string).collect()),
                    join(v.ipv6.iter().map(ToString::to_string).collect()),
                    v.doh.as_deref().unwrap_or("-"),
                    v.dot.as_deref().unwrap_or("-"),
                ));
                for s in &v.dnscrypt {
                    out.push(if only.is_some() {
                        format!("    DNSCrypt {} {}", s.address, s.stamp)
                    } else {
                        format!(
                            "    DNSCrypt {} (stamp: dns providers --provider {})",
                            s.address, p.id
                        )
                    });
                }
            }
        }
        out.join("\n")
    })
}

fn show(ctx: &Ctx, query: &str) -> anyhow::Result<()> {
    let adapter = ctx.find(query)?;
    let settings = ctx.settings();
    let view = dns_ops::dns_view(ctx.backend.as_ref(), &ctx.store, &settings, &adapter.id)
        .map_err(|e| fail(op_exit(&e), e))?;
    ctx.print(&view, || {
        let caps = &view.capabilities;
        let feature = |name: &str, f: &randommac_net::FeatureSupport| match f.reason {
            None if f.available => format!("{name} yes"),
            reason => format!(
                "{name} no ({})",
                reason
                    .map(randommac_net::DnsReason::code)
                    .unwrap_or_default()
            ),
        };
        let mut lines = vec![format!("Adapter:     {}", adapter.name)];
        match &view.status {
            Some(s) => {
                lines.push(format!("Configured:  {}", s.configured));
                let used: Vec<String> = s
                    .effective_ipv4
                    .iter()
                    .map(ToString::to_string)
                    .chain(s.effective_ipv6.iter().map(ToString::to_string))
                    .collect();
                lines.push(format!(
                    "In use:      {}",
                    if used.is_empty() {
                        "-".into()
                    } else {
                        used.join(", ")
                    }
                ));
                if let Some(t) = &s.doh_template {
                    lines.push(format!(
                        "DoH:         {t}{}",
                        if s.fallback == Some(true) {
                            " (falls back to plain DNS)"
                        } else {
                            ""
                        }
                    ));
                }
                if let Some(n) = &s.dot_server_name {
                    lines.push(format!("DoT:         {n}"));
                }
                if s.profile_override {
                    lines.push(
                        "Note:        servers set in the Windows Settings app take precedence"
                            .into(),
                    );
                }
            }
            None => lines.push(format!(
                "Configured:  unknown ({})",
                view.status_error_code.as_deref().unwrap_or("not supported")
            )),
        }
        if let Some(label) = &view.matched {
            lines.push(format!("Matches:     {}", label_text(label)));
        }
        lines.push(format!(
            "Supported:   {}, {}, {}, {}",
            feature("IPv4", &caps.ipv4),
            feature("IPv6", &caps.ipv6),
            feature("DoH", &caps.doh),
            feature("DoT", &caps.dot)
        ));
        lines.push(format!(
            "Restorable:  {}",
            if view.restorable { "yes" } else { "no" }
        ));
        lines.join("\n")
    })
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TestRow {
    label: String,
    server: IpAddr,
    outcome: ProbeOutcome,
}

fn test(ctx: &Ctx, args: &TestArgs) -> anyhow::Result<()> {
    let mut targets = Vec::new();
    for ip in parse_ips(&args.servers)? {
        targets.push(ProbeTarget {
            label: ip.to_string(),
            addr: ip,
        });
    }
    let specs: Vec<DnsTarget> = if args.providers.is_empty() && targets.is_empty() {
        catalog()
            .providers
            .iter()
            .map(|p| DnsTarget::Provider {
                provider: p.id.clone(),
                variant: None,
            })
            .collect()
    } else {
        args.providers.iter().map(|s| parse_provider(s)).collect()
    };
    for spec in specs {
        let DnsTarget::Provider { provider, variant } = &spec else {
            continue;
        };
        let p = catalog()
            .provider(provider)
            .ok_or_else(|| fail(6, format!("unknown DNS provider {provider:?}")))?;
        let v = p
            .variant(variant.as_deref())
            .ok_or_else(|| fail(6, format!("unknown variant of {provider}")))?;
        let label = format!("{} ({})", p.name, v.id);
        for ip in v.ipv4.iter().map(|a| IpAddr::V4(*a)) {
            targets.push(ProbeTarget {
                label: label.clone(),
                addr: ip,
            });
        }
        if args.ipv6 {
            for ip in v.ipv6.iter().map(|a| IpAddr::V6(*a)) {
                targets.push(ProbeTarget {
                    label: label.clone(),
                    addr: ip,
                });
            }
        }
    }
    let opts = ProbeOptions {
        attempts: args.attempts,
        timeout: Duration::from_millis(args.timeout_ms.clamp(100, 10_000)),
        port: args.port,
        ..ProbeOptions::default()
    };
    let mut rows: Vec<Option<TestRow>> = (0..targets.len()).map(|_| None).collect();
    dns_probe::probe_all(
        &targets,
        &opts,
        &AtomicBool::new(false),
        &mut |i, outcome| {
            if let Some(t) = targets.get(i) {
                rows[i] = Some(TestRow {
                    label: t.label.clone(),
                    server: t.addr,
                    outcome,
                });
            }
        },
    );
    let mut rows: Vec<TestRow> = rows.into_iter().flatten().collect();
    rows.sort_by_key(|r| match &r.outcome {
        ProbeOutcome::Answered { stats, .. } => u64::from(stats.median_us),
        _ => u64::MAX,
    });
    let any = rows
        .iter()
        .any(|r| matches!(r.outcome, ProbeOutcome::Answered { .. }));
    ctx.print(&rows, || {
        rows.iter()
            .map(|r| {
                let result = match &r.outcome {
                    ProbeOutcome::Answered {
                        stats,
                        answered,
                        attempts,
                        ..
                    } => format!(
                        "{:>7.1} ms  ({:.1}–{:.1} ms, {answered}/{attempts} answered)",
                        f64::from(stats.median_us) / 1000.0,
                        f64::from(stats.min_us) / 1000.0,
                        f64::from(stats.max_us) / 1000.0
                    ),
                    ProbeOutcome::Timeout => "timed out".into(),
                    ProbeOutcome::Refused { .. } => "refused".into(),
                    ProbeOutcome::Unreachable { message } => format!("unreachable: {message}"),
                    ProbeOutcome::Cancelled => "cancelled".into(),
                };
                format!("{:<34} {:<26} {result}", r.label, r.server)
            })
            .collect::<Vec<_>>()
            .join("\n")
    })?;
    if any {
        Ok(())
    } else {
        Err(fail(5, "no DNS server answered"))
    }
}

pub(crate) fn run(ctx: &Ctx, command: DnsCommand) -> anyhow::Result<()> {
    let opts = DnsOptions::default();
    match command {
        DnsCommand::Providers { provider } => providers(ctx, provider.as_deref()),
        DnsCommand::Show { adapter } => show(ctx, &adapter),
        DnsCommand::Presets => {
            let presets = ctx.settings().dns.presets;
            ctx.print(&presets, || {
                if presets.is_empty() {
                    return "(no presets; create them on the DNS page of the app)".into();
                }
                presets
                    .iter()
                    .map(|p| {
                        let servers: Vec<String> = p
                            .ipv4
                            .iter()
                            .map(ToString::to_string)
                            .chain(p.ipv6.iter().map(ToString::to_string))
                            .collect();
                        format!("{:<24} {}", p.name, servers.join(", "))
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            })
        }
        DnsCommand::Set(args) => {
            let request = request(&args)?;
            if args.dry_run {
                let settings = ctx.settings();
                let resolved = dns::resolve_request(
                    &request,
                    catalog(),
                    &settings.dns.presets,
                    dns::EncryptionSupport {
                        doh: true,
                        dot: true,
                    },
                )
                .map_err(|e| fail(6, e))?;
                return ctx.print(&resolved.config, || {
                    format!(
                        "{} — {}",
                        resolved.config.summary(),
                        label_text(&resolved.label)
                    )
                });
            }
            if !args.no_check {
                check_servers(ctx, &request)?;
            }
            change(ctx, &args.target, &|ctx: &Ctx, id: &str| {
                let settings = ctx.settings();
                dns_ops::apply_dns(
                    ctx.backend.as_ref(),
                    &ctx.store,
                    &settings,
                    id,
                    &request,
                    ChangeSource::Cli,
                    &opts,
                    &mut progress(ctx),
                )
            })
        }
        DnsCommand::Auto(target) => {
            let request = DnsRequest {
                target: DnsTarget::Automatic,
                encryption: EncryptionChoice::None,
                allow_fallback: false,
            };
            change(ctx, &target, &|ctx: &Ctx, id: &str| {
                let settings = ctx.settings();
                dns_ops::apply_dns(
                    ctx.backend.as_ref(),
                    &ctx.store,
                    &settings,
                    id,
                    &request,
                    ChangeSource::Cli,
                    &opts,
                    &mut progress(ctx),
                )
            })
        }
        DnsCommand::Restore(target) => change(ctx, &target, &|ctx: &Ctx, id: &str| {
            let settings = ctx.settings();
            dns_ops::restore_dns(
                ctx.backend.as_ref(),
                &ctx.store,
                &settings,
                id,
                ChangeSource::Cli,
                &opts,
                &mut progress(ctx),
            )
        }),
        DnsCommand::Test(args) => test(ctx, &args),
    }
}
