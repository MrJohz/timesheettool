use anyhow::{anyhow, Result};
use chrono::{SubsecRound, Utc};
use clap::Parser;
use dotenvy::dotenv;
use log::{info, warn};
use timesheettool::{
    commands::{Arguments, Commands},
    dateparse::parse_date,
    db, records,
};
use tzfile::Tz;

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
            let local_tz = Tz::local()?;
            let today = Utc::now().naive_local().date();
            let start_date = log
                .start
                .map(|dt| {
                    parse_date(&dt, &local_tz, today)
                        .ok_or(anyhow!("could not parse start time {dt}"))
                })
                .unwrap_or_else(|| Ok(Utc::now().round_subsecs(0)))?;
            let end_date = log
                .end
                .map(|dt| {
                    parse_date(&dt, &local_tz, today)
                        .ok_or(anyhow!("could not parse end time {dt}"))
                })
                .transpose()?;

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
        Commands::Stop(stop) => {
            let database_url = std::env::var("DATABASE_URL")?;
            let mut conn = db::establish_connection(&database_url)?;
            let mut recs = records::Records::new(&mut conn);
            let local_tz = Tz::local()?;
            let today = Utc::now().naive_local().date();
            let end_date = stop
                .end
                .map(|dt| {
                    parse_date(&dt, &local_tz, today)
                        .ok_or(anyhow!("could not parse end time {dt}"))
                })
                .unwrap_or_else(|| Ok(Utc::now().round_subsecs(0)))?;

            let updated = recs.complete_last_record(end_date, None)?;
            if updated.len() == 1 {
                info!(
                    "Updated previous record for {} to end at {}",
                    updated[0].task, end_date
                );
            } else {
                warn!("No previous record found to be ended at {}", end_date);
            }
        }
    }
    Ok(())
}
