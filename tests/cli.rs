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
fn status_json_tolerates_non_utf8_service_output() {
    let home = TempDir::new().unwrap();
    let dir = TempDir::new().unwrap();
    configure_service(&home, "personal", dir.path().to_str().unwrap(), "true");
    let log_dir = home.path().join(".local/state/preside/logs");
    fs::create_dir_all(&log_dir).unwrap();
    fs::write(log_dir.join("personal.log"), b"output \xff\nready\n").unwrap();

    let output = pp(&home)
        .args(["status", "personal", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let report: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(
        report["status"]["recent_log_lines"],
        serde_json::json!(["output \u{fffd}", "ready"])
    );
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

    pp(&home)
        .args(["status", "personal", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"phase\": \"ready\""));
    pp(&home)
        .args(["doctor", "personal"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Owned process is running"));
    set(&home, "personal", "readiness", "http");
    pp(&home)
        .args(["status", "personal", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"phase\": \"ready\""))
        .stdout(predicate::str::contains("\"state_matches_config\": false"));

    let runtime_path = home
        .path()
        .join(".local/state/preside/services/personal.toml");
    let mut runtime: toml::Value =
        toml::from_str(&fs::read_to_string(&runtime_path).unwrap()).unwrap();
    runtime["readiness"] = "http".into();
    runtime["port"] = 0.into();
    runtime["started_at_unix"] = 0.into();
    fs::write(&runtime_path, toml::to_string(&runtime).unwrap()).unwrap();
    pp(&home)
        .args(["doctor", "personal"])
        .assert()
        .failure()
        .stdout(predicate::str::contains(
            "unhealthy: HTTP localhost:0/ did not return 2xx",
        ));

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

#[test]
fn static_site_rejects_command_and_zero_port() {
    let home = TempDir::new().unwrap();
    for extra in [vec!["--command", "echo wrong"], vec!["--port", "0"]] {
        let mut args = vec!["add", "site", "--dir", "/tmp", "--static"];
        if extra[0] != "--port" {
            args.extend(["--port", "8080"]);
        }
        args.extend(extra);
        pp(&home).args(args).assert().failure();
    }
    pp(&home)
        .args(["add", "site", "--dir", "/tmp", "--static", "--port", "8080"])
        .assert()
        .success();
    set(&home, "site", "port", "0");
    pp(&home)
        .args(["run", "site", "--detach"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("static site"));
}

#[test]
fn static_site_lifecycle_serves_assets_and_rejects_escape_paths() {
    use std::io::{Read, Write};
    use std::net::TcpStream;
    use std::os::unix::fs::symlink;
    use std::time::Duration;

    let home = TempDir::new().unwrap();
    let files = TempDir::new().unwrap();
    let root = files.path().join("site with spaces");
    fs::create_dir_all(root.join("nested")).unwrap();
    fs::create_dir_all(root.join("empty")).unwrap();
    fs::write(root.join("index.html"), "<h1>Preside static</h1>").unwrap();
    fs::write(root.join("app.js"), "console.log('static');").unwrap();
    fs::write(root.join("icon.svg"), "<svg/>").unwrap();
    fs::write(root.join("what?#.txt"), "encoded filename").unwrap();
    fs::write(root.join("nested/index.html"), "nested page").unwrap();
    fs::write(root.join(".env"), "PRIVATE_VALUE").unwrap();
    fs::write(files.path().join("secret.txt"), "OUTSIDE_ROOT").unwrap();
    symlink(files.path().join("secret.txt"), root.join("escape.txt")).unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    pp(&home)
        .args([
            "add",
            "static-site",
            "--dir",
            root.to_str().unwrap(),
            "--port",
            &port.to_string(),
            "--static",
            "--domain",
            "site.example",
            "--readiness",
            "http",
        ])
        .assert()
        .success();
    let result = pp(&home)
        .args(["status", "static-site", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let status: serde_json::Value = serde_json::from_slice(&result).unwrap();
    assert_eq!(status["service"]["kind"], "static");
    assert_eq!(status["missing_fields"], serde_json::json!([]));
    assert!(status["service"].get("command").is_none());
    pp(&home)
        .args(["run", "static-site", "--detach"])
        .assert()
        .success();
    struct StopOnDrop<'a>(&'a TempDir);
    impl Drop for StopOnDrop<'_> {
        fn drop(&mut self) {
            let _ = pp(self.0).args(["stop", "static-site"]).output();
        }
    }
    let _cleanup = StopOnDrop(&home);
    let request = |method: &str, path: &str, host: &str| {
        let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        write!(
            stream,
            "{method} {path} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n"
        )
        .unwrap();
        let mut result = String::new();
        stream.read_to_string(&mut result).unwrap();
        result
    };
    let host = format!("localhost:{port}");
    let index = request("GET", "/", &host);
    assert!(index.starts_with("HTTP/1.1 200"), "{index}");
    assert!(index.contains("text/html"));
    assert!(request("GET", "/", "site.example").starts_with("HTTP/1.1 200"));
    assert!(index.ends_with("<h1>Preside static</h1>"));
    let asset = request("GET", "/app.js?cache=1", &host);
    assert!(asset.starts_with("HTTP/1.1 200"));
    assert!(asset.contains("javascript"));
    assert!(request("GET", "/icon.svg", &host).contains("image/svg+xml"));
    assert!(request("GET", "/what%3F%23.txt", &host).ends_with("encoded filename"));
    let head = request("HEAD", "/", &host);
    assert!(head.starts_with("HTTP/1.1 200"));
    assert!(head.ends_with("\r\n\r\n"));
    assert!(request("GET", "/nested", &host).starts_with("HTTP/1.1 301"));
    assert!(request("GET", "/nested?query=1", &host).contains("Location: /nested/?query=1"));
    assert!(request("GET", "/nested/", &host).ends_with("nested page"));
    for path in ["/missing", "/empty/"] {
        assert!(request("GET", path, &host).starts_with("HTTP/1.1 404"));
    }
    for path in [
        "/../secret.txt",
        "/%2e%2e/secret.txt",
        "/%2e%2e%2fsecret.txt",
        "/escape.txt",
        "/.env",
    ] {
        let response = request("GET", path, &host);
        assert!(response.starts_with("HTTP/1.1 403"), "{path}: {response}");
        assert!(!response.contains("OUTSIDE_ROOT"));
        assert!(!response.contains("PRIVATE_VALUE"));
    }
    assert!(request("POST", "/", &host).starts_with("HTTP/1.1 405"));
    assert!(request("GET", "/", "untrusted.example").starts_with("HTTP/1.1 403"));
    pp(&home).args(["stop", "static-site"]).assert().success();
    let result = pp(&home)
        .args(["status", "static-site", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let status: serde_json::Value = serde_json::from_slice(&result).unwrap();
    assert_eq!(status["status"]["process_alive"], false);
    assert_eq!(status["status"]["listening"], false);
}

struct StopOnDrop<'a> {
    home: &'a TempDir,
    name: &'a str,
}

impl Drop for StopOnDrop<'_> {
    fn drop(&mut self) {
        let _ = pp(self.home).args(["stop", self.name]).output();
    }
}

fn long_running_service(home: &TempDir, dir: &TempDir) {
    pp(home)
        .args([
            "add",
            "managed",
            "--dir",
            dir.path().to_str().unwrap(),
            "--command",
            "exec /bin/sleep 60",
            "--port",
            "49001",
            "--readiness",
            "process",
        ])
        .assert()
        .success();
}

fn status_json(home: &TempDir, name: &str) -> serde_json::Value {
    let output = pp(home).args(["status", name, "--json"]).output().unwrap();
    assert!(output.status.success());
    serde_json::from_slice(&output.stdout).unwrap()
}

#[test]
fn concurrent_config_updates_do_not_lose_sites_or_fields() {
    let home = TempDir::new().unwrap();
    let binary = assert_cmd::cargo::cargo_bin("pp");
    let mut children = Vec::new();
    for index in 0..12 {
        children.push(
            std::process::Command::new(&binary)
                .env("HOME", home.path())
                .args([
                    "set",
                    "shared",
                    &format!("env.FIELD_{index}"),
                    &index.to_string(),
                ])
                .stdout(std::process::Stdio::null())
                .spawn()
                .unwrap(),
        );
    }
    for mut child in children {
        assert!(child.wait().unwrap().success());
    }
    let output = pp(&home).args(["list", "--json"]).output().unwrap();
    let config: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(config["shared"]["env"].as_object().unwrap().len(), 12);
    use std::os::unix::fs::PermissionsExt;
    let permissions = fs::metadata(home.path().join(".config/preside/config.toml"))
        .unwrap()
        .permissions();
    assert_eq!(permissions.mode() & 0o777, 0o600);
}

#[test]
fn concurrent_starts_create_one_managed_process() {
    let home = TempDir::new().unwrap();
    let dir = TempDir::new().unwrap();
    long_running_service(&home, &dir);
    let _cleanup = StopOnDrop {
        home: &home,
        name: "managed",
    };
    let binary = assert_cmd::cargo::cargo_bin("pp");
    let mut children = Vec::new();
    for _ in 0..4 {
        children.push(
            std::process::Command::new(&binary)
                .env("HOME", home.path())
                .args(["run", "managed", "--detach"])
                .stdout(std::process::Stdio::piped())
                .spawn()
                .unwrap(),
        );
    }
    let outputs: Vec<_> = children
        .into_iter()
        .map(|child| child.wait_with_output().unwrap())
        .collect();
    assert!(outputs.iter().all(|output| output.status.success()));
    assert_eq!(
        outputs
            .iter()
            .filter(|output| String::from_utf8_lossy(&output.stdout).starts_with("started managed"))
            .count(),
        1
    );
    assert!(status_json(&home, "managed")["status"]["identity_matches"]
        .as_bool()
        .unwrap());
}

#[test]
fn update_replaces_whole_configuration_and_running_site_requires_restart() {
    let home = TempDir::new().unwrap();
    let dir = TempDir::new().unwrap();
    long_running_service(&home, &dir);
    set(&home, "managed", "env.OLD", "old");
    set(&home, "managed", "domain", "old.local");
    pp(&home)
        .args(["run", "managed", "--detach"])
        .assert()
        .success();
    let _cleanup = StopOnDrop {
        home: &home,
        name: "managed",
    };
    let before = status_json(&home, "managed");
    pp(&home)
        .args([
            "update",
            "managed",
            "--dir",
            dir.path().to_str().unwrap(),
            "--command",
            "exec /bin/sleep 90",
            "--port",
            "49002",
            "--readiness",
            "process",
            "--env",
            "NEW=new",
        ])
        .assert()
        .success();
    let after = status_json(&home, "managed");
    assert_eq!(before["status"]["pid"], after["status"]["pid"]);
    assert_eq!(after["status"]["state_matches_config"], false);
    assert_eq!(after["service"]["env"], serde_json::json!({"NEW": "new"}));
    assert!(after["service"].get("domain").is_none());
    pp(&home).args(["restart", "managed"]).assert().success();
    let restarted = status_json(&home, "managed");
    assert_ne!(before["status"]["pid"], restarted["status"]["pid"]);
    assert_eq!(restarted["status"]["state_matches_config"], true);
}

#[test]
fn rename_and_remove_require_stopped_site() {
    let home = TempDir::new().unwrap();
    let dir = TempDir::new().unwrap();
    long_running_service(&home, &dir);
    let _cleanup = StopOnDrop {
        home: &home,
        name: "managed",
    };
    pp(&home)
        .args(["run", "managed", "--detach"])
        .assert()
        .success();
    pp(&home)
        .args(["rename", "managed", "renamed"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("stop the site"));
    pp(&home)
        .args(["remove", "managed"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("stop the site"));
    assert_eq!(
        status_json(&home, "managed")["status"]["identity_matches"],
        true
    );
    pp(&home).args(["stop", "managed"]).assert().success();
    pp(&home)
        .args(["rename", "managed", "renamed"])
        .assert()
        .success();
    pp(&home).args(["remove", "renamed"]).assert().success();
    let output = pp(&home).args(["list", "--json"]).output().unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&output.stdout).unwrap(),
        serde_json::json!({})
    );
}

#[test]
fn invalid_restart_keeps_existing_process_running() {
    let home = TempDir::new().unwrap();
    let dir = TempDir::new().unwrap();
    long_running_service(&home, &dir);
    let _cleanup = StopOnDrop {
        home: &home,
        name: "managed",
    };
    pp(&home)
        .args(["run", "managed", "--detach"])
        .assert()
        .success();
    let before = status_json(&home, "managed");
    set(
        &home,
        "managed",
        "dir",
        home.path().join("missing").to_str().unwrap(),
    );
    pp(&home).args(["restart", "managed"]).assert().failure();
    assert_eq!(
        before["status"]["pid"],
        status_json(&home, "managed")["status"]["pid"]
    );
}

#[test]
fn stale_whole_site_update_does_not_overwrite_newer_changes() {
    let home = TempDir::new().unwrap();
    let dir = TempDir::new().unwrap();
    long_running_service(&home, &dir);
    let expected = status_json(&home, "managed")["service"].to_string();
    set(&home, "managed", "env.NEW", "preserve");
    pp(&home)
        .args([
            "update",
            "managed",
            "--expected-config",
            &expected,
            "--dir",
            dir.path().to_str().unwrap(),
            "--command",
            "exec /bin/sleep 90",
            "--port",
            "49002",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("site changed since editing"));
    assert_eq!(
        status_json(&home, "managed")["service"]["env"]["NEW"],
        "preserve"
    );
    let current = status_json(&home, "managed")["service"].to_string();
    pp(&home)
        .args([
            "update",
            "managed",
            "--expected-config",
            &current,
            "--dir",
            dir.path().to_str().unwrap(),
            "--static",
            "--port",
            "49002",
        ])
        .assert()
        .success();
    assert_eq!(status_json(&home, "managed")["service"]["kind"], "static");
}

#[test]
fn stop_remains_available_when_configuration_is_invalid() {
    let home = TempDir::new().unwrap();
    let dir = TempDir::new().unwrap();
    long_running_service(&home, &dir);
    let _cleanup = StopOnDrop {
        home: &home,
        name: "managed",
    };
    pp(&home)
        .args(["run", "managed", "--detach"])
        .assert()
        .success();
    fs::write(
        home.path().join(".config/preside/config.toml"),
        "this is invalid TOML = [",
    )
    .unwrap();
    pp(&home).args(["stop", "managed"]).assert().success();
    assert!(!home
        .path()
        .join(".local/state/preside/pids/managed.pid")
        .exists());
}

#[test]
fn process_exiting_during_readiness_is_a_startup_failure() {
    let home = TempDir::new().unwrap();
    let dir = TempDir::new().unwrap();
    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let port = listener.local_addr().unwrap().port().to_string();
    drop(listener);
    pp(&home)
        .args([
            "add",
            "early-exit",
            "--dir",
            dir.path().to_str().unwrap(),
            "--command",
            "sleep 1; exit 7",
            "--port",
            &port,
        ])
        .assert()
        .success();
    let _cleanup = StopOnDrop {
        home: &home,
        name: "early-exit",
    };
    pp(&home)
        .args(["run", "early-exit", "--detach"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "exited while waiting for readiness",
        ));
    assert!(!home
        .path()
        .join(".local/state/preside/pids/early-exit.pid")
        .exists());
}

#[test]
fn owned_runtime_state_recovers_when_pid_file_is_missing() {
    let home = TempDir::new().unwrap();
    let dir = TempDir::new().unwrap();
    long_running_service(&home, &dir);
    let _cleanup = StopOnDrop {
        home: &home,
        name: "managed",
    };
    pp(&home)
        .args(["run", "managed", "--detach"])
        .assert()
        .success();
    let original = status_json(&home, "managed");
    fs::remove_file(home.path().join(".local/state/preside/pids/managed.pid")).unwrap();
    let recovered = status_json(&home, "managed");
    assert_eq!(recovered["status"]["pid"], original["status"]["pid"]);
    assert_eq!(recovered["status"]["pid_file_exists"], false);
    assert_eq!(recovered["status"]["identity_matches"], true);
    pp(&home)
        .args(["run", "managed", "--detach"])
        .assert()
        .success()
        .stdout(predicate::str::contains("already be running"));
    pp(&home).args(["stop", "managed"]).assert().success();
    let stopped = status_json(&home, "managed");
    assert_eq!(stopped["status"]["process_alive"], false);
    assert!(stopped["status"]["state"].is_null());
}

#[test]
fn mismatched_runtime_name_without_pid_never_authorizes_signals_or_replacement() {
    let home = TempDir::new().unwrap();
    let dir = TempDir::new().unwrap();
    long_running_service(&home, &dir);
    let _cleanup = StopOnDrop {
        home: &home,
        name: "managed",
    };
    pp(&home)
        .args(["run", "managed", "--detach"])
        .assert()
        .success();
    let state_path = home
        .path()
        .join(".local/state/preside/services/managed.toml");
    struct RestoreState(std::path::PathBuf, String);
    impl Drop for RestoreState {
        fn drop(&mut self) {
            let _ = fs::write(&self.0, &self.1);
        }
    }
    let restore = RestoreState(state_path.clone(), fs::read_to_string(&state_path).unwrap());
    fs::write(
        &state_path,
        restore
            .1
            .replace("name = \"managed\"", "name = \"different-site\""),
    )
    .unwrap();
    fs::remove_file(home.path().join(".local/state/preside/pids/managed.pid")).unwrap();
    let unknown = status_json(&home, "managed");
    assert_eq!(unknown["status"]["process_alive"], true);
    assert_eq!(unknown["status"]["identity_matches"], false);
    pp(&home)
        .args(["run", "managed", "--detach"])
        .assert()
        .failure();
    pp(&home)
        .args(["stop", "managed"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("unknown process"));
    pp(&home)
        .args(["rename", "managed", "new-name"])
        .assert()
        .failure();
    pp(&home).args(["remove", "managed"]).assert().failure();
    assert_eq!(
        status_json(&home, "managed")["status"]["process_alive"],
        true
    );
    drop(restore);
    pp(&home).args(["stop", "managed"]).assert().success();
}

#[test]
fn json_logs_are_bounded_lossy_and_read_only() {
    let home = TempDir::new().unwrap();
    let path = home.path().join(".local/state/preside/logs/personal.log");
    pp(&home)
        .args(["logs", "personal", "--json"])
        .assert()
        .success();
    assert!(!path.exists());
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut bytes = (0..1100)
        .map(|i| format!("line {i}: {}\n", "x".repeat(30)))
        .collect::<String>()
        .into_bytes();
    bytes.extend_from_slice(b"invalid \xff\n");
    fs::write(&path, &bytes).unwrap();
    let output = pp(&home)
        .args(["logs", "personal", "--json", "--lines", "2000"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let lines = value["lines"].as_array().unwrap();
    assert_eq!(lines.len(), 1000);
    assert!(lines[0].as_str().unwrap().starts_with("line 101:"));
    assert_eq!(lines.last().unwrap(), "invalid \u{fffd}");
    assert_eq!(fs::read(&path).unwrap(), bytes);
    pp(&home)
        .args(["logs", "personal", "--json", "--clear"])
        .assert()
        .failure();
}
