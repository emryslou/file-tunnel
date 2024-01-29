use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(author, version, about)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    SetConfig {
        #[arg(long)]
        path: String,

        #[arg(long)]
        tunnel_host: String,

        #[arg(long)]
        password: Option<String>,
    },
    ShowConfig {
        #[arg(long)]
        names: Option<Vec<String>>
    },

    Start {},
    Stop {},
    Restart {},
}
