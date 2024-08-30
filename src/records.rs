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

    pub fn complete_last_record(&mut self, end_date: DateTime<Utc>) -> Result<Option<Record>> {
        let last_record = get_most_recent_record(self.db)?;
        if let Some((record, (task, project))) = last_record {
            if record.ended_at.is_none() {
                set_record_end_timestamp(self.db, record.id, end_date)?;

                Ok(Some(Record {
                    task: task.name,
                    project: project.map(|p| p.name),
                    started_at: record.started_at,
                    ended_at: Some(end_date),
                }))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    pub fn add_record(&mut self, task_name: &str, start_date: DateTime<Utc>) -> Result<Record> {
        let (task, project_name) = upsert_task(self.db, task_name)?;
        let record = insert_record(self.db, task.id, start_date)?;

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
