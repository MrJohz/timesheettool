use anyhow::Result;
use clap::Parser;
use dotenvy::dotenv;
use timesheettool::commands::{Arguments, Commands};

mod commands;

fn main() -> Result<()> {
    dotenv().ok();
    let args = Arguments::parse();

    stderrlog::new()
        .quiet(args.quiet)
        .verbosity(args.verbose as usize + 2)
        .init()?;

    match args.command {
        Commands::Go(go) => commands::go(go)?,
        Commands::Stop(stop) => commands::stop(stop)?,
        Commands::Ls(list_records) => commands::ls(list_records)?,
    }
    Ok(())
}
