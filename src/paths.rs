use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use directories::BaseDirs;

#[derive(Debug, Clone)]
pub struct PresidePaths {
    pub config_path: PathBuf,
    pub state_dir: PathBuf,
    pub pid_dir: PathBuf,
    pub log_dir: PathBuf,
    pub service_state_dir: PathBuf,
}

impl PresidePaths {
    pub fn discover() -> Result<Self> {
        let base_dirs = BaseDirs::new().context("could not determine home directory")?;
        Ok(Self::from_home(base_dirs.home_dir()))
    }

    pub fn from_home(home: impl AsRef<Path>) -> Self {
        let home = home.as_ref();
        let state_dir = home.join(".local/state/preside");

        Self {
            config_path: config_path(home),
            pid_dir: state_dir.join("pids"),
            log_dir: state_dir.join("logs"),
            service_state_dir: state_dir.join("services"),
            state_dir,
        }
    }

    pub fn pid_path(&self, service: &str) -> PathBuf {
        self.pid_dir.join(format!("{service}.pid"))
    }

    pub fn log_path(&self, service: &str) -> PathBuf {
        self.log_dir.join(format!("{service}.log"))
    }

    pub fn service_state_path(&self, service: &str) -> PathBuf {
        self.service_state_dir.join(format!("{service}.toml"))
    }

    pub fn ensure_state_dirs(&self) -> Result<()> {
        std::fs::create_dir_all(&self.pid_dir).with_context(|| {
            format!("failed to create PID directory {}", self.pid_dir.display())
        })?;
        std::fs::create_dir_all(&self.log_dir).with_context(|| {
            format!("failed to create log directory {}", self.log_dir.display())
        })?;
        std::fs::create_dir_all(&self.service_state_dir).with_context(|| {
            format!(
                "failed to create service state directory {}",
                self.service_state_dir.display()
            )
        })?;
        Ok(())
    }
}

fn config_path(home: &Path) -> PathBuf {
    let candidates = [
        home.join(".config/preside/config.toml"),
        home.join(".config/preside.toml"),
        home.join(".preside.toml"),
    ];

    candidates
        .iter()
        .find(|path| path.exists())
        .cloned()
        .unwrap_or_else(|| candidates[0].clone())
}
