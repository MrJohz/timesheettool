use anyhow::Result;
use chrono::{SubsecRound, Utc};
use clap::Parser;
use dotenvy::dotenv;
use log::info;
use timesheettool::{
    commands::{Arguments, Commands},
    db, records,
};

fn main() -> Result<()> {
    dotenv().ok();
    let args = Arguments::parse();

    stderrlog::new()
        .quiet(args.quiet)
        .verbosity(args.verbose as usize + 2)
        .init()?;

    match args.command {
        Commands::Log(log) => {
            let database_url = std::env::var("DATABASE_URL")?;
            let mut conn = db::establish_connection(&database_url)?;
            let mut recs = records::Records::new(&mut conn);
            let start_date = log.start.unwrap_or_else(Utc::now).round_subsecs(0);
            let end_date = log.end;

            if !log.allow_overlap {
                let updated = recs.complete_last_record(start_date, end_date)?;
                if updated.len() == 2 {
                    info!(
                        "Updated previous record for {} to end at {} and start again at {}",
                        updated[0].task, start_date, updated[1].started_at
                    );
                } else if updated.len() == 1 {
                    info!(
                        "Updated previous record for {} to end at {}",
                        updated[0].task, start_date
                    )
                }
            }

            recs.add_record(&log.name, start_date, end_date)?;
            match end_date {
                None => info!("Added record for {} starting at {start_date}", log.name),
                Some(end_date) => {
                    info!(
                        "Added record for {} starting at {start_date} and ending at {end_date}",
                        log.name
                    )
                }
            }
        }
    }
    Ok(())
}
