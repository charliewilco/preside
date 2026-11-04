use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use nix::sys::signal::{kill, Signal};
use nix::unistd::Pid;
use tempfile::TempDir;

fn wait_for(mut predicate: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !predicate() {
        assert!(
            Instant::now() < deadline,
            "timed out waiting for launch boundary"
        );
        thread::sleep(Duration::from_millis(10));
    }
}

fn alive(pid: i32) -> bool {
    let output = Command::new("ps")
        .args(["-p", &pid.to_string(), "-o", "stat="])
        .output()
        .unwrap();
    output.status.success()
        && !String::from_utf8_lossy(&output.stdout)
            .trim()
            .starts_with('Z')
}

struct Cleanup(i32);

impl Drop for Cleanup {
    fn drop(&mut self) {
        let _ = kill(Pid::from_raw(self.0), Signal::SIGKILL);
    }
}

fn controller(directory: &Path, boundary: &str) -> Child {
    let mut controller = Command::new(std::env::current_exe().unwrap())
        .args(["--ignored", "--exact", "controller_fixture", "--nocapture"])
        .env("PRESIDE_TEST_LAUNCH_DIR", directory)
        .env("PRESIDE_TEST_LAUNCH_BOUNDARY", boundary)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut output = BufReader::new(controller.stdout.take().unwrap());
    loop {
        let mut line = String::new();
        assert_ne!(output.read_line(&mut line).unwrap(), 0);
        if line.trim_end().ends_with("LAUNCH_BOUNDARY") {
            break;
        }
    }
    controller
}

#[test]
fn controller_death_before_release_never_executes_service() {
    for boundary in ["before-state", "after-state", "partial-release"] {
        let directory = TempDir::new().unwrap();
        let mut controller = controller(directory.path(), boundary);
        let pid: i32 = fs::read_to_string(directory.path().join("helper-pid"))
            .unwrap()
            .parse()
            .unwrap();
        let _cleanup = Cleanup(pid);
        controller.kill().unwrap();
        controller.wait().unwrap();
        wait_for(|| !alive(pid));
        assert!(!directory.path().join("executed").exists());
        assert_eq!(
            directory.path().join("state").exists(),
            boundary != "before-state"
        );
    }
}

#[test]
fn controller_death_after_release_preserves_recorded_process_identity() {
    let directory = TempDir::new().unwrap();
    let mut controller = controller(directory.path(), "after-release");
    let pid: i32 = fs::read_to_string(directory.path().join("state"))
        .unwrap()
        .parse()
        .unwrap();
    let _cleanup = Cleanup(pid);
    controller.kill().unwrap();
    controller.wait().unwrap();
    assert!(alive(pid));
    assert_eq!(
        fs::read_to_string(directory.path().join("executed"))
            .unwrap()
            .trim(),
        pid.to_string()
    );
}

#[test]
#[ignore]
fn controller_fixture() {
    let Some(directory) = std::env::var_os("PRESIDE_TEST_LAUNCH_DIR") else {
        return;
    };
    let directory = Path::new(&directory);
    let boundary = std::env::var("PRESIDE_TEST_LAUNCH_BOUNDARY").unwrap();
    let mut helper = Command::new(env!("CARGO_BIN_EXE_pp"))
        .args([
            "__launch",
            "--",
            "sh",
            "-c",
            "echo $$ > executed; exec sleep 60",
        ])
        .current_dir(directory)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    fs::write(directory.join("helper-pid"), helper.id().to_string()).unwrap();
    if boundary != "before-state" {
        preside::storage::atomic_write(
            &directory.join("state"),
            helper.id().to_string().as_bytes(),
        )
        .unwrap();
    }
    if boundary == "partial-release" {
        helper.stdin.as_mut().unwrap().write_all(b"PRES").unwrap();
    }
    if boundary == "after-release" {
        helper
            .stdin
            .take()
            .unwrap()
            .write_all(b"PRESIDE\n")
            .unwrap();
        wait_for(|| directory.join("executed").exists());
    }
    println!("LAUNCH_BOUNDARY");
    std::io::stdout().flush().unwrap();
    let mut line = String::new();
    std::io::stdin().read_line(&mut line).unwrap();
    let _ = helper.kill();
    let _ = helper.wait();
    panic!("controller must be killed at its requested boundary");
}
