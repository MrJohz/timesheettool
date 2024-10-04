// SPDX-License-Identifier: MPL-2.0

use anyhow::Result;
use clap::Parser;
use dotenvy::dotenv;
use timesheettool::{
    commands::{Arguments, Commands},
    config,
};

mod commands;

fn main() -> Result<()> {
    dotenv().ok();
    let args = Arguments::parse();

    stderrlog::new()
        .quiet(args.quiet)
        .verbosity(args.verbose as usize + 2)
        .init()?;

    let config = config::load_config(args.config_file);

    match args.command {
        Commands::Go(go) => commands::go(config, go)?,
        Commands::Stop(stop) => commands::stop(config, stop)?,
        Commands::Ls(list_records) => commands::ls(config, list_records)?,
        Commands::Edit(edit) => commands::edit(config, edit)?,
        Commands::Overtime(overtime) => commands::overtime(config, overtime)?,
    }
    Ok(())
}
