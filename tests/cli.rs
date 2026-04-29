use std::fs;
use std::net::TcpListener;

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
fn add_creates_a_complete_service() {
    let home = TempDir::new().unwrap();
    let dir = TempDir::new().unwrap();

    pp(&home)
        .args([
            "add",
            "personal",
            "--dir",
            dir.path().to_str().unwrap(),
            "--command",
            "npm run dev",
            "--port",
            "8712",
            "--domain",
            "personal.local",
            "--readiness",
            "process",
            "--env",
            "NODE_ENV=development",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("added personal"));

    pp(&home)
        .args(["info", "personal", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"readiness\": \"process\""))
        .stdout(predicate::str::contains("\"NODE_ENV\": \"development\""));
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
fn list_supports_json_output() {
    let home = TempDir::new().unwrap();
    set(&home, "personal", "port", "8712");

    pp(&home)
        .args(["list", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"personal\""))
        .stdout(predicate::str::contains("\"port\": 8712"));
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
fn invalid_service_name_returns_useful_error() {
    let home = TempDir::new().unwrap();

    pp(&home)
        .args(["set", "../personal", "port", "8712"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "invalid service name '../personal'",
        ));
}

#[test]
fn invalid_domain_returns_useful_error() {
    let home = TempDir::new().unwrap();

    pp(&home)
        .args(["set", "personal", "domain", "http://personal.local"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "invalid domain 'http://personal.local'",
        ));
}

#[test]
fn unset_removes_a_service_field() {
    let home = TempDir::new().unwrap();
    set(&home, "personal", "port", "8712");
    set(&home, "personal", "domain", "personal.local");

    pp(&home)
        .args(["unset", "personal", "domain"])
        .assert()
        .success()
        .stdout(predicate::str::contains("unset personal.domain"));

    let config = fs::read_to_string(home.path().join(".config/preside/config.toml")).unwrap();
    assert!(config.contains("port = 8712"));
    assert!(!config.contains("domain"));
}

#[test]
fn remove_deletes_a_service() {
    let home = TempDir::new().unwrap();
    set(&home, "personal", "port", "8712");

    pp(&home)
        .args(["remove", "personal"])
        .assert()
        .success()
        .stdout(predicate::str::contains("removed personal"));

    pp(&home)
        .args(["info", "personal"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "service 'personal' is not configured",
        ));
}

#[test]
fn rename_moves_a_service() {
    let home = TempDir::new().unwrap();
    set(&home, "personal", "port", "8712");

    pp(&home)
        .args(["rename", "personal", "work"])
        .assert()
        .success()
        .stdout(predicate::str::contains("renamed personal to work"));

    pp(&home)
        .args(["info", "personal"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "service 'personal' is not configured",
        ));

    pp(&home)
        .args(["info", "work"])
        .assert()
        .success()
        .stdout(predicate::str::contains("port: 8712"));
}

#[test]
fn path_prints_config_and_state_paths() {
    let home = TempDir::new().unwrap();

    pp(&home)
        .arg("path")
        .assert()
        .success()
        .stdout(predicate::str::contains(".config/preside/config.toml"));

    pp(&home)
        .args(["path", "--log", "personal"])
        .assert()
        .success()
        .stdout(predicate::str::contains("logs/personal.log"));

    pp(&home)
        .args(["path", "--service-state", "personal"])
        .assert()
        .success()
        .stdout(predicate::str::contains("services/personal.toml"));
}

#[test]
fn doctor_succeeds_for_complete_stopped_service() {
    let home = TempDir::new().unwrap();
    let dir = TempDir::new().unwrap();
    configure_service(&home, "personal", dir.path().to_str().unwrap(), "true");

    pp(&home)
        .arg("doctor")
        .assert()
        .success()
        .stdout(predicate::str::contains("Doctor found no issues."));
}

#[test]
fn doctor_fails_for_incomplete_service() {
    let home = TempDir::new().unwrap();
    set(&home, "personal", "port", "8712");

    pp(&home)
        .arg("doctor")
        .assert()
        .failure()
        .stdout(predicate::str::contains("config: missing dir, command"))
        .stdout(predicate::str::contains("Doctor found 2 issue(s)."));
}

#[test]
fn doctor_fix_removes_stale_runtime_state() {
    let home = TempDir::new().unwrap();
    let dir = TempDir::new().unwrap();
    configure_service(&home, "personal", dir.path().to_str().unwrap(), "true");
    let state_dir = home.path().join(".local/state/preside/services");
    let pid_dir = home.path().join(".local/state/preside/pids");
    fs::create_dir_all(&state_dir).unwrap();
    fs::create_dir_all(&pid_dir).unwrap();
    fs::write(pid_dir.join("personal.pid"), "999999\n").unwrap();
    fs::write(
        state_dir.join("personal.toml"),
        "name = \"personal\"\npid = 999999\ndir = \".\"\ncommand = \"true\"\nport = 8712\nreadiness = \"port\"\nstarted_at_unix = 1\nlog_path = \"personal.log\"\n",
    )
    .unwrap();

    pp(&home)
        .args(["doctor", "personal", "--fix"])
        .assert()
        .failure()
        .stdout(predicate::str::contains(
            "fixed: removed stale PID and state files",
        ));

    assert!(!pid_dir.join("personal.pid").exists());
    assert!(!state_dir.join("personal.toml").exists());
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
fn run_injects_configured_environment_variables() {
    let home = TempDir::new().unwrap();
    let dir = TempDir::new().unwrap();
    configure_service(
        &home,
        "personal",
        dir.path().to_str().unwrap(),
        "printf '%s' \"$NODE_ENV\"",
    );
    set(&home, "personal", "env.NODE_ENV", "development");

    pp(&home)
        .args(["r", "personal"])
        .assert()
        .success()
        .stdout(predicate::str::contains("development"));
}

#[test]
fn hidden_complete_services_prints_configured_services() {
    let home = TempDir::new().unwrap();
    set(&home, "personal", "port", "8712");

    pp(&home)
        .args(["__complete", "services"])
        .assert()
        .success()
        .stdout(predicate::str::contains("personal"));
}

#[test]
fn logs_support_lines_and_clear() {
    let home = TempDir::new().unwrap();
    let log_dir = home.path().join(".local/state/preside/logs");
    fs::create_dir_all(&log_dir).unwrap();
    let log_path = log_dir.join("personal.log");
    fs::write(&log_path, "one\ntwo\nthree\n").unwrap();

    pp(&home)
        .args(["logs", "personal", "--lines", "2"])
        .assert()
        .success()
        .stdout(predicate::str::contains("two"))
        .stdout(predicate::str::contains("three"))
        .stdout(predicate::str::contains("one").not());

    pp(&home)
        .args(["logs", "personal", "--clear"])
        .assert()
        .success()
        .stdout(predicate::str::contains("cleared"));

    assert_eq!(fs::read_to_string(&log_path).unwrap(), "");
}

#[test]
fn completions_print_shell_completion_script() {
    let home = TempDir::new().unwrap();

    pp(&home)
        .args(["completions", "bash"])
        .assert()
        .success()
        .stdout(predicate::str::contains("_pp"));
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

#[test]
fn detached_process_readiness_does_not_require_port() {
    let home = TempDir::new().unwrap();
    let dir = TempDir::new().unwrap();
    configure_service(
        &home,
        "personal",
        dir.path().to_str().unwrap(),
        "while true; do sleep 1; done",
    );
    set(&home, "personal", "readiness", "process");

    pp(&home)
        .args(["run", "personal", "--detach"])
        .assert()
        .success()
        .stdout(predicate::str::contains("process is running"));

    pp(&home)
        .args(["logs", "personal", "--lines", "5"])
        .assert()
        .success()
        .stdout(predicate::str::contains("[preside]"))
        .stdout(predicate::str::contains("started pid="));

    pp(&home).args(["stop", "personal"]).assert().success();
}

#[test]
fn detached_run_fails_when_command_exits_immediately() {
    let home = TempDir::new().unwrap();
    let dir = TempDir::new().unwrap();
    configure_service(
        &home,
        "personal",
        dir.path().to_str().unwrap(),
        "printf 'boom\\n'; exit 7",
    );

    pp(&home)
        .args(["run", "personal", "--detach"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "personal exited shortly after starting",
        ));
}

#[test]
fn detached_run_refuses_occupied_port() {
    let home = TempDir::new().unwrap();
    let dir = TempDir::new().unwrap();
    let Ok(listener) = TcpListener::bind("127.0.0.1:0") else {
        return;
    };
    let port = listener.local_addr().unwrap().port().to_string();

    set(&home, "personal", "dir", dir.path().to_str().unwrap());
    set(&home, "personal", "command", "while true; do sleep 1; done");
    set(&home, "personal", "port", &port);

    pp(&home)
        .args(["run", "personal", "--detach"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("already appears to be in use"));
}
