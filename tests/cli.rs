use std::fs;

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

fn pp(home: &TempDir) -> Command {
    let mut command = Command::cargo_bin("pp").expect("binary exists");
    command.env("HOME", home.path());
    command
}

fn set(home: &TempDir, service: &str, field: &str, value: &str) {
    pp(home)
        .args(["set", service, field, value])
        .assert()
        .success();
}

fn configure_service(home: &TempDir, service: &str, dir: &str, command: &str) {
    set(home, service, "dir", dir);
    set(home, service, "command", command);
    set(home, service, "port", "8712");
    set(home, service, "domain", "personal.local");
}

#[test]
fn set_creates_config_and_updates_fields() {
    let home = TempDir::new().unwrap();

    set(&home, "personal", "dir", "~/Code/personal");
    set(&home, "personal", "command", "npm run dev");
    set(&home, "personal", "port", "8712");
    set(&home, "personal", "domain", "personal.local");

    let config = fs::read_to_string(home.path().join(".config/preside/config.toml")).unwrap();

    assert!(config.contains("[services.personal]"));
    assert!(config.contains("dir = \"~/Code/personal\""));
    assert!(config.contains("command = \"npm run dev\""));
    assert!(config.contains("port = 8712"));
    assert!(config.contains("domain = \"personal.local\""));
}

#[test]
fn set_updates_flat_xdg_config_when_it_exists() {
    let home = TempDir::new().unwrap();
    let config_dir = home.path().join(".config");
    fs::create_dir_all(&config_dir).unwrap();
    let config_path = config_dir.join("preside.toml");
    fs::write(&config_path, "[services.personal]\nport = 8712\n").unwrap();

    set(&home, "personal", "domain", "personal.local");

    let config = fs::read_to_string(&config_path).unwrap();
    assert!(config.contains("domain = \"personal.local\""));
    assert!(!home.path().join(".config/preside/config.toml").exists());
}

#[test]
fn set_updates_home_dotfile_config_when_it_exists() {
    let home = TempDir::new().unwrap();
    let config_path = home.path().join(".preside.toml");
    fs::write(&config_path, "[services.personal]\nport = 8712\n").unwrap();

    set(&home, "personal", "domain", "personal.local");

    let config = fs::read_to_string(&config_path).unwrap();
    assert!(config.contains("domain = \"personal.local\""));
    assert!(!home.path().join(".config/preside/config.toml").exists());
}

#[test]
fn list_shows_configured_services() {
    let home = TempDir::new().unwrap();
    set(&home, "personal", "dir", "~/Code/personal");
    set(&home, "personal", "port", "8712");
    set(&home, "personal", "domain", "personal.local");

    pp(&home)
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("personal"))
        .stdout(predicate::str::contains("8712"))
        .stdout(predicate::str::contains("personal.local"))
        .stdout(predicate::str::contains("~/Code/personal"));
}

#[test]
fn info_shows_service_fields_and_config_path() {
    let home = TempDir::new().unwrap();
    set(&home, "personal", "dir", "~/Code/personal");
    set(&home, "personal", "command", "npm run dev");
    set(&home, "personal", "port", "8712");
    set(&home, "personal", "domain", "personal.local");

    pp(&home)
        .args(["i", "personal"])
        .assert()
        .success()
        .stdout(predicate::str::contains("name: personal"))
        .stdout(predicate::str::contains("dir: ~/Code/personal"))
        .stdout(predicate::str::contains("command: npm run dev"))
        .stdout(predicate::str::contains("port: 8712"))
        .stdout(predicate::str::contains("domain: personal.local"))
        .stdout(predicate::str::contains(".config/preside/config.toml"));
}

#[test]
fn invalid_field_returns_useful_error() {
    let home = TempDir::new().unwrap();

    pp(&home)
        .args(["set", "personal", "alias", "personal.local"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid field 'alias'"));
}

#[test]
fn invalid_port_returns_useful_error() {
    let home = TempDir::new().unwrap();

    pp(&home)
        .args(["set", "personal", "port", "70000"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid port '70000'"));
}

#[test]
fn run_injects_environment_variables() {
    let home = TempDir::new().unwrap();
    let dir = TempDir::new().unwrap();
    configure_service(
        &home,
        "personal",
        dir.path().to_str().unwrap(),
        "printf '%s %s %s' \"$PORT\" \"$PRESIDE_NAME\" \"$PRESIDE_DOMAIN\"",
    );

    pp(&home)
        .args(["r", "personal"])
        .assert()
        .success()
        .stdout(predicate::str::contains("8712 personal personal.local"));
}

#[test]
fn detached_run_status_logs_and_stop_work() {
    let home = TempDir::new().unwrap();
    let dir = TempDir::new().unwrap();
    configure_service(
        &home,
        "personal",
        dir.path().to_str().unwrap(),
        "printf 'ready\\n'; while true; do sleep 1; done",
    );

    pp(&home)
        .args(["run", "personal", "--detach"])
        .assert()
        .success()
        .stdout(predicate::str::contains("started personal with PID"));

    pp(&home)
        .args(["status", "personal"])
        .assert()
        .success()
        .stdout(predicate::str::contains("status: running"))
        .stdout(predicate::str::contains("port: 8712"))
        .stdout(predicate::str::contains("domain: personal.local"));

    pp(&home)
        .args(["logs", "personal"])
        .assert()
        .success()
        .stdout(predicate::str::contains("personal.log"));

    pp(&home)
        .args(["stop", "personal"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("stopped personal")
                .or(predicate::str::contains("sent SIGTERM to personal")),
        );
}
