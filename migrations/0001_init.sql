CREATE TABLE settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE TABLE jobs (
    id           INTEGER PRIMARY KEY,
    kind         TEXT NOT NULL,
    payload_json TEXT NOT NULL,
    status       TEXT NOT NULL DEFAULT 'queued' CHECK (status IN ('queued', 'running', 'done', 'failed')),
    attempts     INTEGER NOT NULL DEFAULT 0,
    run_after    INTEGER NOT NULL,
    last_error   TEXT,
    created_at   INTEGER NOT NULL
);

CREATE INDEX jobs_due ON jobs (status, run_after);
