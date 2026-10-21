CREATE TABLE projects (
    id          INTEGER PRIMARY KEY,
    name        TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',   -- helps the AI map meetings to this project
    created_at  INTEGER NOT NULL,
    updated_at  INTEGER NOT NULL
);

-- A full Markdown snapshot per save or merge; version 0 (no row) is an empty PRD.
CREATE TABLE prd_versions (
    id         INTEGER PRIMARY KEY,
    project_id INTEGER NOT NULL REFERENCES projects (id) ON DELETE CASCADE,
    version    INTEGER NOT NULL,
    markdown   TEXT NOT NULL,
    source     TEXT NOT NULL,                -- edit, merge, draft
    created_at INTEGER NOT NULL,
    UNIQUE (project_id, version)
);

-- The current version's sections; `id` stays the same across edits so proposals can target it.
CREATE TABLE prd_sections (
    id              INTEGER PRIMARY KEY,
    project_id      INTEGER NOT NULL REFERENCES projects (id) ON DELETE CASCADE,
    heading         TEXT NOT NULL,           -- '' for text before the first heading
    level           INTEGER NOT NULL,        -- 0 for that intro text
    position        INTEGER NOT NULL,
    content_hash    TEXT NOT NULL,
    updated_version INTEGER NOT NULL
);
