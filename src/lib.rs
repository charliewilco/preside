pub mod cli;
pub mod config;
pub mod paths;
pub mod process;
pub mod service;
pub mod static_site;

use std::process::ExitCode;

use crate::cli::{Cli, Commands};
use crate::config::{AddService, Config};
use crate::paths::PresidePaths;
use anyhow::Result;
use clap::{CommandFactory, Parser};
use clap_complete::generate;

pub fn run() -> Result<ExitCode> {
    let cli = Cli::parse();
    if let Commands::ServeStatic { dir, port, domain } = &cli.command {
        static_site::serve(dir, *port, domain.as_deref())?;
        return Ok(ExitCode::SUCCESS);
    }
    let paths = PresidePaths::discover()?;
    let mut config = Config::load(&paths.config_path)?;

    match cli.command {
        Commands::ServeStatic { .. } => unreachable!(),
        Commands::List { json } => {
            commands::list(&config, json)?;
            Ok(ExitCode::SUCCESS)
        }
        Commands::Add {
            service,
            dir,
            command,
            static_site,
            port,
            domain,
            readiness,
            health_path,
            env,
        } => {
            config.add_service(AddService {
                name: service.clone(),
                kind: if static_site {
                    crate::service::ServiceKind::Static
                } else {
                    crate::service::ServiceKind::Command
                },
                dir,
                command,
                port,
                domain,
                readiness,
                health_path,
                env,
            })?;
            config.save(&paths.config_path)?;
            println!("added {service}");
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
        Commands::Info { service, json } => {
            commands::info(&config, &paths, &service, json)?;
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
        Commands::Rename { old, new } => {
            config.rename_service(&old, &new)?;
            config.save(&paths.config_path)?;
            println!("renamed {old} to {new}");
            Ok(ExitCode::SUCCESS)
        }
        Commands::Doctor { service, fix, json } => {
            commands::doctor(&config, &paths, service.as_deref(), fix, json)
        }
        Commands::Edit => {
            commands::edit(&paths)?;
            Ok(ExitCode::SUCCESS)
        }
        Commands::Path {
            state,
            log,
            pid,
            service_state,
        } => {
            commands::path(
                &paths,
                state,
                log.as_deref(),
                pid.as_deref(),
                service_state.as_deref(),
            )?;
            Ok(ExitCode::SUCCESS)
        }
        Commands::Completions { shell } => {
            let mut command = Cli::command();
            generate(shell, &mut command, "pp", &mut std::io::stdout());
            Ok(ExitCode::SUCCESS)
        }
        Commands::Complete { topic } => {
            commands::complete(&config, &topic)?;
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
        Commands::Logs {
            service,
            lines,
            follow,
            clear,
        } => {
            process::print_logs(&service, &paths, lines, follow, clear)?;
            Ok(ExitCode::SUCCESS)
        }
        Commands::Status { service, json } => {
            commands::status(&config, &paths, &service, json)?;
            Ok(ExitCode::SUCCESS)
        }
    }
}

mod commands {
    use std::fs;
    use std::process::Command;
    use std::process::ExitCode;

    use anyhow::{Context, Result};
    use serde_json::json;

    use crate::config::Config;
    use crate::paths::PresidePaths;
    use crate::process;
    use crate::service::{validate_service_name, ServiceConfig};

    pub fn list(config: &Config, json_output: bool) -> Result<()> {
        if json_output {
            println!("{}", serde_json::to_string_pretty(&config.services)?);
            return Ok(());
        }

        if config.services.is_empty() {
            println!("No services configured.");
            return Ok(());
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

        Ok(())
    }

    pub fn info(
        config: &Config,
        paths: &PresidePaths,
        name: &str,
        json_output: bool,
    ) -> Result<()> {
        let service = config
            .services
            .get(name)
            .ok_or_else(|| crate::service::ServiceError::MissingService(name.to_string()))?;

        if json_output {
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "name": name,
                    "service": service,
                    "config_path": paths.config_path,
                }))?
            );
            return Ok(());
        }

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

    pub fn status(
        config: &Config,
        paths: &PresidePaths,
        name: &str,
        json_output: bool,
    ) -> Result<()> {
        validate_service_name(name)?;

        let service_config = config
            .services
            .get(name)
            .ok_or_else(|| crate::service::ServiceError::MissingService(name.to_string()))?;
        let service = service_config.validate(name).ok();
        let port = service_config.port;
        let status = process::service_status(name, service.as_ref(), port, paths)?;
        let missing_fields = missing_fields(service_config);

        if json_output {
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "name": name,
                    "service": service_config,
                    "missing_fields": missing_fields,
                    "status": status,
                }))?
            );
            return Ok(());
        }

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

    pub fn doctor(
        config: &Config,
        paths: &PresidePaths,
        service: Option<&str>,
        fix: bool,
        json_output: bool,
    ) -> Result<ExitCode> {
        if let Some(service) = service {
            if !config.services.contains_key(service) {
                return Err(
                    crate::service::ServiceError::MissingService(service.to_string()).into(),
                );
            }
        }

        let report = doctor_report(config, paths, service, fix)?;

        if json_output {
            println!("{}", serde_json::to_string_pretty(&report)?);
            return Ok(if report.issues == 0 {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            });
        }

        println!("config: {}", paths.config_path.display());
        println!("state: {}", paths.state_dir.display());

        if config.services.is_empty() {
            println!("No services configured.");
            return Ok(ExitCode::from(1));
        }

        for service in &report.services {
            println!();
            println!("{}", service.name);
            for check in &service.checks {
                println!("  {}: {}", check.name, check.message);
            }
            for action in &service.fixes {
                println!("  fixed: {action}");
            }
        }

        println!();
        if report.issues == 0 {
            println!("Doctor found no issues.");
            Ok(ExitCode::SUCCESS)
        } else {
            println!("Doctor found {} issue(s).", report.issues);
            Ok(ExitCode::from(1))
        }
    }

    pub fn complete(config: &Config, topic: &str) -> Result<()> {
        match topic {
            "services" => {
                for service in config.services.keys() {
                    println!("{service}");
                }
                Ok(())
            }
            _ => anyhow::bail!("unknown completion topic '{topic}'"),
        }
    }

    pub fn edit(paths: &PresidePaths) -> Result<()> {
        if let Some(parent) = paths.config_path.parent() {
            fs::create_dir_all(parent)?;
        }

        if !paths.config_path.exists() {
            fs::write(&paths.config_path, "")?;
        }

        let backup_path = paths.config_path.with_extension("toml.bak");
        fs::copy(&paths.config_path, &backup_path).with_context(|| {
            format!(
                "failed to create config backup at {}",
                backup_path.display()
            )
        })?;

        if let Some(editor) = std::env::var_os("VISUAL").or_else(|| std::env::var_os("EDITOR")) {
            let status = Command::new(editor).arg(&paths.config_path).status()?;

            if !status.success() {
                anyhow::bail!("editor exited with status {status}");
            }
        } else {
            open::that(&paths.config_path)?;
        }

        Config::load(&paths.config_path).with_context(|| {
            format!(
                "edited config is invalid; backup saved at {}",
                backup_path.display()
            )
        })?;
        println!("config ok: {}", paths.config_path.display());
        Ok(())
    }

    pub fn path(
        paths: &PresidePaths,
        state: bool,
        log: Option<&str>,
        pid: Option<&str>,
        service_state: Option<&str>,
    ) -> Result<()> {
        let selected = [state, log.is_some(), pid.is_some(), service_state.is_some()]
            .into_iter()
            .filter(|selected| *selected)
            .count();

        if selected > 1 {
            anyhow::bail!("choose only one path option");
        }

        if state {
            println!("{}", paths.state_dir.display());
        } else if let Some(service) = log {
            validate_service_name(service)?;
            println!("{}", paths.log_path(service).display());
        } else if let Some(service) = pid {
            validate_service_name(service)?;
            println!("{}", paths.pid_path(service).display());
        } else if let Some(service) = service_state {
            validate_service_name(service)?;
            println!("{}", paths.service_state_path(service).display());
        } else {
            println!("{}", paths.config_path.display());
        }

        Ok(())
    }

    #[derive(Debug, serde::Serialize)]
    struct DoctorReport {
        config_path: std::path::PathBuf,
        state_dir: std::path::PathBuf,
        issues: usize,
        fixed: usize,
        services: Vec<DoctorServiceReport>,
    }

    #[derive(Debug, serde::Serialize)]
    struct DoctorServiceReport {
        name: String,
        checks: Vec<DoctorCheck>,
        fixes: Vec<String>,
    }

    #[derive(Debug, serde::Serialize)]
    struct DoctorCheck {
        name: &'static str,
        ok: bool,
        message: String,
    }

    fn doctor_report(
        config: &Config,
        paths: &PresidePaths,
        selected_service: Option<&str>,
        fix: bool,
    ) -> Result<DoctorReport> {
        let mut report = DoctorReport {
            config_path: paths.config_path.clone(),
            state_dir: paths.state_dir.clone(),
            issues: 0,
            fixed: 0,
            services: Vec::new(),
        };

        for (name, service_config) in &config.services {
            if selected_service.is_some_and(|selected| selected != name) {
                continue;
            }

            let mut service_report = DoctorServiceReport {
                name: name.clone(),
                checks: Vec::new(),
                fixes: Vec::new(),
            };

            match validate_service_name(name) {
                Ok(()) => service_report.checks.push(ok("name", "ok")),
                Err(error) => {
                    report.issues += 1;
                    service_report.checks.push(issue("name", error.to_string()));
                    report.services.push(service_report);
                    continue;
                }
            }

            let missing_fields = missing_fields(service_config);
            if missing_fields.is_empty() {
                service_report.checks.push(ok("config", "ok"));
            } else {
                report.issues += missing_fields.len();
                service_report.checks.push(issue(
                    "config",
                    format!("missing {}", missing_fields.join(", ")),
                ));
            }

            let service = service_config.validate(name).ok();

            if let Some(service) = &service {
                match process::expand_service_dir(service) {
                    Ok(dir) => service_report
                        .checks
                        .push(ok("dir", format!("ok ({})", dir.display()))),
                    Err(error) => {
                        report.issues += 1;
                        service_report
                            .checks
                            .push(issue("dir", format!("{error:#}")));
                    }
                }
            }

            let status =
                process::service_status(name, service.as_ref(), service_config.port, paths)?;

            if status.pid_file_exists && !status.process_alive {
                report.issues += 1;
                service_report
                    .checks
                    .push(issue("process", "stale PID file"));

                if fix {
                    process::cleanup_runtime_files(name, paths)?;
                    report.fixed += 1;
                    service_report
                        .fixes
                        .push("removed stale PID and state files".to_string());
                }
            } else if status.process_alive && !status.identity_matches {
                report.issues += 1;
                service_report
                    .checks
                    .push(issue("process", "live PID does not match Preside state"));
            } else if status.identity_matches {
                service_report.checks.push(ok("process", "running"));
            } else {
                service_report.checks.push(ok("process", "stopped"));
            }

            if status.state.is_some() && status.state_matches_config == Some(false) {
                report.issues += 1;
                service_report
                    .checks
                    .push(issue("state", "does not match current config"));
            } else if status.state.is_some() {
                service_report.checks.push(ok("state", "ok"));
            } else {
                service_report.checks.push(ok("state", "none"));
            }

            match status.listening {
                Some(true) if status.identity_matches => {
                    service_report.checks.push(ok("port", "listening"));
                }
                Some(true) => {
                    report.issues += 1;
                    service_report
                        .checks
                        .push(issue("port", "in use by another process or unknown state"));
                }
                Some(false) => service_report.checks.push(ok("port", "not listening")),
                None => service_report.checks.push(ok("port", "not configured")),
            }

            report.services.push(service_report);
        }

        Ok(report)
    }

    fn ok(name: &'static str, message: impl Into<String>) -> DoctorCheck {
        DoctorCheck {
            name,
            ok: true,
            message: message.into(),
        }
    }

    fn issue(name: &'static str, message: impl Into<String>) -> DoctorCheck {
        DoctorCheck {
            name,
            ok: false,
            message: message.into(),
        }
    }

    fn missing_fields(service: &ServiceConfig) -> Vec<&'static str> {
        let mut fields = Vec::new();

        if service.dir.is_none() {
            fields.push("dir");
        }

        if service.kind == crate::service::ServiceKind::Command && service.command.is_none() {
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
