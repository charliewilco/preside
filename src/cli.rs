use clap::{Parser, Subcommand};
use clap_complete::Shell;

use crate::service::Readiness;

#[derive(Debug, Parser)]
#[command(
    name = "pp",
    version,
    about = "Manage named local development services"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    List {
        #[arg(long)]
        json: bool,
    },
    Add {
        service: String,
        #[arg(long)]
        dir: String,
        #[arg(long)]
        command: String,
        #[arg(long)]
        port: u16,
        #[arg(long)]
        domain: Option<String>,
        #[arg(long)]
        readiness: Option<Readiness>,
        #[arg(long = "health-path")]
        health_path: Option<String>,
        #[arg(long = "env")]
        env: Vec<String>,
    },
    #[command(alias = "r")]
    Run {
        service: String,
        #[arg(short = 'd', long)]
        detach: bool,
    },
    #[command(alias = "i")]
    Info {
        service: String,
        #[arg(long)]
        json: bool,
    },
    Set {
        service: String,
        field: String,
        value: String,
    },
    Unset {
        service: String,
        field: String,
    },
    Remove {
        service: String,
    },
    Rename {
        old: String,
        new: String,
    },
    Doctor {
        service: Option<String>,
        #[arg(long)]
        fix: bool,
        #[arg(long)]
        json: bool,
    },
    Edit,
    Path {
        #[arg(long)]
        state: bool,
        #[arg(long)]
        log: Option<String>,
        #[arg(long)]
        pid: Option<String>,
        #[arg(long = "service-state")]
        service_state: Option<String>,
    },
    Completions {
        shell: Shell,
    },
    #[command(name = "__complete", hide = true)]
    Complete {
        topic: String,
    },
    Open {
        service: String,
    },
    Stop {
        service: String,
    },
    Logs {
        service: String,
        #[arg(short = 'n', long, default_value_t = 100)]
        lines: usize,
        #[arg(short = 'f', long)]
        follow: bool,
        #[arg(long)]
        clear: bool,
    },
    Status {
        service: String,
        #[arg(long)]
        json: bool,
    },
}
