CREATE TABLE doc_references (
    id                INTEGER PRIMARY KEY,
    source_id         TEXT NOT NULL CHECK (length(trim(source_id)) > 0),
    target_id         TEXT NOT NULL CHECK (length(trim(target_id)) > 0),
    source_path       TEXT NOT NULL CHECK (length(trim(source_path)) > 0),
    source_start_line INTEGER NOT NULL CHECK (source_start_line > 0),
    source_start_col  INTEGER NOT NULL CHECK (source_start_col >= 0),
    source_end_line   INTEGER NOT NULL CHECK (source_end_line = source_start_line),
    source_end_col    INTEGER NOT NULL CHECK (source_end_col > source_start_col),
    display_text      TEXT
);