use anyhow::Result;
use chrono::{DateTime, Utc};
use diesel::SqliteConnection;

use crate::db::{
    get_most_recent_record, insert_record, query_records, set_record_end_timestamp, upsert_task,
};

pub struct Records<'a> {
    db: &'a mut SqliteConnection,
}

impl<'a> Records<'a> {
    pub fn new(db: &'a mut SqliteConnection) -> Self {
        Self { db }
    }

    pub fn complete_last_record(
        &mut self,
        end_date: DateTime<Utc>,
        start_date: Option<DateTime<Utc>>,
    ) -> Result<Vec<Record>> {
        let last_record = get_most_recent_record(self.db, end_date)?;
        let mut records = Vec::new();
        match last_record {
            None => {}
            Some((record, (task, project))) => {
                match record.ended_at.filter(|date| date <= &end_date) {
                    Some(_) => {}
                    None => {
                        set_record_end_timestamp(self.db, record.id, end_date)?;
                        records.push(Record {
                            task: task.name.clone(),
                            project: project.clone().map(|p| p.name),
                            started_at: record.started_at,
                            ended_at: Some(end_date),
                        })
                    }
                }

                if let Some(start_date) = start_date {
                    match record.ended_at.filter(|date| date <= &start_date) {
                        Some(_) => {}
                        None => {
                            let record =
                                insert_record(self.db, task.id, start_date, record.ended_at)?;
                            records.push(Record {
                                task: task.name,
                                project: project.map(|p| p.name),
                                started_at: start_date,
                                ended_at: record.ended_at,
                            })
                        }
                    }
                }
            }
        }

        Ok(records)
    }

    pub fn add_record(
        &mut self,
        task_name: &str,
        start_date: DateTime<Utc>,
        end_date: Option<DateTime<Utc>>,
    ) -> Result<Record> {
        let (task, project_name) = upsert_task(self.db, task_name)?;
        let record = insert_record(self.db, task.id, start_date, end_date)?;

        Ok(Record {
            task: task.name,
            project: project_name,
            started_at: record.started_at,
            ended_at: record.ended_at,
        })
    }

    pub fn list_records(&mut self) -> Result<Vec<Record>> {
        let records = query_records(self.db)?
            .map(|row| {
                row.map(|(record, (task, project))| Record {
                    task: task.name,
                    project: project.map(|p| p.name),
                    started_at: record.started_at,
                    ended_at: record.ended_at,
                })
                .map_err(|err| anyhow::anyhow!(err))
            })
            .collect::<Result<Vec<Record>>>()?;

        Ok(records)
    }
}

#[derive(Debug)]
pub struct Record {
    pub task: String,
    pub project: Option<String>,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use crate::db::establish_connection;

    use super::*;

    fn dt(time: &str) -> DateTime<Utc> {
        let mut parts = time.split(":");
        let hour = parts.next().unwrap().parse().unwrap();
        let min = parts.next().unwrap().parse().unwrap();
        let sec = parts.next().unwrap().parse().unwrap();
        Utc.with_ymd_and_hms(2024, 5, 12, hour, min, sec).unwrap()
    }

    #[test]
    fn add_record_adds_a_new_record_and_task() {
        let mut conn = establish_connection(":memory:").unwrap();
        let mut records = Records::new(&mut conn);
        let record = records
            .add_record("hello, world", dt("10:00:00"), None)
            .unwrap();
        assert_eq!(record.task, "hello, world");
        assert_eq!(record.started_at, dt("10:00:00"));
        assert_eq!(record.ended_at, None);

        let record_list = records.list_records().unwrap();
        assert_eq!(record_list.len(), 1);
        assert_eq!(record_list[0].task, "hello, world");
    }

    #[test]
    fn adds_record_with_explicit_end_date() {
        let mut conn = establish_connection(":memory:").unwrap();
        let mut records = Records::new(&mut conn);
        let record = records
            .add_record("hello, world", dt("10:00:00"), Some(dt("11:00:00")))
            .unwrap();
        assert_eq!(record.task, "hello, world");
        assert_eq!(record.started_at, dt("10:00:00"));
        assert_eq!(record.ended_at, Some(dt("11:00:00")));

        let record_list = records.list_records().unwrap();
        assert_eq!(record_list.len(), 1);
        assert_eq!(record_list[0].task, "hello, world");
    }

    #[test]
    fn complete_last_record_updates_most_recent_unfinished_record() {
        let mut conn = establish_connection(":memory:").unwrap();
        let mut records = Records::new(&mut conn);
        records
            .add_record("hello, world", dt("10:00:00"), None)
            .unwrap();

        let recs = &records.complete_last_record(dt("11:00:00"), None).unwrap();
        assert_eq!(recs[0].task, "hello, world");
        assert_eq!(recs[0].ended_at, Some(dt("11:00:00")));
        assert_eq!(recs.len(), 1);
    }

    #[test]
    fn complete_last_record_does_not_update_records_after_the_given_date() {
        let mut conn = establish_connection(":memory:").unwrap();
        let mut records = Records::new(&mut conn);
        records.add_record("abc", dt("10:00:00"), None).unwrap();
        records.add_record("def", dt("12:00:00"), None).unwrap();

        let recs = &records.complete_last_record(dt("11:00:00"), None).unwrap();
        assert_eq!(recs[0].task, "abc");
        assert_eq!(recs[0].ended_at, Some(dt("11:00:00")));
        assert_eq!(recs.len(), 1);
    }

    #[test]
    fn complete_last_record_ignores_dates_that_have_finished_before_the_given_date() {
        let mut conn = establish_connection(":memory:").unwrap();
        let mut records = Records::new(&mut conn);
        records
            .add_record("abc", dt("10:00:00"), Some(dt("11:00:00")))
            .unwrap();

        let record = records.complete_last_record(dt("11:30:00"), None).unwrap();
        assert!(record.is_empty());
    }

    #[test]
    fn complete_last_record_truncates_records_that_finish_after_the_given_date() {
        let mut conn = establish_connection(":memory:").unwrap();
        let mut records = Records::new(&mut conn);
        records
            .add_record("abc", dt("10:00:00"), Some(dt("11:30:00")))
            .unwrap();

        let recs = &records.complete_last_record(dt("11:00:00"), None).unwrap();
        assert_eq!(recs[0].task, "abc");
        assert_eq!(recs[0].ended_at, Some(dt("11:00:00")));
        assert_eq!(recs.len(), 1);
    }

    #[test]
    fn complete_last_record_splits_record_into_two_if_dates_passed_are_inside_the_recorded_date_range(
    ) {
        let mut conn = establish_connection(":memory:").unwrap();
        let mut records = Records::new(&mut conn);
        records
            .add_record("abc", dt("10:00:00"), Some(dt("15:00:00")))
            .unwrap();

        let record = records
            .complete_last_record(dt("11:00:00"), Some(dt("12:00:00")))
            .unwrap();
        assert_eq!(record[0].task, "abc");
        assert_eq!(record[0].started_at, dt("10:00:00"));
        assert_eq!(record[0].ended_at, Some(dt("11:00:00")));
        assert_eq!(record[1].task, "abc");
        assert_eq!(record[1].started_at, dt("12:00:00"));
        assert_eq!(record[1].ended_at, Some(dt("15:00:00")));
        assert_eq!(record.len(), 2);
    }

    #[test]
    fn complete_last_record_splits_record_into_two_if_original_date_has_no_end_and_completed_record_does(
    ) {
        let mut conn = establish_connection(":memory:").unwrap();
        let mut records = Records::new(&mut conn);
        records.add_record("abc", dt("10:00:00"), None).unwrap();

        let record = records
            .complete_last_record(dt("11:00:00"), Some(dt("12:00:00")))
            .unwrap();
        assert_eq!(record[0].task, "abc");
        assert_eq!(record[0].started_at, dt("10:00:00"));
        assert_eq!(record[0].ended_at, Some(dt("11:00:00")));
        assert_eq!(record[1].task, "abc");
        assert_eq!(record[1].started_at, dt("12:00:00"));
        assert_eq!(record[1].ended_at, None);
        assert_eq!(record.len(), 2);
    }
}
