use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::service::{
    validate_service_name, Readiness, Service, ServiceConfig, ServiceError, ServiceField,
    ServiceKind, UrlMode,
};

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
        crate::storage::atomic_write(path, contents.as_bytes())
            .with_context(|| format!("failed to write config at {}", path.display()))
    }

    pub fn set_field(&mut self, service: &str, field: &str, value: &str) -> Result<()> {
        validate_service_name(service)?;
        let field = ServiceField::parse(field)?;
        let service = self.services.entry(service.to_string()).or_default();
        service.set(field, value)?;
        Ok(())
    }

    pub fn add_service(&mut self, service: AddService) -> Result<()> {
        validate_service_name(&service.name)?;

        if self.services.contains_key(&service.name) {
            return Err(ServiceError::ServiceExists(service.name).into());
        }

        let config = Self::validated_service(&service)?;
        self.services.insert(service.name, config);
        Ok(())
    }

    pub fn update_service(&mut self, service: AddService) -> Result<()> {
        validate_service_name(&service.name)?;
        if !self.services.contains_key(&service.name) {
            return Err(ServiceError::MissingService(service.name).into());
        }
        let config = Self::validated_service(&service)?;
        self.services.insert(service.name, config);
        Ok(())
    }

    fn validated_service(service: &AddService) -> Result<ServiceConfig> {
        let mut config = ServiceConfig::default();
        config.set(ServiceField::parse("dir")?, &service.dir)?;
        config.kind = service.kind;
        config.url_mode = service.url_mode;
        if let Some(command) = &service.command {
            config.set(ServiceField::parse("command")?, command)?;
        }
        if service.kind == ServiceKind::Static && service.port == 0 {
            anyhow::bail!("static sites require a port between 1 and 65535");
        }
        config.set(ServiceField::parse("port")?, &service.port.to_string())?;

        if let Some(domain) = &service.domain {
            config.set(ServiceField::parse("domain")?, domain)?;
        }

        if let Some(readiness) = service.readiness {
            config.set(ServiceField::parse("readiness")?, readiness.as_str())?;
        }

        if let Some(health_path) = &service.health_path {
            config.set(ServiceField::parse("health_path")?, health_path)?;
        }

        for pair in &service.env {
            let Some((key, value)) = pair.split_once('=') else {
                return Err(ServiceError::InvalidEnvAssignment(pair.clone()).into());
            };
            config.set(ServiceField::parse(&format!("env.{key}"))?, value)?;
        }

        config.validate(&service.name)?;
        Ok(config)
    }

    pub fn unset_field(&mut self, service: &str, field: &str) -> Result<()> {
        validate_service_name(service)?;
        let field = ServiceField::parse(field)?;
        let service_config = self
            .services
            .get_mut(service)
            .ok_or_else(|| ServiceError::MissingService(service.to_string()))?;
        service_config.unset(field);
        Ok(())
    }

    pub fn remove_service(&mut self, service: &str) -> Result<()> {
        validate_service_name(service)?;

        if self.services.remove(service).is_none() {
            return Err(ServiceError::MissingService(service.to_string()).into());
        }

        Ok(())
    }

    pub fn rename_service(&mut self, old: &str, new: &str) -> Result<()> {
        validate_service_name(old)?;
        validate_service_name(new)?;

        if self.services.contains_key(new) {
            return Err(ServiceError::ServiceExists(new.to_string()).into());
        }

        let service = self
            .services
            .remove(old)
            .ok_or_else(|| ServiceError::MissingService(old.to_string()))?;
        self.services.insert(new.to_string(), service);

        Ok(())
    }

    pub fn require_service(&self, name: &str) -> Result<Service> {
        validate_service_name(name)?;
        let service = self
            .services
            .get(name)
            .ok_or_else(|| ServiceError::MissingService(name.to_string()))?;

        Ok(service.validate(name)?)
    }
}

#[derive(Debug, Clone)]
pub struct AddService {
    pub name: String,
    pub dir: String,
    pub command: Option<String>,
    pub kind: ServiceKind,
    pub port: u16,
    pub domain: Option<String>,
    pub url_mode: Option<UrlMode>,
    pub readiness: Option<Readiness>,
    pub health_path: Option<String>,
    pub env: Vec<String>,
}
