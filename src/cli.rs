use clap::{Parser, Subcommand};
use clap_complete::Shell;

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
    List,
    #[command(alias = "r")]
    Run {
        service: String,
        #[arg(short = 'd', long)]
        detach: bool,
    },
    #[command(alias = "i")]
    Info {
        service: String,
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
    },
}
