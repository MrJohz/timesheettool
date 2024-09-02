use std::io::Write;

use anyhow::Result;
use chrono::{DateTime, Datelike, Duration, Timelike, Utc};

use crate::{commands::Granularity, records::Record};

pub fn print(
    writer: &mut impl Write,
    now: DateTime<Utc>,
    granularity: Granularity,
    records: Vec<Record>,
) -> Result<()> {
    match granularity {
        Granularity::All => print_granularity_all(writer, now, records)?,
        _ => unimplemented!("not yet implemented - other granularities like {granularity:?}"),
    }
    Ok(())
}

fn print_granularity_all(
    writer: &mut impl Write,
    now: DateTime<Utc>,
    records: Vec<Record>,
) -> Result<()> {
    let mut last_date = None;
    for record in records {
        if Some(record.started_at.date_naive()) != last_date {
            last_date = Some(record.started_at.date_naive());
            print_date(writer, record.started_at)?;
        } else {
            write!(writer, "             ")?;
        }

        write!(writer, "  ")?;
        print_times(writer, record.started_at, record.ended_at)?;

        writeln!(
            writer,
            " {:>14}  ({:5})  {:10}  {}",
            duration(record.duration(now)),
            &record.id[..5],
            record.project.as_deref().unwrap_or(""),
            record.task,
        )?;
    }
    Ok(())
}

fn print_date(writer: &mut impl Write, started_at: DateTime<Utc>) -> Result<()> {
    let weekday = &started_at.weekday().to_string()[..2];
    let date = started_at.format("%e %b '%y");

    write!(writer, "{weekday} {date}")?;
    Ok(())
}

fn duration(mut duration: Duration) -> String {
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

fn print_times(
    writer: &mut impl Write,
    started_at: DateTime<Utc>,
    ended_at: Option<DateTime<Utc>>,
) -> Result<()> {
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
            let day_gap = (ended_at - started_at).num_days();
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
            project: Some("blob".into()),
            started_at: dt("12:23:34"),
            ended_at: Some(dt("13:34:45")),
        };

        let mut buffer = Vec::new();
        print(&mut buffer, dt("14:00:00"), Granularity::All, vec![record]).unwrap();
        let result = String::from_utf8(buffer).unwrap();
        assert_eq!(
            result,
            "Su 12 May '24  12:23:34-13:34:45       1h 11m 11s  (hello)  blob        blub\n"
        );
    }

    #[test]
    fn prints_records_with_granularity_all_if_ended_at_is_none() {
        let record = Record {
            id: "hello".into(),
            task: "blub".into(),
            project: Some("blob".into()),
            started_at: dt("12:23:34"),
            ended_at: None,
        };

        let mut buffer = Vec::new();
        print(&mut buffer, dt("14:00:00"), Granularity::All, vec![record]).unwrap();
        let result = String::from_utf8(buffer).unwrap();
        assert_eq!(
            result,
            "Su 12 May '24  12:23:34-               1h 36m 26s  (hello)  blob        blub\n"
        );
    }

    #[test]
    fn prints_records_with_granularity_all_deduplicating_dates_where_necessary() {
        let records = vec![
            Record {
                id: "hello".into(),
                task: "blub".into(),
                project: Some("blob".into()),
                started_at: dt("12:23:34"),
                ended_at: Some(dt("13:34:45")),
            },
            Record {
                id: "hello".into(),
                task: "blub".into(),
                project: Some("blob".into()),
                started_at: dt("14:45:56"),
                ended_at: None,
            },
        ];

        let mut buffer = Vec::new();
        print(&mut buffer, dt("15:00:00"), Granularity::All, records).unwrap();
        let result = String::from_utf8(buffer).unwrap();
        assert_eq!(
            result,
            "
Su 12 May '24  12:23:34-13:34:45       1h 11m 11s  (hello)  blob        blub
               14:45:56-                   14m 4s  (hello)  blob        blub\n"
                .trim_start()
        );
    }
}
