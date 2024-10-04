// SPDX-License-Identifier: MPL-2.0

use std::collections::HashMap;

use anyhow::{anyhow, Result};
use chrono::{Duration, Local, NaiveDate, SubsecRound as _, Utc};
use timesheettool::{
    commands::{Go, Granularity, ListRecords, Stop},
    config::Config,
    parse::{parse_date, parse_relative_date},
    print::print,
    records,
};

pub fn go(config: Config, go: Go) -> Result<()> {
    let mut conn = records::establish_connection(&config.database_path)?;
    let mut recs = records::Records::new(&mut conn);
    let today = Local::now().naive_local().date();
    let start_date = go
        .start
        .map(|dt| parse_date(&dt, &Local, today).ok_or(anyhow!("could not parse start time {dt}")))
        .unwrap_or_else(|| Ok(Utc::now().round_subsecs(0)))?;
    let end_date = go
        .end
        .map(|dt| parse_date(&dt, &Local, today).ok_or(anyhow!("could not parse end time {dt}")))
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

    recs.add_record(&go.name, &go.project, start_date, end_date)?;
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
    let today = Local::now().naive_local().date();
    let end_date = stop
        .end
        .map(|dt| parse_date(&dt, &Local, today).ok_or(anyhow!("could not parse end time {dt}")))
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

    let now = Utc::now();
    let today = Local::now().naive_local().date();
    let start = parse_relative_date(&list_records.since, &Local, today)
        .ok_or(anyhow!("could not parse end time {}", &list_records.since))?;
    let end = parse_relative_date(&list_records.until, &Local, today)
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
        &Local,
        config.time_round_minutes,
    )?;
    Ok(())
}

pub(crate) fn edit(config: Config, edit: timesheettool::commands::Edit) -> Result<()> {
    let mut conn = records::establish_connection(&config.database_path)?;
    let mut recs = records::Records::new(&mut conn);
    let today = Local::now().naive_local().date();

    let start_date = edit
        .start
        .map(|dt| parse_date(&dt, &Local, today).ok_or(anyhow!("could not parse start time {dt}")))
        .transpose()?;
    let end_date = edit
        .end
        .map(|dt| parse_date(&dt, &Local, today).ok_or(anyhow!("could not parse end time {dt}")))
        .transpose()?;
    let task_name = edit.task;

    let record = recs.update_record(
        &edit.record_id,
        start_date,
        end_date,
        task_name.as_deref(),
        None,
    )?;

    log::info!("Record updated: {record:?}");

    Ok(())
}

pub(crate) fn overtime(config: Config, overtime: timesheettool::commands::Overtime) -> Result<()> {
    let mut conn = records::establish_connection(&config.database_path)?;
    let mut recs = records::Records::new(&mut conn);

    let now = Utc::now();

    let mut difference = 0.0;
    let mut seconds_for_day = HashMap::new();
    let mut day = None;
    for record in recs.all_records()? {
        let record = record?;
        let rec_day = record.started_at.with_timezone(&Local).date_naive();
        if Some(rec_day) != day {
            if let Some(day) = day {
                print_overtime(
                    &mut difference,
                    &mut seconds_for_day,
                    &day,
                    overtime.hours,
                    config.time_round_minutes,
                );
            }

            day = Some(rec_day);
        }

        let seconds = record.duration(now).num_seconds();
        seconds_for_day
            .entry(record.project)
            .and_modify(|e| *e += seconds)
            .or_insert(seconds);
    }

    if let Some(day) = day {
        print_overtime(
            &mut difference,
            &mut seconds_for_day,
            &day,
            overtime.hours,
            config.time_round_minutes,
        );
    }

    Ok(())
}

fn print_overtime(
    difference: &mut f64,
    seconds_for_day: &mut HashMap<String, i64>,
    day: &NaiveDate,
    hours_for_day: f64,
    rounding_minutes: u32,
) {
    let hours = seconds_for_day
        .drain()
        .map(|(_, value)| round_to_next(value, rounding_minutes as i64 * 60) as f64)
        .sum::<f64>()
        / (60.0 * 60.0);
    *difference += hours - hours_for_day;
    println!("Hours worked for day {day}: {hours:.2}   (balance: {difference:+.2})");
}

fn round_to_next(value: i64, unit: i64) -> i64 {
    let remainder = value % unit;
    if remainder == 0 {
        value
    } else {
        value + unit - remainder
    }
}
