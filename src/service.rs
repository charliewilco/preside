use anyhow::Result;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ServiceConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dir: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Service {
    pub name: String,
    pub dir: String,
    pub command: String,
    pub port: u16,
    pub domain: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceField {
    Dir,
    Command,
    Port,
    Domain,
}

impl ServiceField {
    pub fn parse(value: &str) -> Result<Self, ServiceError> {
        match value {
            "dir" => Ok(Self::Dir),
            "command" => Ok(Self::Command),
            "port" => Ok(Self::Port),
            "domain" => Ok(Self::Domain),
            _ => Err(ServiceError::InvalidField(value.to_string())),
        }
    }
}

impl ServiceConfig {
    pub fn set(&mut self, field: ServiceField, value: &str) -> Result<(), ServiceError> {
        match field {
            ServiceField::Dir => {
                validate_required_value("dir", value)?;
                self.dir = Some(value.to_string());
            }
            ServiceField::Command => {
                validate_required_value("command", value)?;
                self.command = Some(value.to_string());
            }
            ServiceField::Port => {
                self.port = Some(
                    value
                        .parse::<u16>()
                        .map_err(|_| ServiceError::InvalidPort(value.to_string()))?,
                );
            }
            ServiceField::Domain => {
                validate_domain(value)?;
                self.domain = Some(value.to_string());
            }
        }

        Ok(())
    }

    pub fn unset(&mut self, field: ServiceField) {
        match field {
            ServiceField::Dir => self.dir = None,
            ServiceField::Command => self.command = None,
            ServiceField::Port => self.port = None,
            ServiceField::Domain => self.domain = None,
        }
    }

    pub fn validate(&self, name: &str) -> Result<Service, ServiceError> {
        validate_service_name(name)?;

        Ok(Service {
            name: name.to_string(),
            dir: self
                .dir
                .clone()
                .ok_or_else(|| ServiceError::MissingField(name.to_string(), "dir"))?,
            command: self
                .command
                .clone()
                .ok_or_else(|| ServiceError::MissingField(name.to_string(), "command"))?,
            port: self
                .port
                .ok_or_else(|| ServiceError::MissingField(name.to_string(), "port"))?,
            domain: self.domain.clone(),
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

#[derive(Debug, Error)]
pub enum ServiceError {
    #[error("service '{0}' is not configured")]
    MissingService(String),
    #[error("service '{0}' is missing required field '{1}'")]
    MissingField(String, &'static str),
    #[error("invalid service name '{0}'. use letters, numbers, dots, dashes, or underscores, and start with a letter or number")]
    InvalidName(String),
    #[error("invalid field '{0}'. expected one of: dir, command, port, domain")]
    InvalidField(String),
    #[error("invalid port '{0}'. expected a number from 0 to 65535")]
    InvalidPort(String),
    #[error("invalid domain '{0}'. use a host name without a scheme, path, spaces, or port")]
    InvalidDomain(String),
    #[error("'{0}' cannot be empty")]
    EmptyValue(&'static str),
}
