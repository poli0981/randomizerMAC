//! The latency test: how long DNS servers take to answer a plain (UDP) query from this computer.
//! It sends one small query per attempt and nothing else, and only when the user starts it.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, UdpSocket};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use randommac_core::dns::wire::{self, LatencyStats};
use randommac_core::random::{OsRandom, RandomSource};
use serde::Serialize;

pub const MAX_TARGETS: usize = 64;
pub const MAX_ATTEMPTS: u8 = 5;
pub const MAX_CONCURRENCY: usize = 16;
/// The REFUSED response code: the server does not answer queries from this network.
const RCODE_REFUSED: u8 = 5;
/// How often a waiting worker checks for cancellation.
const SLICE: Duration = Duration::from_millis(100);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeTarget {
    pub label: String,
    pub addr: IpAddr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProbeOptions {
    /// Timed queries per server (after one untimed warm-up query).
    pub attempts: u8,
    /// How long to wait for each answer.
    pub timeout: Duration,
    /// Servers probed at the same time.
    pub concurrency: usize,
    /// 53, except in tests.
    pub port: u16,
}

impl Default for ProbeOptions {
    fn default() -> Self {
        Self {
            attempts: 3,
            timeout: Duration::from_millis(1500),
            concurrency: 8,
            port: wire::DNS_PORT,
        }
    }
}

/// What one server did.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ProbeOutcome {
    #[serde(rename_all = "camelCase")]
    Answered {
        stats: LatencyStats,
        answered: u8,
        attempts: u8,
        rcode: u8,
    },
    Timeout,
    #[serde(rename_all = "camelCase")]
    Refused {
        rcode: u8,
    },
    Unreachable {
        message: String,
    },
    Cancelled,
}

fn random_id() -> u16 {
    let mut b = [0u8; 2];
    // Any value works as a query id; fall back to the clock if the OS generator is unavailable.
    if OsRandom.fill(&mut b).is_err() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(0)
            .to_le_bytes();
        b.copy_from_slice(&nanos[..2]);
    }
    u16::from_le_bytes(b)
}

enum Reply {
    Answer(u8, Duration),
    Timeout,
    Cancelled,
    Failed(String),
}

/// Sends one query and waits for its answer, ignoring packets for other ids.
fn query(socket: &UdpSocket, timeout: Duration, cancel: &AtomicBool) -> Reply {
    let id = random_id();
    let started = Instant::now();
    if let Err(e) = socket.send(&wire::build_query(id)) {
        return Reply::Failed(e.to_string());
    }
    let mut buf = [0u8; 512];
    loop {
        if cancel.load(Ordering::Relaxed) {
            return Reply::Cancelled;
        }
        let left = timeout.saturating_sub(started.elapsed());
        if left.is_zero() {
            return Reply::Timeout;
        }
        if socket.set_read_timeout(Some(left.min(SLICE))).is_err() {
            return Reply::Failed("cannot set a read timeout".into());
        }
        match socket.recv(&mut buf) {
            Ok(n) => {
                if let Ok(info) = wire::parse_response(id, &buf[..n]) {
                    return Reply::Answer(info.rcode, started.elapsed());
                }
            }
            Err(e)
                if matches!(
                    e.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) => {}
            Err(e) => return Reply::Failed(e.to_string()),
        }
    }
}

/// Probes one server: a warm-up query, then `attempts` timed ones.
pub fn probe_one(addr: IpAddr, opts: &ProbeOptions, cancel: &AtomicBool) -> ProbeOutcome {
    let local: IpAddr = match addr {
        IpAddr::V4(_) => IpAddr::V4(Ipv4Addr::UNSPECIFIED),
        IpAddr::V6(_) => IpAddr::V6(Ipv6Addr::UNSPECIFIED),
    };
    let socket = match UdpSocket::bind(SocketAddr::new(local, 0))
        .and_then(|s| s.connect(SocketAddr::new(addr, opts.port)).map(|()| s))
    {
        Ok(s) => s,
        Err(e) => {
            return ProbeOutcome::Unreachable {
                message: e.to_string(),
            };
        }
    };
    match query(&socket, opts.timeout, cancel) {
        Reply::Answer(rcode, _) if rcode == RCODE_REFUSED => {
            return ProbeOutcome::Refused { rcode };
        }
        Reply::Answer(..) => {}
        Reply::Timeout => return ProbeOutcome::Timeout,
        Reply::Cancelled => return ProbeOutcome::Cancelled,
        Reply::Failed(message) => return ProbeOutcome::Unreachable { message },
    }
    let attempts = opts.attempts.clamp(1, MAX_ATTEMPTS);
    let mut samples = Vec::new();
    let mut rcode = 0;
    for _ in 0..attempts {
        match query(&socket, opts.timeout, cancel) {
            Reply::Answer(code, elapsed) => {
                rcode = code;
                samples.push(elapsed);
            }
            Reply::Timeout => {}
            Reply::Cancelled => return ProbeOutcome::Cancelled,
            Reply::Failed(message) => return ProbeOutcome::Unreachable { message },
        }
    }
    match LatencyStats::from_samples(&samples) {
        Some(stats) => ProbeOutcome::Answered {
            stats,
            answered: u8::try_from(samples.len()).unwrap_or(u8::MAX),
            attempts,
            rcode,
        },
        None => ProbeOutcome::Timeout,
    }
}

/// Probes up to [`MAX_TARGETS`] servers, a few at a time, and reports each result (by index into
/// `targets`) on the calling thread as soon as it is known. Setting `cancel` stops the test
/// within a fraction of a second; unfinished servers report [`ProbeOutcome::Cancelled`].
pub fn probe_all(
    targets: &[ProbeTarget],
    opts: &ProbeOptions,
    cancel: &AtomicBool,
    on_result: &mut dyn FnMut(usize, ProbeOutcome),
) {
    let targets = &targets[..targets.len().min(MAX_TARGETS)];
    if targets.is_empty() {
        return;
    }
    let next = AtomicUsize::new(0);
    let workers = opts
        .concurrency
        .clamp(1, MAX_CONCURRENCY)
        .min(targets.len());
    let (tx, rx) = mpsc::channel();
    std::thread::scope(|scope| {
        for _ in 0..workers {
            let tx = tx.clone();
            let next = &next;
            scope.spawn(move || {
                loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    let Some(target) = targets.get(i) else { break };
                    let outcome = if cancel.load(Ordering::Relaxed) {
                        ProbeOutcome::Cancelled
                    } else {
                        probe_one(target.addr, opts, cancel)
                    };
                    if tx.send((i, outcome)).is_err() {
                        break;
                    }
                }
            });
        }
        drop(tx);
        for (i, outcome) in rx {
            on_result(i, outcome);
        }
    });
}

#[cfg(test)]
mod tests {
    use std::thread;

    use super::*;

    #[derive(Clone, Copy)]
    enum Behavior {
        Answer,
        Refuse,
        /// Sends a reply for another query first, then the right one.
        WrongIdFirst,
        Silent,
    }

    /// A local DNS "server" that answers every query according to `behavior`.
    fn responder(ip: IpAddr, behavior: Behavior) -> Option<u16> {
        let socket = UdpSocket::bind(SocketAddr::new(ip, 0)).ok()?;
        let port = socket.local_addr().ok()?.port();
        thread::spawn(move || {
            let mut buf = [0u8; 512];
            while let Ok((n, from)) = socket.recv_from(&mut buf) {
                if n < 12 {
                    continue;
                }
                let mut reply = buf[..n].to_vec();
                reply[2] |= 0x80;
                match behavior {
                    Behavior::Answer => {}
                    Behavior::Refuse => reply[3] = (reply[3] & 0xf0) | RCODE_REFUSED,
                    Behavior::WrongIdFirst => {
                        let mut wrong = reply.clone();
                        wrong[0] ^= 0xff;
                        let _ = socket.send_to(&wrong, from);
                    }
                    Behavior::Silent => continue,
                }
                let _ = socket.send_to(&reply, from);
            }
        });
        Some(port)
    }

    fn opts(port: u16) -> ProbeOptions {
        ProbeOptions {
            attempts: 2,
            timeout: Duration::from_millis(400),
            concurrency: 4,
            port,
        }
    }

    const LOCAL: IpAddr = IpAddr::V4(Ipv4Addr::LOCALHOST);

    #[test]
    fn answered_refused_and_silent_servers() {
        let never = AtomicBool::new(false);
        let port = responder(LOCAL, Behavior::Answer).expect("responder");
        assert!(matches!(
            probe_one(LOCAL, &opts(port), &never),
            ProbeOutcome::Answered {
                answered: 2,
                attempts: 2,
                rcode: 0,
                ..
            }
        ));
        let port = responder(LOCAL, Behavior::WrongIdFirst).expect("responder");
        assert!(matches!(
            probe_one(LOCAL, &opts(port), &never),
            ProbeOutcome::Answered { answered: 2, .. }
        ));
        let port = responder(LOCAL, Behavior::Refuse).expect("responder");
        assert_eq!(
            probe_one(LOCAL, &opts(port), &never),
            ProbeOutcome::Refused { rcode: 5 }
        );
        let port = responder(LOCAL, Behavior::Silent).expect("responder");
        assert_eq!(probe_one(LOCAL, &opts(port), &never), ProbeOutcome::Timeout);
    }

    #[test]
    fn ipv6_loopback_when_available() {
        let v6 = IpAddr::V6(Ipv6Addr::LOCALHOST);
        let Some(port) = responder(v6, Behavior::Answer) else {
            return; // no IPv6 loopback on this machine
        };
        assert!(matches!(
            probe_one(v6, &opts(port), &AtomicBool::new(false)),
            ProbeOutcome::Answered { .. }
        ));
    }

    #[test]
    fn cancelled_before_and_during() {
        let port = responder(LOCAL, Behavior::Silent).expect("responder");
        let cancelled = AtomicBool::new(true);
        assert_eq!(
            probe_one(LOCAL, &opts(port), &cancelled),
            ProbeOutcome::Cancelled
        );

        let targets: Vec<ProbeTarget> = (0..3)
            .map(|i| ProbeTarget {
                label: format!("t{i}"),
                addr: LOCAL,
            })
            .collect();
        let mut seen = Vec::new();
        probe_all(&targets, &opts(port), &cancelled, &mut |i, o| {
            seen.push((i, o))
        });
        seen.sort_by_key(|(i, _)| *i);
        assert_eq!(seen.len(), 3);
        assert!(seen.iter().all(|(_, o)| *o == ProbeOutcome::Cancelled));
    }

    #[test]
    fn probe_all_reports_every_target_once() {
        let answer = responder(LOCAL, Behavior::Answer).expect("responder");
        let targets: Vec<ProbeTarget> = (0..6)
            .map(|i| ProbeTarget {
                label: format!("t{i}"),
                addr: LOCAL,
            })
            .collect();
        let mut seen = Vec::new();
        probe_all(
            &targets,
            &opts(answer),
            &AtomicBool::new(false),
            &mut |i, o| seen.push((i, o)),
        );
        seen.sort_by_key(|(i, _)| *i);
        assert_eq!(
            seen.iter().map(|(i, _)| *i).collect::<Vec<_>>(),
            [0, 1, 2, 3, 4, 5]
        );
        assert!(
            seen.iter()
                .all(|(_, o)| matches!(o, ProbeOutcome::Answered { .. }))
        );

        let mut none = 0;
        probe_all(&[], &opts(answer), &AtomicBool::new(false), &mut |_, _| {
            none += 1
        });
        assert_eq!(none, 0);
    }

    #[test]
    fn unreachable_and_serialization() {
        // Port 0 cannot be connected to.
        let o = probe_one(
            LOCAL,
            &ProbeOptions { port: 0, ..opts(0) },
            &AtomicBool::new(false),
        );
        assert!(
            matches!(o, ProbeOutcome::Unreachable { .. } | ProbeOutcome::Timeout),
            "{o:?}"
        );
        let json = serde_json::to_value(ProbeOutcome::Refused { rcode: 5 }).expect("ser");
        assert_eq!(json, serde_json::json!({"kind": "refused", "rcode": 5}));
        assert_eq!(ProbeOptions::default().port, 53);
    }
}
