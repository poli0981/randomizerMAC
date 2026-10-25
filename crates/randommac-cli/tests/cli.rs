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

#[test]
fn text_output_of_show_and_history() {
    let dir = tempfile::tempdir().expect("tempdir");
    cli(dir.path())
        .args(["show", "eth0"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("Name:          Ethernet")
                .and(predicate::str::contains("Up, connected"))
                .and(predicate::str::contains("00:E0:4C:68:12:34 (Realtek)"))
                .and(predicate::str::contains("Methods:       mock"))
                .and(predicate::str::contains("IPv4:          192.168.1.23")),
        );
    cli(dir.path())
        .args(["show", "vboxnet0"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Kind:          Ethernet (virtual)",
        ));
    cli(dir.path())
        .arg("history")
        .assert()
        .success()
        .stdout(predicate::str::contains("(no history)"));

    cli(dir.path())
        .args(["set", "eth0", "02:11:22:33:44:55"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "changed Ethernet -> 02:11:22:33:44:55 (Applied via mock)",
        ))
        .stderr(predicate::str::contains("... writing the new address"));
    cli(dir.path())
        .args(["history", "--adapter", "Ethernet"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("apply").and(predicate::str::contains(
                "00:E0:4C:68:12:34 -> 02:11:22:33:44:55",
            )),
        );
    // An adapter that no longer exists is matched by id.
    cli(dir.path())
        .args(["history", "--adapter", "gone"])
        .assert()
        .success()
        .stdout(predicate::str::contains("(no history)"));
}

#[test]
fn dry_runs_validate_without_changing() {
    let dir = tempfile::tempdir().expect("tempdir");
    cli(dir.path())
        .args(["set", "eth0", "02:11:22:33:44:55", "--dry-run"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "02:11:22:33:44:55 can be applied to Ethernet",
        ));
    cli(dir.path())
        .args(["set", "eth0", "FF:FF:FF:FF:FF:FF", "--dry-run"])
        .assert()
        .code(6);
    let out = cli(dir.path())
        .args([
            "--json",
            "random",
            "eth0",
            "--mode",
            "keep-vendor",
            "--dry-run",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let mac: String = serde_json::from_slice(&out).expect("json string");
    assert!(mac.starts_with("00:E0:4C:"), "{mac}");
    cli(dir.path())
        .args(["random", "eth0", "--dry-run"])
        .assert()
        .success()
        .stdout(predicate::str::is_match("^[0-9A-F]{2}(:[0-9A-F]{2}){5}\n$").expect("regex"));
    cli(dir.path())
        .args(["--json", "history"])
        .assert()
        .success()
        .stdout(predicate::str::contains("[]"));
}

#[test]
fn blacklist_rules_are_enforced_unless_ignored() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        dir.path().join("blacklist.json"),
        r#"{"schemaVersion":1,"entries":[{"id":"b1","pattern":"02:AB:CD:*","createdAt":"2026-01-01T00:00:00Z"}]}"#,
    )
    .expect("write blacklist");
    cli(dir.path())
        .args(["set", "eth0", "02:AB:CD:00:00:01"])
        .assert()
        .code(6)
        .stderr(predicate::str::contains("02:AB:CD"));
    cli(dir.path())
        .args(["set", "eth0", "02:AB:CD:00:00:01", "--dry-run"])
        .assert()
        .code(6);
    cli(dir.path())
        .args(["set", "eth0", "02:AB:CD:00:00:01", "--ignore-blacklist"])
        .assert()
        .success();
}

#[test]
fn failed_and_rolled_back_changes() {
    let dir = tempfile::tempdir().expect("tempdir");
    // The mock's "Ethernet 3" adapter is disabled.
    cli(dir.path())
        .args(["set", "Ethernet 3", "02:11:22:33:44:55"])
        .assert()
        .code(5)
        .stderr(predicate::str::contains("the adapter is disabled"));
    let out = cli(dir.path())
        .args(["--json", "set", "enx0", "02:11:22:33:44:55"])
        .assert()
        .code(5)
        .get_output()
        .stdout
        .clone();
    let entry: serde_json::Value = serde_json::from_slice(&out).expect("json");
    assert_eq!(entry["status"], "failed");
    cli(dir.path())
        .env("RANDOMMAC_MOCK_SCENARIO", "driver_ignores")
        .args(["set", "eth0", "02:11:22:33:44:55"])
        .assert()
        .code(5)
        .stdout(
            predicate::str::contains("Ethernet was not changed (RolledBack via mock)")
                .and(predicate::str::contains("detail: DRIVER_REJECTED")),
        );
}

#[test]
fn linux_method_and_json_details() {
    let dir = tempfile::tempdir().expect("tempdir");
    for method in ["auto", "nm", "networkmanager", "iproute2"] {
        cli(dir.path())
            .args(["--method", method, "set", "wlan0", "02:11:22:33:44:55"])
            .assert()
            .success();
    }
    cli(dir.path())
        .args(["--method", "bogus", "list"])
        .assert()
        .code(2);
    let out = cli(dir.path())
        .args(["--json", "inspect", "02:11:22:33:44:55"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let info: serde_json::Value = serde_json::from_slice(&out).expect("json");
    assert_eq!(info["locallyAdministered"], true);
    assert_eq!(info["windowsWifiCompatible"], true);
    cli(dir.path()).args(["inspect", "nope"]).assert().code(6);
    let out = cli(dir.path())
        .args(["--json", "vendors"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let vendors: serde_json::Value = serde_json::from_slice(&out).expect("json");
    assert!(
        vendors
            .as_array()
            .is_some_and(|v| v.iter().any(|v| v["id"] == "apple"))
    );
    let out = cli(dir.path())
        .args(["--json", "list", "--all"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let list: serde_json::Value = serde_json::from_slice(&out).expect("json");
    assert_eq!(list.as_array().map(Vec::len), Some(4));
}
