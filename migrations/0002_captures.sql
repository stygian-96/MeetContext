CREATE TABLE captures (
    id          INTEGER PRIMARY KEY,
    name        TEXT NOT NULL,
    state       TEXT NOT NULL,           -- recording, ready, failed, interrupted
    consent_at  INTEGER NOT NULL,
    started_at  INTEGER NOT NULL,
    ended_at    INTEGER,
    duration_ms INTEGER,
    error       TEXT
);

CREATE TABLE audio_chunks (
    id         INTEGER PRIMARY KEY,
    capture_id INTEGER NOT NULL REFERENCES captures (id) ON DELETE CASCADE,
    track      TEXT NOT NULL CHECK (track IN ('mic', 'system')),
    seq        INTEGER NOT NULL,
    start_ms   INTEGER NOT NULL,
    end_ms     INTEGER NOT NULL,
    path       TEXT NOT NULL,
    bytes      INTEGER NOT NULL,
    UNIQUE (capture_id, track, seq)
);
