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
        task -> Text,
        project_id -> Integer,
        started_at -> TimestamptzSqlite,
        ended_at -> Nullable<TimestamptzSqlite>,
    }
}

diesel::joinable!(records -> projects (project_id));

diesel::allow_tables_to_appear_in_same_query!(
    projects,
    records,
);
