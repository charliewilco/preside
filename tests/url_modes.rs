use assert_cmd::Command;
use serde_json::Value;
use tempfile::TempDir;

fn pp(home: &TempDir) -> Command {
    let mut command = Command::cargo_bin("pp").expect("binary exists");
    command.env("HOME", home.path());
    command
}

fn status(home: &TempDir) -> Value {
    let output = pp(home)
        .args(["status", "site", "--json"])
        .output()
        .unwrap();
    assert!(output.status.success());
    serde_json::from_slice(&output.stdout).unwrap()
}

fn add(home: &TempDir, mode: Option<&str>) {
    let mut command = pp(home);
    command.args([
        "add",
        "site",
        "--static",
        "--dir",
        home.path().to_str().unwrap(),
        "--port",
        "8080",
        "--domain",
        "site.local",
    ]);
    if let Some(mode) = mode {
        command.args(["--url-mode", mode]);
    }
    command.assert().success();
}

#[test]
fn domain_url_mode_is_explicit_and_legacy_configs_keep_their_url() {
    for (mode, expected) in [
        (None, "http://site.local"),
        (Some("direct"), "http://site.local:8080"),
        (Some("proxy"), "http://site.local"),
    ] {
        let home = TempDir::new().unwrap();
        add(&home, mode);
        let report = status(&home);
        assert_eq!(report["status"]["url"], expected);
        if let Some(mode) = mode {
            assert_eq!(report["service"]["url_mode"], mode);
        } else {
            assert!(report["service"].get("url_mode").is_none());
        }
    }
}

#[test]
fn url_modes_support_set_unset_update_and_no_domain() {
    let home = TempDir::new().unwrap();
    add(&home, None);
    pp(&home)
        .args(["set", "site", "url-mode", "direct"])
        .assert()
        .success();
    assert_eq!(status(&home)["status"]["url"], "http://site.local:8080");
    pp(&home)
        .args(["unset", "site", "url_mode"])
        .assert()
        .success();
    assert_eq!(status(&home)["status"]["url"], "http://site.local");
    pp(&home)
        .args([
            "update",
            "site",
            "--static",
            "--dir",
            home.path().to_str().unwrap(),
            "--port",
            "8081",
            "--domain",
            "site.local",
            "--url-mode",
            "direct",
        ])
        .assert()
        .success();
    assert_eq!(status(&home)["status"]["url"], "http://site.local:8081");
    pp(&home)
        .args(["unset", "site", "domain"])
        .assert()
        .success();
    assert_eq!(status(&home)["status"]["url"], "http://localhost:8081");
    pp(&home)
        .args(["set", "site", "url_mode", "https"])
        .assert()
        .failure();
    assert_eq!(status(&home)["service"]["url_mode"], "direct");
}
