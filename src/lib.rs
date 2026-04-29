pub mod cli;
pub mod config;
pub mod paths;
pub mod process;
pub mod service;

use std::process::ExitCode;

use anyhow::Result;
use clap::Parser;

use crate::cli::{Cli, Commands};
use crate::config::Config;
use crate::paths::PresidePaths;

pub fn run() -> Result<ExitCode> {
    let cli = Cli::parse();
    let paths = PresidePaths::discover()?;
    let mut config = Config::load(&paths.config_path)?;

    match cli.command {
        Commands::List => {
            commands::list(&config);
            Ok(ExitCode::SUCCESS)
        }
        Commands::Run { service, detach } => {
            let service = config.require_service(&service)?;

            if detach {
                process::run_detached(&service, &paths)?;
                Ok(ExitCode::SUCCESS)
            } else {
                process::run_foreground(&service)
            }
        }
        Commands::Info { service } => {
            commands::info(&config, &paths, &service)?;
            Ok(ExitCode::SUCCESS)
        }
        Commands::Set {
            service,
            field,
            value,
        } => {
            config.set_field(&service, &field, &value)?;
            config.save(&paths.config_path)?;
            println!("set {service}.{field}");
            Ok(ExitCode::SUCCESS)
        }
        Commands::Unset { service, field } => {
            config.unset_field(&service, &field)?;
            config.save(&paths.config_path)?;
            println!("unset {service}.{field}");
            Ok(ExitCode::SUCCESS)
        }
        Commands::Remove { service } => {
            config.remove_service(&service)?;
            config.save(&paths.config_path)?;
            println!("removed {service}");
            Ok(ExitCode::SUCCESS)
        }
        Commands::Open { service } => {
            let service = config.require_service(&service)?;
            let url = process::service_url(&service);
            open::that(&url)?;
            println!("{url}");
            Ok(ExitCode::SUCCESS)
        }
        Commands::Stop { service } => {
            process::stop_service(&service, &paths)?;
            Ok(ExitCode::SUCCESS)
        }
        Commands::Logs { service } => {
            process::print_logs(&service, &paths)?;
            Ok(ExitCode::SUCCESS)
        }
        Commands::Status { service } => {
            commands::status(&config, &paths, &service)?;
            Ok(ExitCode::SUCCESS)
        }
    }
}

mod commands {
    use anyhow::Result;

    use crate::config::Config;
    use crate::paths::PresidePaths;
    use crate::process;
    use crate::service::validate_service_name;

    pub fn list(config: &Config) {
        if config.services.is_empty() {
            println!("No services configured.");
            return;
        }

        for (name, service) in &config.services {
            println!(
                "{:<18} {:<7} {:<24} {}",
                name,
                display_optional(service.port.map(|port| port.to_string())),
                display_optional(service.domain.clone()),
                display_optional(service.dir.clone())
            );
        }
    }

    pub fn info(config: &Config, paths: &PresidePaths, name: &str) -> Result<()> {
        let service = config
            .services
            .get(name)
            .ok_or_else(|| crate::service::ServiceError::MissingService(name.to_string()))?;

        println!("name: {name}");
        println!("dir: {}", display_optional(service.dir.clone()));
        println!("command: {}", display_optional(service.command.clone()));
        println!(
            "port: {}",
            display_optional(service.port.map(|port| port.to_string()))
        );
        println!("domain: {}", display_optional(service.domain.clone()));
        println!("config: {}", paths.config_path.display());

        Ok(())
    }

    pub fn status(config: &Config, paths: &PresidePaths, name: &str) -> Result<()> {
        validate_service_name(name)?;

        let service_config = config
            .services
            .get(name)
            .ok_or_else(|| crate::service::ServiceError::MissingService(name.to_string()))?;
        let service = service_config.validate(name).ok();
        let port = service_config.port;
        let status = process::service_status(name, service.as_ref(), port, paths)?;
        let missing_fields = missing_fields(service_config);

        println!("{name}");
        println!(
            "  config: {}",
            if missing_fields.is_empty() {
                "complete".to_string()
            } else {
                format!("incomplete ({})", missing_fields.join(", "))
            }
        );
        println!("  dir: {}", display_optional(service_config.dir.clone()));
        println!(
            "  command: {}",
            display_optional(service_config.command.clone())
        );
        println!(
            "  port: {}",
            display_optional(port.map(|port| port.to_string()))
        );
        println!(
            "  domain: {}",
            display_optional(service_config.domain.clone())
        );
        println!("  url: {}", status.url.as_deref().unwrap_or("-"));
        println!(
            "  status: {}",
            if status.process_alive {
                "running"
            } else {
                "stopped"
            }
        );
        println!(
            "  listening: {}",
            status
                .listening
                .map(|listening| if listening { "yes" } else { "no" })
                .unwrap_or("-")
        );
        println!(
            "  identity: {}",
            if status.identity_matches {
                "matches"
            } else if status.pid.is_some() {
                "unknown"
            } else {
                "-"
            }
        );
        println!(
            "  state matches config: {}",
            status
                .state_matches_config
                .map(|matches| if matches { "yes" } else { "no" })
                .unwrap_or("-")
        );
        println!(
            "  pid: {}",
            status
                .pid
                .map(|pid| pid.to_string())
                .unwrap_or_else(|| "-".to_string())
        );
        println!("  pid file: {}", status.pid_path.display());
        println!("  state file: {}", status.state_path.display());
        println!("  log: {}", status.log_path.display());
        println!(
            "  log modified: {}",
            status
                .log_modified_unix
                .map(|modified| modified.to_string())
                .unwrap_or_else(|| "-".to_string())
        );

        if !status.recent_log_lines.is_empty() {
            println!("  recent logs:");
            for line in &status.recent_log_lines {
                println!("    {line}");
            }
        }

        Ok(())
    }

    fn missing_fields(service: &crate::service::ServiceConfig) -> Vec<&'static str> {
        let mut fields = Vec::new();

        if service.dir.is_none() {
            fields.push("dir");
        }

        if service.command.is_none() {
            fields.push("command");
        }

        if service.port.is_none() {
            fields.push("port");
        }

        fields
    }

    fn display_optional(value: Option<String>) -> String {
        value.unwrap_or_else(|| "-".to_string())
    }
}
