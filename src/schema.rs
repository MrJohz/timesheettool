// @generated automatically by Diesel CLI.

diesel::table! {
    projects (id) {
        id -> Integer,
        name -> Text,
    }
}

diesel::table! {
    records (id) {
        id -> Integer,
        task_id -> Integer,
        started_at -> TimestamptzSqlite,
        ended_at -> Nullable<TimestamptzSqlite>,
    }
}

diesel::table! {
    tasks (id) {
        id -> Integer,
        name -> Text,
        project_id -> Nullable<Integer>,
    }
}

diesel::joinable!(records -> tasks (task_id));
diesel::joinable!(tasks -> projects (project_id));

diesel::allow_tables_to_appear_in_same_query!(
    projects,
    records,
    tasks,
);
