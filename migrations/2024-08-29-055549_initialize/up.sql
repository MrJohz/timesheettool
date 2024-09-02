PRAGMA foreign_keys = ON;

CREATE TABLE
    projects (
        id INTEGER NOT NULL PRIMARY KEY,
        name TEXT UNIQUE NOT NULL
    );

CREATE TABLE
    records (
        id INTEGER NOT NULL PRIMARY KEY,
        task TEXT NOT NULL,
        project_id INTEGER NOT NULL REFERENCES projects,
        started_at TIMESTAMP NOT NULL,
        ended_at TIMESTAMP,
        CONSTRAINT ended_at_gt_started_at CHECK (
            ended_at IS NULL
            OR ended_at > started_at
        )
    );