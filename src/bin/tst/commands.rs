// SPDX-License-Identifier: MPL-2.0

use std::{collections::HashMap, io::Write, iter::Peekable};

use anyhow::{anyhow, Result};
use chrono::{
    DateTime, Datelike, Duration, DurationRound, Local, NaiveDate, SubsecRound as _, TimeDelta,
    Utc, Weekday,
};
use itertools::Itertools;
use timesheettool::{
    commands::{Go, Granularity, ListRecords, Stop},
    config::Config,
    parse::{parse_date, parse_relative_date},
    print::print,
    records::{self, Record},
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
    let start = parse_relative_date(&list_records.since, &Local, today).ok_or(anyhow!(
        "could not parse start time {}",
        &list_records.since
    ))?;
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

pub(crate) fn times(config: Config, times: timesheettool::commands::Times) -> Result<()> {
    let mut conn = records::establish_connection(&config.database_path)?;
    let mut recs = records::Records::new(&mut conn);

    let now = Utc::now();
    let today = Local::now().naive_local().date();
    let start = parse_relative_date(&times.since, &Local, today)
        .ok_or(anyhow!("could not parse start time {}", &times.since))?;
    let end = parse_relative_date(&times.until, &Local, today)
        .ok_or(anyhow!("could not parse end time {}", &times.until))?;

    let mut stdout = std::io::stdout().lock();
    let days = recs
        .list_records(start, end)?
        .into_iter()
        .chunk_by(|r| r.started_at.with_timezone(&Local).date_naive());

    for (day, records) in &days {
        let mut records = records.peekable();

        let start = records
            .peek()
            .unwrap()
            .started_at
            .duration_trunc(TimeDelta::minutes(15))
            .unwrap();
        let start_local = start.with_timezone(&Local);

        let start_text = start_local.format("%H:%M");

        let (end, pauses) = breaks(records);
        let end = end.map(|last| {
            // There is no `duration_ceil` or similar, but this *should* do the right
            // thing, right?
            (last + TimeDelta::seconds(((60 * 15) / 2) - 1))
                .duration_round(TimeDelta::minutes(15))
                .unwrap()
        });
        let mut hours = end.unwrap_or(now) - start;

        let end = end
            .map(|last| last.with_timezone(&Local).format("%H:%M").to_string())
            .unwrap_or("     ".into());

        let mut pause_sum = TimeDelta::zero();
        let pauses = pauses
            .into_iter()
            .map(|(start, end)| {
                pause_sum += end - start;
                let start = start.with_timezone(&Local).format("%H:%M");
                let end = end.with_timezone(&Local).format("%H:%M");
                format!("{start} - {end}")
            })
            .join(", ");

        hours -= (pause_sum).max(TimeDelta::minutes(30));

        writeln!(
            stdout,
            "{day}: {start_text} - {end}  (hours: {}, breaks: {pauses})",
            format_duration(hours),
        )?;
    }

    Ok(())
}

fn format_duration(delta: TimeDelta) -> String {
    let minutes = delta.num_minutes() % 60;
    let hours = delta.num_minutes() / 60;
    return format!("{hours:0>2}:{minutes:0>2}");
}

fn breaks(
    records: impl Iterator<Item = Record>,
) -> (Option<DateTime<Utc>>, Vec<(DateTime<Utc>, DateTime<Utc>)>) {
    let mut end: Option<DateTime<Utc>> = None;
    let mut pauses = Vec::new();
    for record in records {
        if let Some(end) = end {
            let gap = record.started_at - end;
            if gap > TimeDelta::seconds(60) {
                let gap_start = end.duration_round(TimeDelta::minutes(5)).unwrap();
                let mut gap_end = record
                    .started_at
                    .duration_round(TimeDelta::minutes(5))
                    .unwrap();

                let new_gap = gap_end - gap_start;
                if new_gap < TimeDelta::minutes(30) {
                    gap_end += TimeDelta::minutes(30) - new_gap;
                }
                pauses.push((gap_start, gap_end));
            }
        }
        end = record.ended_at;
    }

    (end, pauses)
}

pub(crate) fn overtime(config: Config, overtime: timesheettool::commands::Overtime) -> Result<()> {
    let mut conn = records::establish_connection(&config.database_path)?;
    let mut recs = records::Records::new(&mut conn);

    let now = Utc::now();
    let today = Local::now().naive_local().date();
    let start = parse_relative_date(&overtime.since, &Local, today)
        .ok_or(anyhow!("could not parse start time {}", &overtime.since))?
        .with_timezone(&Local)
        .date_naive();

    for record in OvertimeIter::new(
        recs.all_records()?,
        overtime.hours,
        config.time_round_minutes,
        now,
    ) {
        let record = record?;
        if record.date < start {
            continue;
        }
        println!(
            "Hours worked for day {}: {:.2} ({:+.2})   (balance: {:+.2})",
            record.date, record.hours_day, record.hours_difference, record.hours_total
        );
    }

    Ok(())
}

struct OvertimeIter<T>
where
    T: Iterator<Item = Result<Record>>,
{
    now: DateTime<Utc>,
    day: Option<NaiveDate>,
    seconds_day: HashMap<String, i64>,
    hours_total: f64,
    hours_for_day: f64,
    rounding_minutes: u32,
    records: Peekable<T>,
    finished: bool,
}

#[derive(Debug)]
struct OvertimeRecord {
    date: NaiveDate,
    hours_day: f64,
    hours_difference: f64,
    hours_total: f64,
}

impl<T> OvertimeIter<T>
where
    T: Iterator<Item = Result<Record>>,
{
    pub fn new(records: T, hours_for_day: f64, rounding_minutes: u32, now: DateTime<Utc>) -> Self {
        Self {
            now,
            hours_for_day,
            rounding_minutes,
            records: records.peekable(),
            day: None,
            seconds_day: HashMap::new(),
            hours_total: 0.0,
            finished: false,
        }
    }
}

impl<T> Iterator for OvertimeIter<T>
where
    T: Iterator<Item = Result<Record>>,
{
    type Item = Result<OvertimeRecord>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }

        let today = self.day;
        loop {
            match self.records.peek() {
                None => {
                    self.finished = true;
                    break;
                }
                Some(Err(_)) => {
                    return Some(Err(self.records.next().unwrap().unwrap_err()));
                }
                Some(Ok(record)) => {
                    let day = record.started_at.with_timezone(&Local).date_naive();
                    if Some(day) != self.day {
                        self.day = Some(day);
                        break;
                    }

                    let seconds = record.duration(self.now).num_seconds();

                    self.seconds_day
                        .entry(record.project.clone())
                        .and_modify(|e| *e += seconds)
                        .or_insert(seconds);
                }
            }
            self.records.next();
        }

        match today {
            None => self.next(),
            Some(day) => {
                let hours_for_day = if matches!(day.weekday(), Weekday::Sat | Weekday::Sun) {
                    0.0
                } else {
                    self.hours_for_day
                };

                let hours = self
                    .seconds_day
                    .drain()
                    .map(|(_, value)| {
                        round_to_next(value, self.rounding_minutes as i64 * 60) as f64
                    })
                    .sum::<f64>()
                    / (60.0 * 60.0);

                let difference = hours - hours_for_day;
                self.hours_total += difference;

                Some(Ok(OvertimeRecord {
                    date: day,
                    hours_day: hours,
                    hours_difference: difference,
                    hours_total: self.hours_total,
                }))
            }
        }
    }
}

fn round_to_next(value: i64, unit: i64) -> i64 {
    let remainder = value % unit;
    if remainder == 0 {
        value
    } else {
        value + unit - remainder
    }
}
