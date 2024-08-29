use anyhow::Result;
use chrono::{DateTime, Utc};
use diesel::SqliteConnection;

pub struct Records<'a> {
    _db: &'a mut SqliteConnection,
}

impl<'a> Records<'a> {
    pub fn new(db: &'a mut SqliteConnection) -> Self {
        Self { _db: db }
    }

    pub fn add_record(self, task_name: &str, start_date: DateTime<Utc>) -> Result<()> {
        dbg!(task_name, start_date);
        Ok(())
    }
}
