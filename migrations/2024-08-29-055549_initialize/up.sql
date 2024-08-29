PRAGMA foreign_keys = ON;

CREATE TABLE projects (
    id INTEGER NOT NULL PRIMARY KEY,
    name TEXT NOT NULL
);

CREATE TABLE tasks (
    id INTEGER NOT NULL PRIMARY KEY,
    name TEXT NOT NULL,
    project_id INTEGER REFERENCES projects
);

CREATE TABLE records (
    id INTEGER NOT NULL PRIMARY KEY,
    task_id INTEGER NOT NULL REFERENCES tasks,
    occurred_at TIMESTAMP NOT NULL,
    ended_at TIMESTAMP NOT NULL
);
