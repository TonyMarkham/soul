CREATE TABLE markdown_sources (
    source_id   TEXT NOT NULL CHECK (length(trim(source_id)) > 0),
    source_path TEXT NOT NULL CHECK (length(trim(source_path)) > 0),
    PRIMARY KEY (source_id, source_path)
);