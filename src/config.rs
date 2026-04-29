use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::service::{Service, ServiceConfig, ServiceError, ServiceField};

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub services: BTreeMap<String, ServiceConfig>,
}

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }

        let contents = fs::read_to_string(path)
            .with_context(|| format!("failed to read config at {}", path.display()))?;
        toml::from_str(&contents)
            .with_context(|| format!("failed to parse config at {}", path.display()))
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("failed to create config directory {}", parent.display())
            })?;
        }

        let contents = toml::to_string_pretty(self).context("failed to serialize config")?;
        fs::write(path, contents)
            .with_context(|| format!("failed to write config at {}", path.display()))
    }

    pub fn set_field(&mut self, service: &str, field: &str, value: &str) -> Result<()> {
        let field = ServiceField::parse(field)?;
        let service = self.services.entry(service.to_string()).or_default();
        service.set(field, value)?;
        Ok(())
    }

    pub fn require_service(&self, name: &str) -> Result<Service> {
        let service = self
            .services
            .get(name)
            .ok_or_else(|| ServiceError::MissingService(name.to_string()))?;

        Ok(service.validate(name)?)
    }
}
