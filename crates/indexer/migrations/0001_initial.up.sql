CREATE TABLE documents (
    id      TEXT NOT NULL,
    kind    TEXT NOT NULL,
    title   TEXT,
    path    TEXT NOT NULL
);

CREATE TABLE annotations (
    id       TEXT NOT NULL,
    metadata TEXT NOT NULL,
    path     TEXT NOT NULL,
    line     INTEGER NOT NULL,
    syntax   TEXT NOT NULL,
    raw      TEXT NOT NULL
);

CREATE TABLE diagnostics (
    severity TEXT NOT NULL,
    path     TEXT NOT NULL,
    line     INTEGER,
    message  TEXT NOT NULL
);