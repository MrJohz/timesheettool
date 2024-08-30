use anyhow::{bail, Result};
use chrono::Utc;
use diesel::upsert::excluded;
use diesel::{prelude::*, sql_query};
use diesel::{Connection, SqliteConnection};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};

pub fn establish_connection(database_url: &str) -> Result<SqliteConnection> {
    log::trace!("Connecting to SQLite DB at {database_url}");
    let mut conn = SqliteConnection::establish(database_url)?;
    sql_query("PRAGMA application_id = 0x9b34493a;PRAGMA foreign_keys = TRUE;PRAGMA optimize;")
        .execute(&mut conn)?;
    log::trace!("Connection to SQLite DB at {database_url} successful");
    run_migrations(&mut conn)?;
    Ok(conn)
}

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

fn run_migrations(db: &mut SqliteConnection) -> Result<()> {
    match db.run_pending_migrations(MIGRATIONS) {
        Ok(migrations) => {
            if !migrations.is_empty() {
                log::trace!(
                    "Ran {} migration(s) to update SQLite DB schema to latest version",
                    migrations.len()
                );
            }
            Ok(())
        }
        Err(_) => anyhow::bail!("Could not update database to the latest version"),
    }
}

#[derive(Queryable, Identifiable, Selectable, Debug, PartialEq, Clone)]
#[diesel(table_name=crate::schema::projects)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Project {
    pub id: i32,
    pub name: String,
}

#[derive(Queryable, Identifiable, Selectable, Associations, Debug, PartialEq)]
#[diesel(table_name=crate::schema::tasks)]
#[diesel(belongs_to(Project))]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Task {
    pub id: i32,
    pub name: String,
    pub project_id: Option<i32>,
}

#[derive(Queryable, Identifiable, Selectable, Associations, Debug, PartialEq)]
#[diesel(table_name = crate::schema::records)]
#[diesel(belongs_to(Task))]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Record {
    pub id: i32,
    pub task_id: i32,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub ended_at: Option<chrono::DateTime<chrono::Utc>>,
}

pub fn upsert_task(conn: &mut SqliteConnection, name: &str) -> Result<(Task, Option<String>)> {
    use crate::schema::projects;
    use crate::schema::tasks;
    let task = diesel::insert_into(tasks::table)
        .values(tasks::name.eq(name))
        .on_conflict(tasks::name)
        .do_update()
        // "updates" the task name to itself - this should be a no-op, but allows us to use
        // the returning clause to fetch the task ID and other details.
        .set(tasks::name.eq(excluded(tasks::name)))
        .returning(Task::as_returning())
        .get_result(conn)?;
    let project_name = projects::table
        .filter(projects::id.nullable().eq(task.project_id))
        .select(projects::name)
        .get_result(conn)
        .optional()?;
    Ok((task, project_name))
}

pub fn get_most_recent_record(
    conn: &mut SqliteConnection,
    before: chrono::DateTime<Utc>,
) -> Result<Option<RecordTuple>> {
    use crate::schema::projects;
    use crate::schema::records;
    use crate::schema::tasks;

    Ok(records::table
        .inner_join(tasks::table.left_outer_join(projects::table))
        .filter(records::started_at.lt(before))
        .order(records::started_at.desc())
        .first(conn)
        .optional()?)
}

pub fn set_record_end_timestamp(
    conn: &mut SqliteConnection,
    record_id: i32,
    timestamp: chrono::DateTime<Utc>,
) -> Result<()> {
    use crate::schema::records;
    let count = diesel::update(records::table.filter(records::id.eq(record_id)))
        .set(records::ended_at.eq(Some(timestamp)))
        .execute(conn)?;
    if count < 1 {
        bail!("No record found with id {record_id}")
    }
    Ok(())
}

pub fn insert_record(
    conn: &mut SqliteConnection,
    task_id: i32,
    start_date: chrono::DateTime<Utc>,
    end_date: Option<chrono::DateTime<Utc>>,
) -> Result<Record> {
    use crate::schema::records;
    let record = diesel::insert_into(records::table)
        .values((
            records::task_id.eq(task_id),
            records::started_at.eq(start_date),
            records::ended_at.eq(end_date),
        ))
        .returning(Record::as_returning())
        .get_result(conn)?;
    Ok(record)
}

pub type RecordTuple = (Record, (Task, Option<Project>));
pub fn query_records(
    conn: &mut SqliteConnection,
) -> Result<impl Iterator<Item = QueryResult<RecordTuple>> + '_> {
    use crate::schema::projects;
    use crate::schema::records;
    use crate::schema::tasks;

    Ok(records::table
        .inner_join(tasks::table.left_outer_join(projects::table))
        .load_iter(conn)?)
}
