use std::{fmt::Display, io::Write};

use anyhow::Result;
use chrono::{DateTime, Datelike, Duration, TimeZone, Timelike, Utc};

use crate::{commands::Granularity, records::Record};

pub fn print<Tz>(
    writer: &mut impl Write,
    now: DateTime<Utc>,
    granularity: Granularity,
    records: Vec<Record>,
    tz: &Tz,
) -> Result<()>
where
    Tz: TimeZone,
    Tz::Offset: Display,
{
    match granularity {
        Granularity::All => print_granularity_all(writer, now, records, tz)?,
        Granularity::Daily => print_granularity_daily(writer, now, records, tz)?,
        _ => unimplemented!("not yet implemented - other granularities like {granularity:?}"),
    }
    Ok(())
}

fn print_granularity_all<Tz>(
    writer: &mut impl Write,
    now: DateTime<Utc>,
    records: Vec<Record>,
    tz: &Tz,
) -> Result<()>
where
    Tz: TimeZone,
    Tz::Offset: Display,
{
    let mut last_date = None;
    writeln!(
        writer,
        "Date           Times                     Duration  ( id  )  Project     Task"
    )?;
    for record in records {
        let started_at = record.started_at.with_timezone(tz);
        if Some(started_at.date_naive()) != last_date {
            last_date = Some(started_at.date_naive());
            print_date(writer, &started_at)?;
        } else {
            write!(writer, "             ")?;
        }

        write!(writer, "  ")?;
        let ended_at = record.ended_at.map(|e| e.with_timezone(tz));
        print_times(writer, &started_at, &ended_at)?;

        writeln!(
            writer,
            " {:>14}  ({:5})  {:10}  {}",
            duration_to_string(record.duration(now)),
            &record.id[..5],
            record.project,
            record.task,
        )?;
    }
    Ok(())
}

fn print_granularity_daily<Tz>(
    writer: &mut impl Write,
    now: DateTime<Utc>,
    records: Vec<Record>,
    tz: &Tz,
) -> Result<()>
where
    Tz: TimeZone,
    Tz::Offset: Display,
{
    writeln!(writer, "Date               Duration  Project     Task")?;

    let mut records = records.into_iter().peekable();
    while let Some(record) = records.next() {
        let started_at = record.started_at.with_timezone(tz);
        let mut printing_date = Some(&started_at);
        let date = started_at.date_naive();
        let mut records_vec = vec![record];
        while let Some(record) = records.peek() {
            let started_at = record.started_at.with_timezone(tz);
            if started_at.date_naive() != date {
                break;
            }

            records_vec.push(records.next().unwrap());
        }

        records_vec.sort_unstable_by(|a, b| a.task.cmp(&b.task));
        let mut records = records_vec.into_iter().peekable();
        while let Some(record) = records.next() {
            let task = &record.task;
            let mut duration = record.duration(now);
            while let Some(record) = records.peek() {
                if &record.task != task {
                    break;
                }

                duration += record.duration(now);
                records.next();
            }

            print_daily_line(writer, printing_date, duration, record.project, task)?;
            printing_date = None;
        }
    }

    Ok(())
}

fn print_daily_line<Tz>(
    writer: &mut impl Write,
    date: Option<&DateTime<Tz>>,
    duration: Duration,
    project: String,
    task: &str,
) -> Result<()>
where
    Tz: TimeZone,
    Tz::Offset: Display,
{
    match date {
        Some(date) => print_date(writer, date)?,
        None => write!(writer, "             ")?,
    }
    writeln!(
        writer,
        "{:>14}  {:10}  {}",
        duration_to_string(duration),
        project,
        task,
    )?;
    Ok(())
}

fn print_date<Tz>(writer: &mut impl Write, started_at: &DateTime<Tz>) -> Result<()>
where
    Tz: TimeZone,
    Tz::Offset: Display,
{
    let weekday = &started_at.weekday().to_string()[..2];
    let date = started_at.format("%e %b '%y");

    write!(writer, "{weekday} {date}")?;
    Ok(())
}

fn duration_to_string(mut duration: Duration) -> String {
    let mut buf = String::new();
    let days = duration.num_days();
    if days > 0 {
        buf.push_str(&days.to_string());
        buf.push('d');
    }
    duration -= Duration::days(days);
    let hours = duration.num_hours();
    if hours > 0 || !buf.is_empty() {
        if !buf.is_empty() {
            buf.push(' ');
        }
        buf.push_str(&hours.to_string());
        buf.push('h');
    }
    duration -= Duration::hours(hours);
    let minutes = duration.num_minutes();
    if minutes > 0 || !buf.is_empty() {
        if !buf.is_empty() {
            buf.push(' ');
        }
        buf.push_str(&minutes.to_string());
        buf.push('m');
    }
    duration -= Duration::minutes(minutes);
    let seconds = duration.num_seconds();
    if seconds > 0 || !buf.is_empty() {
        if !buf.is_empty() {
            buf.push(' ');
        }
        buf.push_str(&seconds.to_string());
        buf.push('s');
    }

    buf
}

fn print_times<Tz>(
    writer: &mut impl Write,
    started_at: &DateTime<Tz>,
    ended_at: &Option<DateTime<Tz>>,
) -> Result<()>
where
    Tz: TimeZone,
    Tz::Offset: Display,
{
    write!(
        writer,
        "{:02}:{:02}:{:02}-",
        started_at.hour(),
        started_at.minute(),
        started_at.second()
    )?;

    match ended_at {
        Some(ended_at) => {
            write!(
                writer,
                "{:02}:{:02}:{:02}",
                ended_at.hour(),
                ended_at.minute(),
                ended_at.second(),
            )?;
            let end_date = ended_at.date_naive();
            let start_date = started_at.date_naive();
            let day_gap = (end_date - start_date).num_days();
            if day_gap > 0 {
                write!(writer, "+{day_gap}")?;
            } else {
                write!(writer, "  ")?;
            }
        }
        None => {
            write!(writer, "          ")?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone as _;

    use super::*;

    fn dt(time: &str) -> DateTime<Utc> {
        let mut parts = time.split(":");
        let hour = parts.next().unwrap().parse().unwrap();
        let min = parts.next().unwrap().parse().unwrap();
        let sec = parts.next().unwrap().parse().unwrap();
        Utc.with_ymd_and_hms(2024, 5, 12, hour, min, sec).unwrap()
    }

    #[test]
    fn prints_records_with_granularity_all() {
        let record = Record {
            id: "hello".into(),
            task: "blub".into(),
            project: "blob".into(),
            started_at: dt("12:23:34"),
            ended_at: Some(dt("13:34:45")),
        };

        let mut buffer = Vec::new();
        print(
            &mut buffer,
            dt("14:00:00"),
            Granularity::All,
            vec![record],
            &Utc,
        )
        .unwrap();
        let result = String::from_utf8(buffer).unwrap();
        assert_eq!(
            result,
            "
Date           Times                     Duration  ( id  )  Project     Task
Su 12 May '24  12:23:34-13:34:45       1h 11m 11s  (hello)  blob        blub\n"
                .trim_start()
        );
    }

    #[test]
    fn prints_records_with_granularity_all_if_ended_at_is_none() {
        let record = Record {
            id: "hello".into(),
            task: "blub".into(),
            project: "blob".into(),
            started_at: dt("12:23:34"),
            ended_at: None,
        };

        let mut buffer = Vec::new();
        print(
            &mut buffer,
            dt("14:00:00"),
            Granularity::All,
            vec![record],
            &Utc,
        )
        .unwrap();
        let result = String::from_utf8(buffer).unwrap();
        assert_eq!(
            result,
            "
Date           Times                     Duration  ( id  )  Project     Task
Su 12 May '24  12:23:34-               1h 36m 26s  (hello)  blob        blub\n"
                .trim_start()
        );
    }

    #[test]
    fn prints_records_with_granularity_all_deduplicating_dates_where_necessary() {
        let records = vec![
            Record {
                id: "hello".into(),
                task: "blub".into(),
                project: "blob".into(),
                started_at: dt("12:23:34"),
                ended_at: Some(dt("13:34:45")),
            },
            Record {
                id: "hello".into(),
                task: "blub".into(),
                project: "blob".into(),
                started_at: dt("14:45:56"),
                ended_at: None,
            },
        ];

        let mut buffer = Vec::new();
        print(&mut buffer, dt("15:00:00"), Granularity::All, records, &Utc).unwrap();
        let result = String::from_utf8(buffer).unwrap();
        assert_eq!(
            result,
            "
Date           Times                     Duration  ( id  )  Project     Task
Su 12 May '24  12:23:34-13:34:45       1h 11m 11s  (hello)  blob        blub
               14:45:56-                   14m 4s  (hello)  blob        blub\n"
                .trim_start()
        );
    }
}
