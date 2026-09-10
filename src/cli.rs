use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "ytcli", about = "Escucha YouTube desde la terminal")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Placeholder hasta Task 5
    Ping,
}
