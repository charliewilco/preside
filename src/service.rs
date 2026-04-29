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
            ServiceField::Dir => self.dir = Some(value.to_string()),
            ServiceField::Command => self.command = Some(value.to_string()),
            ServiceField::Port => {
                self.port = Some(
                    value
                        .parse::<u16>()
                        .map_err(|_| ServiceError::InvalidPort(value.to_string()))?,
                );
            }
            ServiceField::Domain => self.domain = Some(value.to_string()),
        }

        Ok(())
    }

    pub fn validate(&self, name: &str) -> Result<Service, ServiceError> {
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

#[derive(Debug, Error)]
pub enum ServiceError {
    #[error("service '{0}' is not configured")]
    MissingService(String),
    #[error("service '{0}' is missing required field '{1}'")]
    MissingField(String, &'static str),
    #[error("invalid field '{0}'. expected one of: dir, command, port, domain")]
    InvalidField(String),
    #[error("invalid port '{0}'. expected a number from 0 to 65535")]
    InvalidPort(String),
}
