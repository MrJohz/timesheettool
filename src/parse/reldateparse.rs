use std::sync::LazyLock;

use chrono::{DateTime, Datelike, Days, Months, NaiveDate, TimeZone as _, Utc};
use regex::Regex;
use tzfile::Tz;

static REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?xi)
^ # anchor to start of string

(\d+)
\s*
(?:
  (?: # days
    (d)(?:ay|ays)?
  ) | (?: # weeks
    (w)(?:k|eek|ks|eeks)?
  ) | (?: # months
    (m)(?:o|onth|os|onths)?
  ) | (?: # years
    (y)(?:r|e|ear|rs|es|ears)?
  )
)
$ # anchor to end of string
",
    )
    .expect("Could not parse Regex")
});

pub fn parse_relative_date(date: &str, timezone: &Tz, today: NaiveDate) -> Option<DateTime<Utc>> {
    let date = date.trim();
    if date.eq_ignore_ascii_case("now") {
        let tomorrow = today.succ_opt()?;
        return start_of_day(timezone, tomorrow);
    }

    let captures = REGEX.captures(date)?;
    let count = captures[1].parse::<u32>().ok()?.saturating_sub(1);
    if captures.get(2).is_some() {
        let start_date = today - Days::new(count as u64);
        start_of_day(timezone, start_date)
    } else if captures.get(3).is_some() {
        let day_of_week = today.weekday();
        let week_start = today - Days::new(day_of_week.num_days_from_monday().into());
        let start_date = week_start - Days::new((count * 7).into());
        start_of_day(timezone, start_date)
    } else if captures.get(4).is_some() {
        let start_date = (today - Months::new(count)).with_day(1)?;
        start_of_day(timezone, start_date)
    } else if captures.get(5).is_some() {
        let start_date = today
            .with_day(1)?
            .with_month(1)?
            .with_year(today.year() - count as i32)?;
        start_of_day(timezone, start_date)
    } else {
        None
    }
}

fn start_of_day(timezone: &Tz, day: NaiveDate) -> Option<DateTime<Utc>> {
    let start = timezone
        .with_ymd_and_hms(day.year(), day.month(), day.day(), 0, 0, 0)
        .earliest()?
        .with_timezone(&Utc);
    Some(start)
}

#[cfg(test)]
mod tests {
    use chrono::Weekday;

    use super::*;

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2024, 4, 5).unwrap()
    }

    #[test]
    fn parse_relative_date_returns_start_of_next_day_if_passed_now() {
        let result = parse_relative_date("now", &Tz::named("Etc/UTC").unwrap(), today()).unwrap();
        assert_eq!(result, Utc.with_ymd_and_hms(2024, 4, 6, 0, 0, 0).unwrap());
    }

    #[test]
    fn parse_relative_date_returns_start_of_next_day_in_other_timezone_if_passed_now() {
        let result =
            parse_relative_date("now", &Tz::named("Europe/Berlin").unwrap(), today()).unwrap();
        assert_eq!(result, Utc.with_ymd_and_hms(2024, 4, 5, 22, 0, 0).unwrap());
    }

    #[test]
    fn parse_relative_date_returns_start_of_day_when_passed_1_day() {
        let result = parse_relative_date("1 day", &Tz::named("Etc/UTC").unwrap(), today()).unwrap();
        assert_eq!(result, Utc.with_ymd_and_hms(2024, 4, 5, 0, 0, 0).unwrap());
    }

    #[test]
    fn parse_relative_date_returns_start_of_day_when_passed_0_day() {
        let result = parse_relative_date("0 day", &Tz::named("Etc/UTC").unwrap(), today()).unwrap();
        assert_eq!(result, Utc.with_ymd_and_hms(2024, 4, 5, 0, 0, 0).unwrap());
    }

    #[test]
    fn parse_relative_date_returns_start_of_prev_day_when_passed_2_days() {
        let result = parse_relative_date("2 day", &Tz::named("Etc/UTC").unwrap(), today()).unwrap();
        assert_eq!(result, Utc.with_ymd_and_hms(2024, 4, 4, 0, 0, 0).unwrap());
    }

    #[test]
    fn parse_relative_date_returns_start_of_month_when_passed_1m() {
        let result = parse_relative_date("1m", &Tz::named("Etc/UTC").unwrap(), today()).unwrap();
        assert_eq!(result, Utc.with_ymd_and_hms(2024, 4, 1, 0, 0, 0).unwrap());
    }

    #[test]
    fn parse_relative_date_returns_start_of_month_when_passed_4m() {
        let result = parse_relative_date("4m", &Tz::named("Etc/UTC").unwrap(), today()).unwrap();
        assert_eq!(result, Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap());
    }

    #[test]
    fn parse_relative_date_returns_start_of_year_when_passed_1y() {
        let result = parse_relative_date("1y", &Tz::named("Etc/UTC").unwrap(), today()).unwrap();
        assert_eq!(result, Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap());
    }

    #[test]
    fn parse_relative_date_returns_start_of_year_when_passed_5y() {
        let result = parse_relative_date("5y", &Tz::named("Etc/UTC").unwrap(), today()).unwrap();
        assert_eq!(result, Utc.with_ymd_and_hms(2020, 1, 1, 0, 0, 0).unwrap());
    }

    #[test]
    fn cannot_parse_combinations_of_multiple_units() {
        let result = parse_relative_date("5y 4m", &Tz::named("Etc/UTC").unwrap(), today());
        assert_eq!(result, None);
    }

    #[test]
    fn parses_week_to_start_of_current_week() {
        let result = parse_relative_date("1w", &Tz::named("Etc/UTC").unwrap(), today()).unwrap();
        assert_eq!(result, Utc.with_ymd_and_hms(2024, 4, 1, 0, 0, 0).unwrap());
        assert_eq!(result.weekday(), Weekday::Mon);
    }

    #[test]
    fn parses_weeks_to_start_of_current_week() {
        let result = parse_relative_date("3w", &Tz::named("Etc/UTC").unwrap(), today()).unwrap();
        assert_eq!(result, Utc.with_ymd_and_hms(2024, 3, 18, 0, 0, 0).unwrap());
        assert_eq!(result.weekday(), Weekday::Mon);
    }
}
