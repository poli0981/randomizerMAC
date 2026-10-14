//! End-to-end tests of the CLI against the mock backend (no real adapters are touched).

use assert_cmd::Command;
use predicates::prelude::*;

fn cli(dir: &std::path::Path) -> Command {
    let mut cmd = Command::cargo_bin("randommac-cli").expect("binary is built");
    cmd.arg("--mock").arg("--data-dir").arg(dir);
    cmd.env_remove("RANDOMMAC_MOCK_SCENARIO");
    cmd
}

#[test]
fn list_and_show() {
    let dir = tempfile::tempdir().expect("tempdir");
    cli(dir.path()).arg("list").assert().success().stdout(
        predicate::str::contains("Wi-Fi").and(predicate::str::contains("VirtualBox").not()),
    );
    cli(dir.path())
        .args(["list", "--all"])
        .assert()
        .success()
        .stdout(predicate::str::contains("(virtual)"));
    let out = cli(dir.path())
        .args(["--json", "show", "wlan0"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v: serde_json::Value = serde_json::from_slice(&out).expect("json");
    assert_eq!(v["id"], "mock-wifi");
    assert_eq!(v["kind"], "wireless");
    assert_eq!(v["originalMac"], "00:1B:21:3A:4B:5C");
}

#[test]
fn random_set_restore_and_history() {
    let dir = tempfile::tempdir().expect("tempdir");
    cli(dir.path())
        .args(["--json", "random", "Ethernet"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"status\": \"applied\""));
    cli(dir.path())
        .args(["set", "eth0", "02-AB-CD-EF-00-01"])
        .assert()
        .success()
        .stdout(predicate::str::contains("02:AB:CD:EF:00:01"));
    cli(dir.path())
        .args(["restore", "eth0"])
        .assert()
        .success()
        .stdout(predicate::str::contains("00:E0:4C:68:12:34"));
    let out = cli(dir.path())
        .args(["--json", "history", "--limit", "10"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let entries: serde_json::Value = serde_json::from_slice(&out).expect("json");
    let entries = entries.as_array().expect("array");
    assert_eq!(entries.len(), 3);
    assert_eq!(entries[0]["action"], "restore");
    assert!(entries.iter().all(|e| e["source"] == "cli"));
}

#[test]
fn errors_have_exit_codes() {
    let dir = tempfile::tempdir().expect("tempdir");
    cli(dir.path()).args(["show", "nope"]).assert().code(4);
    cli(dir.path())
        .args(["set", "eth0", "01:00:5E:00:00:01"])
        .assert()
        .code(6);
    cli(dir.path())
        .args(["set", "eth0", "not-a-mac"])
        .assert()
        .code(6);
    cli(dir.path())
        .args(["random", "eth0", "--mode", "vendor"])
        .assert()
        .code(2);
    cli(dir.path())
        .env("RANDOMMAC_MOCK_SCENARIO", "driver_ignores")
        .args(["random", "eth0"])
        .assert()
        .code(5);
}

#[test]
fn inspect_vendors_and_dry_run() {
    let dir = tempfile::tempdir().expect("tempdir");
    cli(dir.path())
        .args(["inspect", "00-03-93-12-34-56"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("Apple")
                .and(predicate::str::contains("Windows Wi-Fi ready:   false")),
        );
    cli(dir.path())
        .arg("vendors")
        .assert()
        .success()
        .stdout(predicate::str::contains("tp-link"));
    let out = cli(dir.path())
        .args([
            "--json",
            "random",
            "wlan0",
            "--mode",
            "vendor",
            "--vendor",
            "apple",
            "--dry-run",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let mac: String = serde_json::from_slice(&out).expect("json string");
    assert!(mac.parse::<randommac_core_shim::Mac>().is_ok());
    // Dry runs change nothing.
    cli(dir.path())
        .args(["--json", "history"])
        .assert()
        .success()
        .stdout(predicate::str::contains("[]"));
}

/// Minimal parser so this test does not depend on internal crates.
mod randommac_core_shim {
    pub struct Mac;
    impl std::str::FromStr for Mac {
        type Err = ();
        fn from_str(s: &str) -> Result<Self, ()> {
            let parts: Vec<&str> = s.split(':').collect();
            (parts.len() == 6
                && parts
                    .iter()
                    .all(|p| p.len() == 2 && u8::from_str_radix(p, 16).is_ok()))
            .then_some(Mac)
            .ok_or(())
        }
    }
}
