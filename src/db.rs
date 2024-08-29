use anyhow::Result;
use diesel::prelude::*;
use diesel::{Connection, SqliteConnection};
use dotenvy::dotenv;

pub fn establish_connection() -> SqliteConnection {
    dotenv().ok();

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    SqliteConnection::establish(&database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {}", database_url))
}

#[derive(Queryable, Identifiable, Selectable, Debug, PartialEq)]
#[diesel(table_name= crate::schema::projects)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Project {
    pub id: i32,
    pub name: String,
}

#[derive(Queryable, Identifiable, Selectable, Associations, Debug, PartialEq)]
#[diesel(table_name= crate::schema::tasks)]
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
    pub occurred_at: chrono::DateTime<chrono::Utc>,
    pub ended_at: chrono::DateTime<chrono::Utc>,
}

pub fn query_records(
    conn: &mut SqliteConnection,
) -> Result<Vec<(Record, (Task, Option<Project>))>> {
    use crate::schema::projects;
    use crate::schema::records;
    use crate::schema::tasks;

    Ok(records::table
        .inner_join(tasks::table.left_outer_join(projects::table))
        .load(conn)?)
}
