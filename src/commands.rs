use crate::cli::{Cli, Commands};
use crate::Result;

pub fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::Ping => {
            println!("pong");
            Ok(())
        }
    }
}
