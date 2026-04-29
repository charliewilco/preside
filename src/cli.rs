use clap::{Parser, Subcommand};

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
    Open {
        service: String,
    },
    Stop {
        service: String,
    },
    Logs {
        service: String,
    },
    Status {
        service: String,
    },
}
