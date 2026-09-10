use clap::Parser;
use ytcli::cli::Cli;
use ytcli::commands;

fn main() {
    let cli = Cli::parse();
    if let Err(err) = commands::run(cli) {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}
