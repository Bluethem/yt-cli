use clap::Parser;
use ytcli::cli::Cli;
use ytcli::commands;

fn main() {
    // Carga `.env` del directorio actual si existe (no falla si no hay archivo).
    let _ = dotenvy::dotenv();

    let cli = Cli::parse();
    if let Err(err) = commands::run(cli) {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}
