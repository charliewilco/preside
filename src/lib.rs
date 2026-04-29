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
            let service_config = config.require_service(&service)?;
            commands::status(&service_config, &paths)?;
            Ok(ExitCode::SUCCESS)
        }
    }
}

mod commands {
    use anyhow::Result;

    use crate::config::Config;
    use crate::paths::PresidePaths;
    use crate::process;
    use crate::service::Service;

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

    pub fn status(service: &Service, paths: &PresidePaths) -> Result<()> {
        let status = process::service_status(service, paths)?;

        println!("{}", service.name);
        println!("  port: {}", service.port);
        println!("  domain: {}", service.domain.as_deref().unwrap_or("-"));
        println!(
            "  status: {}",
            if status.alive { "running" } else { "stopped" }
        );
        println!(
            "  listening: {}",
            if status.listening { "yes" } else { "no" }
        );
        println!(
            "  pid: {}",
            status
                .pid
                .map(|pid| pid.to_string())
                .unwrap_or_else(|| "-".to_string())
        );
        println!("  pid file: {}", status.pid_path.display());
        println!("  log: {}", status.log_path.display());

        Ok(())
    }

    fn display_optional(value: Option<String>) -> String {
        value.unwrap_or_else(|| "-".to_string())
    }
}
