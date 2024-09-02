use anyhow::{anyhow, Result};
use chrono::{Duration, SubsecRound as _, Utc};
use timesheettool::{
    commands::{Go, Granularity, ListRecords, Stop},
    config::Config,
    parse::{parse_date, parse_relative_date},
    print::print,
    records,
};
use tzfile::Tz;

pub fn go(config: Config, go: Go) -> Result<()> {
    let mut conn = records::establish_connection(&config.database_path)?;
    let mut recs = records::Records::new(&mut conn);
    let local_tz = Tz::local()?;
    let today = Utc::now().naive_local().date();
    let start_date = go
        .start
        .map(|dt| {
            parse_date(&dt, &local_tz, today).ok_or(anyhow!("could not parse start time {dt}"))
        })
        .unwrap_or_else(|| Ok(Utc::now().round_subsecs(0)))?;
    let end_date = go
        .end
        .map(|dt| parse_date(&dt, &local_tz, today).ok_or(anyhow!("could not parse end time {dt}")))
        .transpose()?;

    if !go.allow_overlap {
        let updated = recs.complete_last_record(start_date, end_date)?;
        if updated.len() == 2 {
            log::info!(
                "Updated previous record for {} to end at {} and start again at {}",
                updated[0].task,
                start_date,
                updated[1].started_at
            );
        } else if updated.len() == 1 {
            log::info!(
                "Updated previous record for {} to end at {}",
                updated[0].task,
                start_date
            )
        }
    }

    recs.add_record(&go.name, start_date, end_date)?;
    match end_date {
        None => log::info!("Added record for {} starting at {start_date}", go.name),
        Some(end_date) => {
            log::info!(
                "Added record for {} starting at {start_date} and ending at {end_date}",
                go.name
            )
        }
    }

    Ok(())
}

pub fn stop(config: Config, stop: Stop) -> Result<()> {
    let mut conn = records::establish_connection(&config.database_path)?;
    let mut recs = records::Records::new(&mut conn);
    let local_tz = Tz::local()?;
    let today = Utc::now().naive_local().date();
    let end_date = stop
        .end
        .map(|dt| parse_date(&dt, &local_tz, today).ok_or(anyhow!("could not parse end time {dt}")))
        .unwrap_or_else(|| Ok(Utc::now().round_subsecs(0)))?;

    let updated = recs.complete_last_record(end_date, None)?;
    if updated.len() == 1 {
        log::info!(
            "Updated previous record for {} to end at {}",
            updated[0].task,
            end_date
        );
    } else {
        log::warn!("No previous record found to be ended at {}", end_date);
    }

    Ok(())
}

pub fn ls(config: Config, list_records: ListRecords) -> Result<()> {
    let mut conn = records::establish_connection(&config.database_path)?;
    let mut recs = records::Records::new(&mut conn);

    let local_tz = Tz::local()?;
    let now = Utc::now();
    let today = now.naive_local().date();
    let start = parse_relative_date(&list_records.since, &local_tz, today)
        .ok_or(anyhow!("could not parse end time {}", &list_records.since))?;
    let end = parse_relative_date(&list_records.until, &local_tz, today)
        .ok_or(anyhow!("could not parse end time {}", &list_records.until))?;

    // TODO: this logic is a bit flimsy.  I think it needs to be based on the unit used by the user in parse_relative_date,
    // i.e. if I write a request in weeks, then I want to see daily granularity, and if I write a request in months then I
    // want to see monthly granularity?
    let granularity = if list_records.granularity != Granularity::Auto {
        list_records.granularity
    } else if end - start <= Duration::days(6) {
        Granularity::All
    } else if end - start <= Duration::weeks(4) {
        Granularity::Daily
    } else if end - start <= Duration::days(60) {
        Granularity::Weekly
    } else {
        Granularity::Monthly
    };

    let mut stdout = std::io::stdout().lock();
    print(
        &mut stdout,
        now,
        granularity,
        recs.list_records(start, end)?,
    )?;
    Ok(())
}

pub(crate) fn edit(config: Config, edit: timesheettool::commands::Edit) -> Result<()> {
    let mut conn = records::establish_connection(&config.database_path)?;
    let mut recs = records::Records::new(&mut conn);
    let local_tz = Tz::local()?;
    let today = Utc::now().naive_local().date();

    let start_date = edit
        .start
        .map(|dt| {
            parse_date(&dt, &local_tz, today).ok_or(anyhow!("could not parse start time {dt}"))
        })
        .transpose()?;
    let end_date = edit
        .end
        .map(|dt| parse_date(&dt, &local_tz, today).ok_or(anyhow!("could not parse end time {dt}")))
        .transpose()?;
    let task_name = edit.task;

    let record = recs.update_record(&edit.record_id, start_date, end_date, task_name.as_deref())?;

    log::info!("Record updated: {record:?}");

    Ok(())
}
