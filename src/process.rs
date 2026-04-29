use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom};
use std::net::{SocketAddr, TcpStream};
use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, bail, Context, Result};
use nix::errno::Errno;
use nix::sys::signal::{self, Signal};
use nix::unistd::{setsid, Pid};
use serde::{Deserialize, Serialize};

use crate::paths::PresidePaths;
use crate::service::{validate_service_name, Service};

const LOG_ROTATE_BYTES: u64 = 10 * 1024 * 1024;
const STARTUP_ALIVE_WAIT: Duration = Duration::from_millis(500);
const PORT_WAIT_ATTEMPTS: usize = 20;
const PORT_WAIT_INTERVAL: Duration = Duration::from_millis(100);

#[derive(Debug, Clone)]
pub struct ServiceStatus {
    pub pid_path: PathBuf,
    pub log_path: PathBuf,
    pub state_path: PathBuf,
    pub pid: Option<i32>,
    pub pid_file_exists: bool,
    pub process_alive: bool,
    pub identity_matches: bool,
    pub state_matches_config: Option<bool>,
    pub listening: Option<bool>,
    pub url: Option<String>,
    pub state: Option<ServiceState>,
    pub log_modified_unix: Option<u64>,
    pub recent_log_lines: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServiceState {
    pub name: String,
    pub pid: u32,
    pub dir: String,
    pub command: String,
    pub port: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub env: BTreeMap<String, String>,
    pub started_at_unix: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub process_started_at: Option<String>,
    pub log_path: String,
}

pub fn run_foreground(service: &Service) -> Result<ExitCode> {
    let dir = expand_service_dir(service)?;
    let status = service_command(service)
        .current_dir(&dir)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .with_context(|| format!("failed to spawn service '{}'", service.name))?;

    if status.success() {
        return Ok(ExitCode::SUCCESS);
    }

    let code = status
        .code()
        .or_else(|| status.signal().map(|signal| 128 + signal))
        .unwrap_or(1);

    Ok(ExitCode::from((code & 0xff) as u8))
}

pub fn run_detached(service: &Service, paths: &PresidePaths) -> Result<()> {
    let dir = expand_service_dir(service)?;
    paths.ensure_state_dirs()?;

    let status = service_status(&service.name, Some(service), Some(service.port), paths)?;

    if status.identity_matches {
        let pid = status.pid.expect("identity match requires a PID");
        println!(
            "{} appears to already be running with PID {pid}",
            service.name
        );
        return Ok(());
    }

    if status.process_alive {
        bail!(
            "{} has a live PID file at {}, but it does not match Preside state; refusing to start over it",
            service.name,
            status.pid_path.display()
        );
    }

    if status.pid_file_exists || status.state.is_some() {
        cleanup_runtime_files(&service.name, paths)?;
    }

    if port_listening(service.port) {
        bail!(
            "port {} already appears to be in use; refusing to start {} detached",
            service.port,
            service.name
        );
    }

    let log_path = paths.log_path(&service.name);
    let stdout = open_log(&log_path)?;
    let stderr = stdout
        .try_clone()
        .with_context(|| format!("failed to clone log file {}", log_path.display()))?;

    let mut command = service_command(service);
    command
        .current_dir(&dir)
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr));

    unsafe {
        command.pre_exec(|| {
            setsid().map_err(std::io::Error::other)?;
            Ok(())
        });
    }

    let mut child = command
        .spawn()
        .with_context(|| format!("failed to spawn detached service '{}'", service.name))?;
    let pid = child.id();

    let pid_path = paths.pid_path(&service.name);
    fs::write(&pid_path, format!("{pid}\n"))
        .with_context(|| format!("failed to write PID file {}", pid_path.display()))?;

    let state = ServiceState::new(service, pid, &dir, &log_path);
    write_state(paths, &state)?;

    thread::sleep(STARTUP_ALIVE_WAIT);

    if let Some(status) = child.try_wait()? {
        let recent_logs = recent_log_lines(&log_path, 20)?;
        cleanup_runtime_files(&service.name, paths)?;

        if recent_logs.is_empty() {
            bail!(
                "{} exited shortly after starting with status {status}",
                service.name
            );
        }

        bail!(
            "{} exited shortly after starting with status {status}\nrecent logs:\n{}",
            service.name,
            recent_logs.join("\n")
        );
    }

    if !process_alive(pid as i32) {
        let recent_logs = recent_log_lines(&log_path, 20)?;
        cleanup_runtime_files(&service.name, paths)?;

        if recent_logs.is_empty() {
            bail!("{} exited shortly after starting", service.name);
        }

        bail!(
            "{} exited shortly after starting\nrecent logs:\n{}",
            service.name,
            recent_logs.join("\n")
        );
    }

    let listening = wait_until_port_listening(service.port);

    println!("started {} with PID {pid}", service.name);
    println!("log: {}", log_path.display());

    if listening {
        println!("port {} is listening", service.port);
    } else {
        println!(
            "port {} is not listening yet; check `pp status {}` or `pp logs {}`",
            service.port, service.name, service.name
        );
    }

    Ok(())
}

pub fn stop_service(service: &str, paths: &PresidePaths) -> Result<()> {
    validate_service_name(service)?;
    let status = service_status(service, None, None, paths)?;
    let Some(pid) = status.pid else {
        println!("{service} is not running.");
        return Ok(());
    };

    if !status.process_alive {
        cleanup_runtime_files(service, paths)?;
        println!("{service} is not running. Removed stale state.");
        return Ok(());
    }

    if !status.identity_matches {
        bail!(
            "{service} has PID {pid}, but it does not match Preside state; refusing to signal an unknown process"
        );
    }

    send_signal(pid, Signal::SIGTERM)
        .with_context(|| format!("failed to send SIGTERM to {service} with PID {pid}"))?;

    if wait_until_stopped(pid) {
        cleanup_runtime_files(service, paths)?;
        println!("stopped {service}");
        return Ok(());
    }

    send_signal(pid, Signal::SIGKILL)
        .with_context(|| format!("failed to send SIGKILL to {service} with PID {pid}"))?;

    if wait_until_stopped(pid) {
        cleanup_runtime_files(service, paths)?;
        println!("force stopped {service}");
        return Ok(());
    }

    println!("{service} did not stop after SIGKILL; leaving state in place.");
    Ok(())
}

pub fn service_status(
    name: &str,
    service: Option<&Service>,
    port: Option<u16>,
    paths: &PresidePaths,
) -> Result<ServiceStatus> {
    validate_service_name(name)?;

    let pid_path = paths.pid_path(name);
    let log_path = paths.log_path(name);
    let state_path = paths.service_state_path(name);
    let pid = read_pid(&pid_path)?;
    let state = read_state(&state_path)?;
    let process_alive = pid.is_some_and(process_alive);
    let identity_matches = pid
        .zip(state.as_ref())
        .is_some_and(|(pid, state)| state_identifies_process(state, pid));
    let state_matches_config = service.map(|service| {
        state
            .as_ref()
            .is_some_and(|state| state.matches_service(service))
    });
    let listening = port.map(port_listening);
    let url = service.map(service_url);
    let log_modified_unix = file_modified_unix(&log_path)?;
    let recent_log_lines = recent_log_lines(&log_path, 5)?;

    Ok(ServiceStatus {
        pid_path,
        log_path,
        state_path,
        pid,
        pid_file_exists: pid.is_some(),
        process_alive,
        identity_matches,
        state_matches_config,
        listening,
        url,
        state,
        log_modified_unix,
        recent_log_lines,
    })
}

pub fn print_logs(
    service: &str,
    paths: &PresidePaths,
    lines: usize,
    follow: bool,
    clear: bool,
) -> Result<()> {
    validate_service_name(service)?;
    let log_path = paths.log_path(service);

    if clear {
        clear_log(&log_path)?;
        println!("cleared {}", log_path.display());

        if !follow {
            return Ok(());
        }
    }

    println!("log: {}", log_path.display());

    if !log_path.exists() {
        if let Some(parent) = log_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create log directory {}", parent.display()))?;
        }

        File::create(&log_path)
            .with_context(|| format!("failed to create log file {}", log_path.display()))?;
    }

    for line in recent_log_lines(&log_path, lines)? {
        println!("{line}");
    }

    if follow {
        follow_log(&log_path)?;
    }

    Ok(())
}

pub fn clear_log(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create log directory {}", parent.display()))?;
    }

    File::create(path).with_context(|| format!("failed to clear log file {}", path.display()))?;
    Ok(())
}

pub fn service_url(service: &Service) -> String {
    match &service.domain {
        Some(domain) => format!("http://{domain}"),
        None => format!("http://localhost:{}", service.port),
    }
}

pub fn expand_service_dir(service: &Service) -> Result<PathBuf> {
    let expanded = shellexpand::full(&service.dir)
        .with_context(|| format!("failed to expand directory for service '{}'", service.name))?;
    let dir = PathBuf::from(expanded.as_ref());

    if !dir.is_dir() {
        bail!(
            "directory for service '{}' does not exist: {}",
            service.name,
            dir.display()
        );
    }

    Ok(dir)
}

pub fn port_listening(port: u16) -> bool {
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    TcpStream::connect_timeout(&addr, Duration::from_millis(100)).is_ok()
}

pub fn recent_log_lines(path: &Path, lines: usize) -> Result<Vec<String>> {
    if !path.exists() || lines == 0 {
        return Ok(Vec::new());
    }

    let mut file =
        File::open(path).with_context(|| format!("failed to open log file {}", path.display()))?;
    let len = file.metadata()?.len();
    let start = len.saturating_sub(16 * 1024);
    file.seek(SeekFrom::Start(start))?;

    let mut contents = String::new();
    file.read_to_string(&mut contents)?;

    let mut result = contents
        .lines()
        .rev()
        .take(lines)
        .map(str::to_string)
        .collect::<Vec<_>>();
    result.reverse();

    Ok(result)
}

impl ServiceState {
    fn new(service: &Service, pid: u32, dir: &Path, log_path: &Path) -> Self {
        Self {
            name: service.name.clone(),
            pid,
            dir: dir.display().to_string(),
            command: service.command.clone(),
            port: service.port,
            domain: service.domain.clone(),
            env: service.env.clone(),
            started_at_unix: unix_now(),
            process_started_at: process_started_at(pid as i32),
            log_path: log_path.display().to_string(),
        }
    }

    fn matches_service(&self, service: &Service) -> bool {
        self.name == service.name
            && self.command == service.command
            && self.port == service.port
            && self.domain == service.domain
            && self.env == service.env
    }
}

fn service_command(service: &Service) -> Command {
    let mut command = Command::new("sh");
    command
        .arg("-c")
        .arg(&service.command)
        .env("PORT", service.port.to_string())
        .env("PRESIDE_NAME", &service.name);

    if let Some(domain) = &service.domain {
        command.env("PRESIDE_DOMAIN", domain);
    }

    for (key, value) in &service.env {
        command.env(key, value);
    }

    command
}

fn open_log(path: &Path) -> Result<File> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create log directory {}", parent.display()))?;
    }

    rotate_log_if_needed(path, LOG_ROTATE_BYTES)?;

    OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .with_context(|| format!("failed to open log file {}", path.display()))
}

fn follow_log(path: &Path) -> Result<()> {
    let mut file =
        File::open(path).with_context(|| format!("failed to open log file {}", path.display()))?;
    file.seek(SeekFrom::End(0))?;

    loop {
        let mut contents = String::new();
        let bytes = file.read_to_string(&mut contents)?;

        if bytes > 0 {
            print!("{contents}");
        } else {
            thread::sleep(Duration::from_millis(500));
        }
    }
}

fn rotate_log_if_needed(path: &Path, max_bytes: u64) -> Result<()> {
    let Ok(metadata) = fs::metadata(path) else {
        return Ok(());
    };

    if metadata.len() <= max_bytes {
        return Ok(());
    }

    let backup = rotated_log_path(path);

    if backup.exists() {
        fs::remove_file(&backup)
            .with_context(|| format!("failed to remove old log backup {}", backup.display()))?;
    }

    fs::rename(path, &backup).with_context(|| {
        format!(
            "failed to rotate log {} to {}",
            path.display(),
            backup.display()
        )
    })?;

    Ok(())
}

fn rotated_log_path(path: &Path) -> PathBuf {
    match path.extension().and_then(|extension| extension.to_str()) {
        Some(extension) => path.with_extension(format!("{extension}.1")),
        None => path.with_extension("1"),
    }
}

fn read_pid(path: &Path) -> Result<Option<i32>> {
    if !path.exists() {
        return Ok(None);
    }

    let contents = fs::read_to_string(path)
        .with_context(|| format!("failed to read PID file {}", path.display()))?;
    let trimmed = contents.trim();

    if trimmed.is_empty() {
        return Err(anyhow!("PID file is empty: {}", path.display()));
    }

    trimmed
        .parse::<i32>()
        .map(Some)
        .with_context(|| format!("invalid PID in {}", path.display()))
}

fn read_state(path: &Path) -> Result<Option<ServiceState>> {
    if !path.exists() {
        return Ok(None);
    }

    let contents = fs::read_to_string(path)
        .with_context(|| format!("failed to read service state {}", path.display()))?;
    toml::from_str(&contents)
        .map(Some)
        .with_context(|| format!("failed to parse service state {}", path.display()))
}

fn write_state(paths: &PresidePaths, state: &ServiceState) -> Result<()> {
    let path = paths.service_state_path(&state.name);
    let contents = toml::to_string_pretty(state).context("failed to serialize service state")?;

    fs::write(&path, contents)
        .with_context(|| format!("failed to write service state {}", path.display()))
}

fn cleanup_runtime_files(service: &str, paths: &PresidePaths) -> Result<()> {
    remove_file_if_exists(&paths.pid_path(service))?;
    remove_file_if_exists(&paths.service_state_path(service))?;
    Ok(())
}

fn remove_file_if_exists(path: &Path) -> Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error).with_context(|| format!("failed to remove {}", path.display())),
    }
}

fn state_identifies_process(state: &ServiceState, pid: i32) -> bool {
    if state.pid != pid as u32 {
        return false;
    }

    match (&state.process_started_at, process_started_at(pid)) {
        (Some(expected), Some(actual)) => expected == &actual,
        (None, _) => true,
        _ => false,
    }
}

fn process_alive(pid: i32) -> bool {
    match signal::kill(Pid::from_raw(pid), None) {
        Ok(()) => !process_is_zombie(pid),
        Err(Errno::EPERM) => true,
        Err(Errno::ESRCH) => false,
        Err(_) => false,
    }
}

fn send_signal(pid: i32, signal: Signal) -> nix::Result<()> {
    signal::kill(Pid::from_raw(-pid), signal).or_else(|_| signal::kill(Pid::from_raw(pid), signal))
}

fn wait_until_stopped(pid: i32) -> bool {
    for _ in 0..20 {
        if !process_alive(pid) {
            return true;
        }

        thread::sleep(Duration::from_millis(100));
    }

    false
}

fn wait_until_port_listening(port: u16) -> bool {
    for _ in 0..PORT_WAIT_ATTEMPTS {
        if port_listening(port) {
            return true;
        }

        thread::sleep(PORT_WAIT_INTERVAL);
    }

    false
}

fn process_is_zombie(pid: i32) -> bool {
    let pid_arg = pid.to_string();
    let Ok(output) = Command::new("ps")
        .args(["-p", &pid_arg, "-o", "stat="])
        .output()
    else {
        return false;
    };

    if !output.status.success() {
        return false;
    }

    String::from_utf8_lossy(&output.stdout).contains('Z')
}

fn process_started_at(pid: i32) -> Option<String> {
    let pid_arg = pid.to_string();
    let output = Command::new("ps")
        .args(["-p", &pid_arg, "-o", "lstart="])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let value = String::from_utf8_lossy(&output.stdout).trim().to_string();

    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

fn file_modified_unix(path: &Path) -> Result<Option<u64>> {
    if !path.exists() {
        return Ok(None);
    }

    let modified = fs::metadata(path)
        .with_context(|| format!("failed to read metadata for {}", path.display()))?
        .modified()
        .with_context(|| format!("failed to read modified time for {}", path.display()))?;

    Ok(Some(system_time_unix(modified)))
}

fn unix_now() -> u64 {
    system_time_unix(SystemTime::now())
}

fn system_time_unix(time: SystemTime) -> u64 {
    time.duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0))
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn service(domain: Option<&str>) -> Service {
        Service {
            name: "personal".to_string(),
            dir: ".".to_string(),
            command: "true".to_string(),
            port: 8712,
            domain: domain.map(str::to_string),
            env: BTreeMap::new(),
        }
    }

    #[test]
    fn url_prefers_domain() {
        assert_eq!(
            service_url(&service(Some("personal.local"))),
            "http://personal.local"
        );
    }

    #[test]
    fn url_uses_localhost_without_domain() {
        assert_eq!(service_url(&service(None)), "http://localhost:8712");
    }

    #[test]
    fn rotates_log_when_it_exceeds_limit() {
        let temp_dir = TempDir::new().unwrap();
        let log_path = temp_dir.path().join("personal.log");
        fs::write(&log_path, "0123456789").unwrap();

        rotate_log_if_needed(&log_path, 5).unwrap();

        assert!(!log_path.exists());
        assert_eq!(
            fs::read_to_string(temp_dir.path().join("personal.log.1")).unwrap(),
            "0123456789"
        );
    }

    #[test]
    fn recent_log_lines_returns_the_tail() {
        let temp_dir = TempDir::new().unwrap();
        let log_path = temp_dir.path().join("personal.log");
        fs::write(&log_path, "one\ntwo\nthree\n").unwrap();

        assert_eq!(
            recent_log_lines(&log_path, 2).unwrap(),
            vec!["two".to_string(), "three".to_string()]
        );
    }
}
