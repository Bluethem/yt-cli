use crate::cli::{Cli, Commands};
use crate::Result;

pub fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::Search { query, limit } => {
            eprintln!("search: query={query:?} limit={limit} (no implementado)");
            Ok(())
        }
        Commands::Play { target } => {
            eprintln!("play: target={target:?} (no implementado)");
            Ok(())
        }
        Commands::Pause => {
            eprintln!("pause (no implementado)");
            Ok(())
        }
        Commands::Resume => {
            eprintln!("resume (no implementado)");
            Ok(())
        }
        Commands::Stop => {
            eprintln!("stop (no implementado)");
            Ok(())
        }
        Commands::Volume { level } => {
            eprintln!("volume: level={level} (no implementado)");
            Ok(())
        }
        Commands::Status => {
            eprintln!("status (no implementado)");
            Ok(())
        }
        Commands::Now => {
            eprintln!("now (no implementado)");
            Ok(())
        }
    }
}
