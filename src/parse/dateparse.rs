use std::{str::FromStr, sync::LazyLock};

use chrono::{DateTime, Datelike, Days, NaiveDate, TimeZone, Utc, Weekday};
use regex::{Match, Regex};

static REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?xi)
^ # anchor to start of string

(?: # date part (optional, defaults to current day)
  (?: # date is in ISO format (yyyy-mm-dd)
    (\d{4})-(\d{2})-(\d{2})
    (?:\s*T?\s*) # can be either a T or nothing, with arbitrary whitespace allowed everywhere
  ) | (?: # date is a name referring to a day relative to the local date
    (yesterday | today | monday | tuesday | wednesday | thursday | friday | saturday | sunday)
    (?:\s*) # only whitespace as separator
  )
)?
#time part
(\d{2}):(\d{2})(?::(\d{2}))?

$ # anchor to end of string
",
    )
    .expect("Could not parse Regex")
});

pub fn parse_date<Tz>(date: &str, timezone: &Tz, today: NaiveDate) -> Option<DateTime<Utc>>
where
    Tz: TimeZone,
{
    let captures = REGEX.captures(date.trim())?;

    let today = parse_relative_date(captures.get(4).map(|f| f.as_str()), today)?;

    let date = timezone.with_ymd_and_hms(
        capture_with_default(captures.get(1), today.year()),
        capture_with_default(captures.get(2), today.month()),
        capture_with_default(captures.get(3), today.day()),
        captures[5].parse().unwrap(),
        captures[6].parse().unwrap(),
        capture_with_default(captures.get(7), 0),
    );

    Some(date.latest()?.with_timezone(&Utc))
}

fn parse_relative_date(relation: Option<&str>, today: NaiveDate) -> Option<NaiveDate> {
    match relation {
        None => Some(today),
        Some(day) if day.eq_ignore_ascii_case("today") => Some(today),
        Some(day) if day.eq_ignore_ascii_case("yesterday") => today.pred_opt(),
        Some(day) => {
            let weekday = day.parse().ok()?;
            find_last_day(today, weekday)
        }
    }
}

fn find_last_day(today: NaiveDate, day_of_week: Weekday) -> Option<NaiveDate> {
    let current_day = today.weekday();
    match current_day.days_since(day_of_week) {
        // don't allow user to specify "monday" on a monday,
        // as it is ambiguous if they mean today or last monday
        0 => None,
        n => Some(today - (Days::new(n as u64))),
    }
}

fn capture_with_default<T: FromStr>(m: Option<Match>, default: T) -> T {
    m.map(|m| m.as_str().parse().ok().unwrap())
        .unwrap_or(default)
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;
    use tzfile::Tz;

    use super::*;

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2024, 4, 5).unwrap()
    }

    #[test]
    fn parses_simplified_iso_format() {
        let parsed = parse_date("2022-01-05 01:05:07", &Utc, today()).unwrap();
        assert_eq!(parsed, Utc.with_ymd_and_hms(2022, 1, 5, 1, 5, 7).unwrap());
    }

    #[test]
    fn parses_simplified_iso_format_with_excess_whitespace() {
        let parsed = parse_date("    2022-01-05    01:05:07    ", &Utc, today()).unwrap();
        assert_eq!(parsed, Utc.with_ymd_and_hms(2022, 1, 5, 1, 5, 7).unwrap());
    }

    #[test]
    fn parses_simplified_iso_format_with_lowercase_t() {
        let parsed = parse_date("2022-01-05 t 01:05:07", &Utc, today()).unwrap();
        assert_eq!(parsed, Utc.with_ymd_and_hms(2022, 1, 5, 1, 5, 7).unwrap());
    }

    #[test]
    fn parses_simplified_iso_format_with_uppercase_t() {
        let parsed = parse_date("2022-01-05T01:05:07", &Utc, today()).unwrap();
        assert_eq!(parsed, Utc.with_ymd_and_hms(2022, 1, 5, 1, 5, 7).unwrap());
    }

    #[test]
    fn parses_simplified_iso_format_from_given_timezone() {
        let parsed = parse_date(
            "2022-01-05 01:05:00",
            &&Tz::named("Europe/Berlin").unwrap(),
            today(),
        )
        .unwrap();
        assert_eq!(parsed, Utc.with_ymd_and_hms(2022, 1, 5, 0, 5, 0).unwrap());
    }

    #[test]
    fn seconds_defaults_to_zero_if_not_provided() {
        let parsed = parse_date("2022-01-05 01:05", &Utc, today()).unwrap();
        assert_eq!(parsed, Utc.with_ymd_and_hms(2022, 1, 5, 1, 5, 0).unwrap());
    }

    #[test]
    fn can_leave_aside_date_part_to_get_current_date() {
        let parsed = parse_date("01:05:00", &Utc, today()).unwrap();
        assert_eq!(parsed, Utc.with_ymd_and_hms(2024, 4, 5, 1, 5, 0).unwrap())
    }

    #[test]
    fn today_represents_the_current_date() {
        let parsed = parse_date("today 01:05:00", &Utc, today()).unwrap();
        assert_eq!(parsed, Utc.with_ymd_and_hms(2024, 4, 5, 1, 5, 0).unwrap())
    }

    #[test]
    fn yesterday_represents_the_day_before_today() {
        let parsed = parse_date("yesterday 01:05:00", &Utc, today()).unwrap();
        assert_eq!(parsed, Utc.with_ymd_and_hms(2024, 4, 4, 1, 5, 0).unwrap())
    }

    #[test]
    fn parsing_relative_is_case_insensitive() {
        let parsed = parse_date("YEstERdaY 01:05:00", &Utc, today()).unwrap();
        assert_eq!(parsed, Utc.with_ymd_and_hms(2024, 4, 4, 1, 5, 0).unwrap())
    }

    #[test]
    fn allows_relative_parse_with_weekdays() {
        let parsed = parse_date("tuesday 01:05:00", &Utc, today()).unwrap();
        assert_eq!(parsed, Utc.with_ymd_and_hms(2024, 4, 2, 1, 5, 0).unwrap())
    }

    #[test]
    fn doesnt_allow_parsing_relative_date_with_current_day_of_week() {
        let parsed = parse_date("friday 01:05:00", &Utc, today());
        assert_eq!(parsed, None);
    }
}
