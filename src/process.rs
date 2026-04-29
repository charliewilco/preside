use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom};
use std::net::{SocketAddr, TcpStream};
use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::path::PathBuf;
use std::process::{Command, ExitCode, Stdio};
use std::thread;
use std::time::Duration;

use anyhow::{anyhow, bail, Context, Result};
use nix::errno::Errno;
use nix::sys::signal::{self, Signal};
use nix::unistd::{setsid, Pid};

use crate::paths::PresidePaths;
use crate::service::Service;

#[derive(Debug, Clone)]
pub struct ServiceStatus {
    pub pid_path: PathBuf,
    pub log_path: PathBuf,
    pub pid: Option<i32>,
    pub alive: bool,
    pub listening: bool,
}

pub fn run_foreground(service: &Service) -> Result<ExitCode> {
    let dir = expand_dir(service)?;
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
    let dir = expand_dir(service)?;
    paths.ensure_state_dirs()?;

    let pid_path = paths.pid_path(&service.name);
    let log_path = paths.log_path(&service.name);

    if let Some(pid) = read_pid(&pid_path)? {
        if process_alive(pid) {
            println!(
                "{} appears to already be running with PID {pid}",
                service.name
            );
            return Ok(());
        }

        println!(
            "removing stale PID file for {} at {}",
            service.name,
            pid_path.display()
        );
        fs::remove_file(&pid_path)
            .with_context(|| format!("failed to remove stale PID file {}", pid_path.display()))?;
    }

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

    let child = command
        .spawn()
        .with_context(|| format!("failed to spawn detached service '{}'", service.name))?;
    let pid = child.id();

    fs::write(&pid_path, format!("{pid}\n"))
        .with_context(|| format!("failed to write PID file {}", pid_path.display()))?;

    println!("started {} with PID {pid}", service.name);
    println!("log: {}", log_path.display());

    Ok(())
}

pub fn stop_service(service: &str, paths: &PresidePaths) -> Result<()> {
    let pid_path = paths.pid_path(service);
    let Some(pid) = read_pid(&pid_path)? else {
        println!("{service} is not running.");
        return Ok(());
    };

    if !process_alive(pid) {
        fs::remove_file(&pid_path)
            .with_context(|| format!("failed to remove stale PID file {}", pid_path.display()))?;
        println!("{service} is not running. Removed stale PID file.");
        return Ok(());
    }

    send_signal(pid, Signal::SIGTERM)
        .with_context(|| format!("failed to send SIGTERM to {service} with PID {pid}"))?;

    if wait_until_stopped(pid) {
        remove_pid_file(&pid_path)?;
        println!("stopped {service}");
        return Ok(());
    }

    send_signal(pid, Signal::SIGKILL)
        .with_context(|| format!("failed to send SIGKILL to {service} with PID {pid}"))?;

    if wait_until_stopped(pid) {
        remove_pid_file(&pid_path)?;
        println!("force stopped {service}");
        return Ok(());
    }

    println!("{service} did not stop after SIGKILL; leaving PID file in place.");
    Ok(())
}

pub fn service_status(service: &Service, paths: &PresidePaths) -> Result<ServiceStatus> {
    let pid_path = paths.pid_path(&service.name);
    let log_path = paths.log_path(&service.name);
    let pid = read_pid(&pid_path)?;
    let alive = pid.is_some_and(process_alive);
    let listening = port_listening(service.port);

    Ok(ServiceStatus {
        pid_path,
        log_path,
        pid,
        alive,
        listening,
    })
}

pub fn print_logs(service: &str, paths: &PresidePaths) -> Result<()> {
    let log_path = paths.log_path(service);
    println!("log: {}", log_path.display());

    if !log_path.exists() {
        println!("No log file found.");
        return Ok(());
    }

    let mut file = File::open(&log_path)
        .with_context(|| format!("failed to open log file {}", log_path.display()))?;
    let len = file.metadata()?.len();
    let start = len.saturating_sub(8 * 1024);
    file.seek(SeekFrom::Start(start))?;

    let mut contents = String::new();
    file.read_to_string(&mut contents)?;

    if !contents.is_empty() {
        print!("{contents}");
    }

    Ok(())
}

pub fn service_url(service: &Service) -> String {
    match &service.domain {
        Some(domain) => format!("http://{domain}"),
        None => format!("http://localhost:{}", service.port),
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

    command
}

fn expand_dir(service: &Service) -> Result<PathBuf> {
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

fn open_log(path: &PathBuf) -> Result<File> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create log directory {}", parent.display()))?;
    }

    OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .with_context(|| format!("failed to open log file {}", path.display()))
}

fn read_pid(path: &PathBuf) -> Result<Option<i32>> {
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

fn remove_pid_file(path: &PathBuf) -> Result<()> {
    fs::remove_file(path)
        .with_context(|| format!("failed to remove PID file after stop {}", path.display()))
}

fn process_is_zombie(pid: i32) -> bool {
    let Ok(output) = Command::new("ps")
        .args(["-p", &pid.to_string(), "-o", "stat="])
        .output()
    else {
        return false;
    };

    if !output.status.success() {
        return false;
    }

    String::from_utf8_lossy(&output.stdout).contains('Z')
}

fn port_listening(port: u16) -> bool {
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    TcpStream::connect_timeout(&addr, Duration::from_millis(100)).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn service(domain: Option<&str>) -> Service {
        Service {
            name: "personal".to_string(),
            dir: ".".to_string(),
            command: "true".to_string(),
            port: 8712,
            domain: domain.map(str::to_string),
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
}
