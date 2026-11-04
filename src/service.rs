use std::collections::BTreeMap;

use anyhow::Result;
use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ServiceConfig {
    #[serde(default)]
    pub kind: ServiceKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dir: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub readiness: Option<Readiness>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub health_path: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub env: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Service {
    pub kind: ServiceKind,
    pub name: String,
    pub dir: String,
    pub command: String,
    pub port: u16,
    pub domain: Option<String>,
    pub readiness: Readiness,
    pub health_path: Option<String>,
    pub env: BTreeMap<String, String>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ServiceKind {
    #[default]
    Command,
    Static,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum Readiness {
    Process,
    #[default]
    Port,
    Http,
}

impl Readiness {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Process => "process",
            Self::Port => "port",
            Self::Http => "http",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceField {
    Dir,
    Kind,
    Command,
    Port,
    Domain,
    Readiness,
    HealthPath,
    Env,
}

impl ServiceField {
    pub fn parse(value: &str) -> Result<ParsedServiceField, ServiceError> {
        match value {
            "kind" => Ok(ParsedServiceField::Scalar(Self::Kind)),
            "dir" => Ok(ParsedServiceField::Scalar(Self::Dir)),
            "command" => Ok(ParsedServiceField::Scalar(Self::Command)),
            "port" => Ok(ParsedServiceField::Scalar(Self::Port)),
            "domain" => Ok(ParsedServiceField::Scalar(Self::Domain)),
            "readiness" => Ok(ParsedServiceField::Scalar(Self::Readiness)),
            "health_path" | "health-path" => Ok(ParsedServiceField::Scalar(Self::HealthPath)),
            field if field.starts_with("env.") => {
                let key = field.trim_start_matches("env.");
                validate_env_key(key)?;
                Ok(ParsedServiceField::Env(key.to_string()))
            }
            _ => Err(ServiceError::InvalidField(value.to_string())),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParsedServiceField {
    Scalar(ServiceField),
    Env(String),
}

impl ServiceConfig {
    pub fn set(&mut self, field: ParsedServiceField, value: &str) -> Result<(), ServiceError> {
        match field {
            ParsedServiceField::Scalar(ServiceField::Kind) => {
                self.kind = match value {
                    "command" => ServiceKind::Command,
                    "static" => ServiceKind::Static,
                    _ => return Err(ServiceError::InvalidKind(value.to_string())),
                };
            }
            ParsedServiceField::Scalar(ServiceField::Dir) => {
                validate_required_value("dir", value)?;
                self.dir = Some(value.to_string());
            }
            ParsedServiceField::Scalar(ServiceField::Command) => {
                validate_required_value("command", value)?;
                self.command = Some(value.to_string());
            }
            ParsedServiceField::Scalar(ServiceField::Port) => {
                self.port = Some(
                    value
                        .parse::<u16>()
                        .map_err(|_| ServiceError::InvalidPort(value.to_string()))?,
                );
            }
            ParsedServiceField::Scalar(ServiceField::Domain) => {
                validate_domain(value)?;
                self.domain = Some(value.to_string());
            }
            ParsedServiceField::Scalar(ServiceField::Readiness) => {
                self.readiness = Some(parse_readiness(value)?);
            }
            ParsedServiceField::Scalar(ServiceField::HealthPath) => {
                validate_health_path(value)?;
                self.health_path = Some(value.to_string());
            }
            ParsedServiceField::Scalar(ServiceField::Env) => unreachable!("env requires a key"),
            ParsedServiceField::Env(key) => {
                self.env.insert(key, value.to_string());
            }
        }

        Ok(())
    }

    pub fn unset(&mut self, field: ParsedServiceField) {
        match field {
            ParsedServiceField::Scalar(ServiceField::Kind) => self.kind = ServiceKind::Command,
            ParsedServiceField::Scalar(ServiceField::Dir) => self.dir = None,
            ParsedServiceField::Scalar(ServiceField::Command) => self.command = None,
            ParsedServiceField::Scalar(ServiceField::Port) => self.port = None,
            ParsedServiceField::Scalar(ServiceField::Domain) => self.domain = None,
            ParsedServiceField::Scalar(ServiceField::Readiness) => self.readiness = None,
            ParsedServiceField::Scalar(ServiceField::HealthPath) => self.health_path = None,
            ParsedServiceField::Scalar(ServiceField::Env) => unreachable!("env requires a key"),
            ParsedServiceField::Env(key) => {
                self.env.remove(&key);
            }
        }
    }

    pub fn validate(&self, name: &str) -> Result<Service, ServiceError> {
        validate_service_name(name)?;
        if self.kind == ServiceKind::Static && self.port == Some(0) {
            return Err(ServiceError::InvalidPort("0 (static site)".to_string()));
        }

        Ok(Service {
            kind: self.kind,
            name: name.to_string(),
            dir: self
                .dir
                .clone()
                .ok_or_else(|| ServiceError::MissingField(name.to_string(), "dir"))?,
            command: match self.kind {
                ServiceKind::Static => String::new(),
                ServiceKind::Command => self
                    .command
                    .clone()
                    .ok_or_else(|| ServiceError::MissingField(name.to_string(), "command"))?,
            },
            port: self
                .port
                .ok_or_else(|| ServiceError::MissingField(name.to_string(), "port"))?,
            domain: self.domain.clone(),
            readiness: self.readiness.unwrap_or_default(),
            health_path: self.health_path.clone(),
            env: self.env.clone(),
        })
    }
}

pub fn validate_service_name(name: &str) -> Result<(), ServiceError> {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return Err(ServiceError::InvalidName(name.to_string()));
    };

    if !first.is_ascii_alphanumeric() {
        return Err(ServiceError::InvalidName(name.to_string()));
    }

    if chars.any(|char| !char.is_ascii_alphanumeric() && !matches!(char, '-' | '_' | '.')) {
        return Err(ServiceError::InvalidName(name.to_string()));
    }

    Ok(())
}

fn validate_required_value(field: &'static str, value: &str) -> Result<(), ServiceError> {
    if value.trim().is_empty() {
        return Err(ServiceError::EmptyValue(field));
    }

    Ok(())
}

fn validate_domain(domain: &str) -> Result<(), ServiceError> {
    if domain.trim().is_empty()
        || domain.contains("://")
        || domain.contains('/')
        || domain.chars().any(char::is_whitespace)
        || domain.starts_with('.')
        || domain.ends_with('.')
        || domain.contains("..")
        || domain
            .chars()
            .any(|char| !char.is_ascii_alphanumeric() && !matches!(char, '-' | '.'))
    {
        return Err(ServiceError::InvalidDomain(domain.to_string()));
    }

    Ok(())
}

fn validate_env_key(key: &str) -> Result<(), ServiceError> {
    let mut chars = key.chars();
    let Some(first) = chars.next() else {
        return Err(ServiceError::InvalidEnvKey(key.to_string()));
    };

    if !first.is_ascii_alphabetic() && first != '_' {
        return Err(ServiceError::InvalidEnvKey(key.to_string()));
    }

    if chars.any(|char| !char.is_ascii_alphanumeric() && char != '_') {
        return Err(ServiceError::InvalidEnvKey(key.to_string()));
    }

    Ok(())
}

fn parse_readiness(value: &str) -> Result<Readiness, ServiceError> {
    match value {
        "process" => Ok(Readiness::Process),
        "port" => Ok(Readiness::Port),
        "http" => Ok(Readiness::Http),
        _ => Err(ServiceError::InvalidReadiness(value.to_string())),
    }
}

fn validate_health_path(path: &str) -> Result<(), ServiceError> {
    if path.starts_with('/') && !path.chars().any(char::is_whitespace) {
        return Ok(());
    }

    Err(ServiceError::InvalidHealthPath(path.to_string()))
}

#[derive(Debug, Error)]
pub enum ServiceError {
    #[error("invalid service kind '{0}'. expected command or static")]
    InvalidKind(String),
    #[error("service '{0}' is not configured")]
    MissingService(String),
    #[error("service '{0}' already exists")]
    ServiceExists(String),
    #[error("service '{0}' is missing required field '{1}'")]
    MissingField(String, &'static str),
    #[error("invalid service name '{0}'. use letters, numbers, dots, dashes, or underscores, and start with a letter or number")]
    InvalidName(String),
    #[error("invalid field '{0}'. expected one of: kind, dir, command, port, domain, readiness, health_path, env.<KEY>")]
    InvalidField(String),
    #[error("invalid port '{0}'. expected a number from 0 to 65535")]
    InvalidPort(String),
    #[error("invalid domain '{0}'. use a host name without a scheme, path, spaces, or port")]
    InvalidDomain(String),
    #[error("'{0}' cannot be empty")]
    EmptyValue(&'static str),
    #[error("invalid env key '{0}'. use letters, numbers, and underscores, and start with a letter or underscore")]
    InvalidEnvKey(String),
    #[error("invalid readiness '{0}'. expected one of: process, port, http")]
    InvalidReadiness(String),
    #[error("invalid health path '{0}'. expected a path starting with '/' and no whitespace")]
    InvalidHealthPath(String),
    #[error("invalid env assignment '{0}'. expected KEY=value")]
    InvalidEnvAssignment(String),
}
