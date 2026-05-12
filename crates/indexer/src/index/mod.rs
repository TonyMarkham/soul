use crate::{
    IndexerError, IndexerResult, constants,
    graph::ExplainResult,
    markdown::wikilink_validation::{
        is_valid_reference_display_text, is_valid_reference_target_id,
    },
    model::{
        AnnotationSyntax, CodeAnnotation, Diagnostic, DiagnosticSeverity, Document, MarkdownSource,
        Reference, SemanticGraph,
    },
};

use sqlx::{SqlitePool, sqlite::SqliteConnectOptions};
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
    str::FromStr as _,
};

pub async fn write_index(root: &Path, graph: &SemanticGraph) -> IndexerResult<PathBuf> {
    let index_path = root.join(constants::SOUL_DIR).join(constants::INDEX_FILE);
    std::fs::create_dir_all(root.join(constants::SOUL_DIR))
        .map_err(|e| IndexerError::config_read(index_path.clone(), e))?;

    let url = format!("sqlite://{}", index_path.display());
    let opts = SqliteConnectOptions::from_str(&url)
        .map_err(|e| IndexerError::index_db(index_path.clone(), e))?
        .create_if_missing(true);
    let pool = SqlitePool::connect_with(opts)
        .await
        .map_err(|e| IndexerError::index_db(index_path.clone(), e))?;

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .map_err(|e| IndexerError::index_db(index_path.clone(), e.into()))?;

    let mut tx = pool
        .begin()
        .await
        .map_err(|e| IndexerError::index_db(index_path.clone(), e))?;

    sqlx::query("DELETE FROM documents")
        .execute(&mut *tx)
        .await
        .map_err(|e| IndexerError::index_db(index_path.clone(), e))?;
    sqlx::query("DELETE FROM annotations")
        .execute(&mut *tx)
        .await
        .map_err(|e| IndexerError::index_db(index_path.clone(), e))?;
    sqlx::query("DELETE FROM diagnostics")
        .execute(&mut *tx)
        .await
        .map_err(|e| IndexerError::index_db(index_path.clone(), e))?;
    sqlx::query("DELETE FROM doc_references")
        .execute(&mut *tx)
        .await
        .map_err(|e| IndexerError::index_db(index_path.clone(), e))?;
    sqlx::query("DELETE FROM markdown_sources")
        .execute(&mut *tx)
        .await
        .map_err(|e| IndexerError::index_db(index_path.clone(), e))?;

    for doc in &graph.documents {
        sqlx::query("INSERT INTO documents (id, kind, title, path) VALUES (?, ?, ?, ?)")
            .bind(&doc.id)
            .bind(doc.kind.to_string())
            .bind(&doc.title)
            .bind(doc.path.to_string_lossy().as_ref())
            .execute(&mut *tx)
            .await
            .map_err(|e| IndexerError::index_db(index_path.clone(), e))?;
    }

    for ann in &graph.annotations {
        let metadata = serde_json::to_string(&ann.metadata).unwrap_or_else(|_| "{}".to_string());
        sqlx::query(
            "INSERT INTO annotations (id, metadata, path, line, syntax, raw) VALUES (?, ?, ?, ?, ?, ?)",
        )
            .bind(&ann.id)
            .bind(&metadata)
            .bind(ann.path.to_string_lossy().as_ref())
            .bind(ann.line as i64)
            .bind(ann.syntax.to_string())
            .bind(&ann.raw)
            .execute(&mut *tx)
            .await
            .map_err(|e| IndexerError::index_db(index_path.clone(), e))?;
    }

    for diag in &graph.diagnostics {
        sqlx::query("INSERT INTO diagnostics (severity, path, line, message) VALUES (?, ?, ?, ?)")
            .bind(diag.severity.to_string())
            .bind(diag.path.to_string_lossy().as_ref())
            .bind(diag.line.map(|n| n as i64))
            .bind(&diag.message)
            .execute(&mut *tx)
            .await
            .map_err(|e| IndexerError::index_db(index_path.clone(), e))?;
    }

    for reference in &graph.references {
        sqlx::query(
            "INSERT INTO doc_references (source_id, target_id, source_path, source_start_line, source_start_col, source_end_line, source_end_col, display_text) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
            .bind(&reference.source_id)
            .bind(&reference.target_id)
            .bind(reference.source_path.to_string_lossy().as_ref())
            .bind(reference.source_start_line as i64)
            .bind(reference.source_start_col as i64)
            .bind(reference.source_end_line as i64)
            .bind(reference.source_end_col as i64)
            .bind(&reference.display_text)
            .execute(&mut *tx)
            .await
            .map_err(|e| IndexerError::index_db(index_path.clone(), e))?;
    }

    for source in &graph.markdown_sources {
        sqlx::query("INSERT INTO markdown_sources (source_id, source_path) VALUES (?, ?)")
            .bind(&source.source_id)
            .bind(source.source_path.to_string_lossy().as_ref())
            .execute(&mut *tx)
            .await
            .map_err(|e| IndexerError::index_db(index_path.clone(), e))?;
    }

    tx.commit()
        .await
        .map_err(|e| IndexerError::index_db(index_path.clone(), e))?;

    Ok(index_path)
}

pub async fn open_index(root: &Path) -> IndexerResult<Option<SqlitePool>> {
    let index_path = root.join(constants::SOUL_DIR).join(constants::INDEX_FILE);

    if !index_path.exists() {
        return Ok(None);
    }

    let url = format!("sqlite://{}", index_path.display());
    let pool = SqlitePool::connect(&url)
        .await
        .map_err(|e| IndexerError::index_db(index_path.clone(), e))?;

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .map_err(|e| IndexerError::index_db(index_path.clone(), e.into()))?;

    Ok(Some(pool))
}

async fn pool_index_path(pool: &SqlitePool) -> PathBuf {
    let row = sqlx::query("PRAGMA database_list")
        .fetch_all(pool)
        .await
        .ok()
        .and_then(|rows| {
            rows.into_iter().find_map(|row| {
                use sqlx::Row as _;
                let name = row.try_get::<String, _>("name").ok()?;
                (name == "main")
                    .then(|| row.try_get::<String, _>("file").ok())
                    .flatten()
            })
        });

    row.map(PathBuf::from).unwrap_or_default()
}

fn row_to_document(row: &sqlx::sqlite::SqliteRow) -> Document {
    use sqlx::Row;

    Document {
        id: row.get("id"),
        kind: row.get("kind"),
        title: row.get("title"),
        path: PathBuf::from(row.get::<String, _>("path")),
    }
}

fn row_to_annotation(row: &sqlx::sqlite::SqliteRow) -> CodeAnnotation {
    use sqlx::Row;

    let metadata_str = row.get::<String, _>("metadata");
    let metadata = serde_json::from_str(&metadata_str).unwrap_or_default();

    CodeAnnotation {
        id: row.get("id"),
        metadata,
        path: PathBuf::from(row.get::<String, _>("path")),
        line: row.get::<i64, _>("line") as usize,
        syntax: row
            .get::<String, _>("syntax")
            .parse()
            .unwrap_or(AnnotationSyntax("rust-attribute".to_string())),
        raw: row.get("raw"),
    }
}

fn row_to_diagnostic(row: &sqlx::sqlite::SqliteRow) -> Diagnostic {
    use sqlx::Row;

    Diagnostic {
        severity: row
            .get::<String, _>("severity")
            .parse()
            .unwrap_or(DiagnosticSeverity::Error),
        path: PathBuf::from(row.get::<String, _>("path")),
        line: row.get::<Option<i64>, _>("line").map(|n| n as usize),
        message: row.get("message"),
    }
}

fn positive_usize(index_path: &Path, field: &str, value: i64) -> IndexerResult<usize> {
    if value <= 0 {
        return Err(IndexerError::index_corruption(
            index_path.to_path_buf(),
            format!("invalid `{field}`: expected positive integer, got {value}"),
        ));
    }

    usize::try_from(value).map_err(|_| {
        IndexerError::index_corruption(
            index_path.to_path_buf(),
            format!("invalid `{field}`: out of range for usize"),
        )
    })
}

fn non_negative_usize(index_path: &Path, field: &str, value: i64) -> IndexerResult<usize> {
    if value < 0 {
        return Err(IndexerError::index_corruption(
            index_path.to_path_buf(),
            format!("invalid `{field}`: expected non-negative integer, got {value}"),
        ));
    }

    usize::try_from(value).map_err(|_| {
        IndexerError::index_corruption(
            index_path.to_path_buf(),
            format!("invalid `{field}`: out of range for usize"),
        )
    })
}

fn row_to_markdown_source(
    row: &sqlx::sqlite::SqliteRow,
    index_path: &Path,
) -> IndexerResult<MarkdownSource> {
    use sqlx::Row;

    let source_id = row.get::<String, _>("source_id");
    let source_path = row.get::<String, _>("source_path");

    if source_id.trim().is_empty() {
        return Err(IndexerError::index_corruption(
            index_path.to_path_buf(),
            "invalid `markdown_sources.source_id`: empty or whitespace",
        ));
    }

    if source_id != source_id.trim() {
        return Err(IndexerError::index_corruption(
            index_path.to_path_buf(),
            format!("invalid `markdown_sources.source_id`: not trimmed (`{source_id}`)"),
        ));
    }

    if source_path.trim().is_empty() {
        return Err(IndexerError::index_corruption(
            index_path.to_path_buf(),
            "invalid `markdown_sources.source_path`: empty or whitespace",
        ));
    }

    let source_path_buf = PathBuf::from(&source_path);
    if source_path_buf.is_absolute() {
        return Err(IndexerError::index_corruption(
            index_path.to_path_buf(),
            format!(
                "invalid `markdown_sources.source_path`: expected repo-relative path, got `{}`",
                source_path_buf.display()
            ),
        ));
    }

    Ok(MarkdownSource {
        source_id,
        source_path: source_path_buf,
    })
}

fn row_to_reference(row: &sqlx::sqlite::SqliteRow, index_path: &Path) -> IndexerResult<Reference> {
    use sqlx::Row;

    let source_id = row.get::<String, _>("source_id");
    let target_id = row.get::<String, _>("target_id");
    let source_path = row.get::<String, _>("source_path");

    if source_id.trim().is_empty() || source_id != source_id.trim() {
        return Err(IndexerError::index_corruption(
            index_path.to_path_buf(),
            format!("invalid `doc_references.source_id`: `{source_id}`"),
        ));
    }

    if target_id.trim().is_empty() || target_id != target_id.trim() {
        return Err(IndexerError::index_corruption(
            index_path.to_path_buf(),
            format!("invalid `doc_references.target_id`: `{target_id}`"),
        ));
    }

    if !is_valid_reference_target_id(&target_id) {
        return Err(IndexerError::index_corruption(
            index_path.to_path_buf(),
            format!("invalid `doc_references.target_id` grammar: `{target_id}`"),
        ));
    }

    let source_path_buf = PathBuf::from(&source_path);
    if source_path.trim().is_empty() || source_path_buf.is_absolute() {
        return Err(IndexerError::index_corruption(
            index_path.to_path_buf(),
            format!(
                "invalid `doc_references.source_path`: expected repo-relative path, got `{}`",
                source_path_buf.display()
            ),
        ));
    }

    let source_start_line = positive_usize(
        index_path,
        "doc_references.source_start_line",
        row.get::<i64, _>("source_start_line"),
    )?;
    let source_start_col = non_negative_usize(
        index_path,
        "doc_references.source_start_col",
        row.get::<i64, _>("source_start_col"),
    )?;
    let source_end_line = positive_usize(
        index_path,
        "doc_references.source_end_line",
        row.get::<i64, _>("source_end_line"),
    )?;
    let source_end_col = non_negative_usize(
        index_path,
        "doc_references.source_end_col",
        row.get::<i64, _>("source_end_col"),
    )?;

    if source_end_line != source_start_line {
        return Err(IndexerError::index_corruption(
            index_path.to_path_buf(),
            format!(
                "invalid reference span: end line {} must equal start line {}",
                source_end_line, source_start_line
            ),
        ));
    }

    if source_end_col <= source_start_col {
        return Err(IndexerError::index_corruption(
            index_path.to_path_buf(),
            format!(
                "invalid reference span: end col {} must be greater than start col {}",
                source_end_col, source_start_col
            ),
        ));
    }

    let display_text = row.get::<Option<String>, _>("display_text");
    if let Some(text) = &display_text
        && !is_valid_reference_display_text(text)
    {
        return Err(IndexerError::index_corruption(
            index_path.to_path_buf(),
            "invalid `doc_references.display_text`: exceeds max display length",
        ));
    }

    Ok(Reference {
        source_id,
        target_id,
        source_path: source_path_buf,
        source_start_line,
        source_start_col,
        source_end_line,
        source_end_col,
        display_text,
    })
}

fn validate_references(
    references: &[Reference],
    document_sources: &BTreeSet<(String, PathBuf)>,
    markdown_source_pairs: &BTreeSet<(String, PathBuf)>,
    index_path: &Path,
) -> IndexerResult<()> {
    let mut seen_references = BTreeSet::new();

    for reference in references {
        let source_pair = (reference.source_id.clone(), reference.source_path.clone());
        if !document_sources.contains(&source_pair) && !markdown_source_pairs.contains(&source_pair)
        {
            return Err(IndexerError::index_corruption(
                index_path.to_path_buf(),
                format!(
                    "reference source `{}` at `{}` does not match any document or markdown source",
                    reference.source_id,
                    reference.source_path.display()
                ),
            ));
        }

        let key = (
            reference.source_id.clone(),
            reference.target_id.clone(),
            reference.source_path.clone(),
            reference.source_start_line,
            reference.source_start_col,
            reference.source_end_line,
            reference.source_end_col,
            reference.display_text.clone(),
        );

        if !seen_references.insert(key) {
            return Err(IndexerError::index_corruption(
                index_path.to_path_buf(),
                "duplicate doc_references row",
            ));
        }
    }

    Ok(())
}

pub async fn load_graph(pool: &SqlitePool) -> IndexerResult<SemanticGraph> {
    let index_path = pool_index_path(pool).await;

    let doc_rows = sqlx::query("SELECT id, kind, title, path FROM documents ORDER BY path, id")
        .fetch_all(pool)
        .await
        .map_err(|e| IndexerError::index_db(index_path.clone(), e))?;
    let documents: Vec<Document> = doc_rows.iter().map(row_to_document).collect();

    let ann_rows = sqlx::query(
        "SELECT id, metadata, path, line, syntax, raw FROM annotations ORDER BY path, line, id",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| IndexerError::index_db(index_path.clone(), e))?;
    let annotations: Vec<CodeAnnotation> = ann_rows.iter().map(row_to_annotation).collect();

    let reference_rows = sqlx::query(
        "SELECT source_id, target_id, source_path, source_start_line, source_start_col, source_end_line, source_end_col, display_text FROM doc_references ORDER BY source_path, source_start_line, source_start_col, target_id",
    )
        .fetch_all(pool)
        .await
        .map_err(|e| IndexerError::index_db(index_path.clone(), e))?;
    let references: Vec<Reference> = reference_rows
        .iter()
        .map(|row| row_to_reference(row, &index_path))
        .collect::<IndexerResult<Vec<_>>>()?;

    let markdown_source_rows = sqlx::query(
        "SELECT source_id, source_path FROM markdown_sources ORDER BY source_path, source_id",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| IndexerError::index_db(index_path.clone(), e))?;
    let markdown_sources: Vec<MarkdownSource> = markdown_source_rows
        .iter()
        .map(|row| row_to_markdown_source(row, &index_path))
        .collect::<IndexerResult<Vec<_>>>()?;

    let diag_rows = sqlx::query(
        "SELECT severity, path, line, message FROM diagnostics ORDER BY path, line, message",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| IndexerError::index_db(index_path.clone(), e))?;
    let diagnostics: Vec<Diagnostic> = diag_rows.iter().map(row_to_diagnostic).collect();

    let document_sources: BTreeSet<(String, PathBuf)> = documents
        .iter()
        .map(|document| (document.id.clone(), document.path.clone()))
        .collect();
    let markdown_source_pairs: BTreeSet<(String, PathBuf)> = markdown_sources
        .iter()
        .map(|source| (source.source_id.clone(), source.source_path.clone()))
        .collect();

    validate_references(
        &references,
        &document_sources,
        &markdown_source_pairs,
        &index_path,
    )?;

    Ok(SemanticGraph {
        documents,
        annotations,
        references,
        markdown_sources,
        diagnostics,
    })
}

pub async fn explain_from_index(pool: &SqlitePool, id: &str) -> IndexerResult<ExplainResult> {
    let index_path = pool_index_path(pool).await;

    let doc_rows =
        sqlx::query("SELECT id, kind, title, path FROM documents WHERE id = ? ORDER BY path, id")
            .bind(id)
            .fetch_all(pool)
            .await
            .map_err(|e| IndexerError::index_db(index_path.clone(), e))?;
    let documents: Vec<Document> = doc_rows.iter().map(row_to_document).collect();

    let ann_rows =
        sqlx::query(
            "SELECT id, metadata, path, line, syntax, raw FROM annotations WHERE id = ? ORDER BY path, line, id",
        )
            .bind(id)
            .fetch_all(pool)
            .await
            .map_err(|e| IndexerError::index_db(index_path.clone(), e))?;
    let annotations: Vec<CodeAnnotation> = ann_rows.iter().map(row_to_annotation).collect();

    let reference_rows = sqlx::query(
        "SELECT source_id, target_id, source_path, source_start_line, source_start_col, source_end_line, source_end_col, display_text FROM doc_references WHERE target_id = ? ORDER BY source_path, source_start_line, source_start_col, target_id",
    )
        .bind(id)
        .fetch_all(pool)
        .await
        .map_err(|e| IndexerError::index_db(index_path.clone(), e))?;
    let references: Vec<Reference> = reference_rows
        .iter()
        .map(|row| row_to_reference(row, &index_path))
        .collect::<IndexerResult<Vec<_>>>()?;

    let all_document_source_rows = sqlx::query("SELECT id, path FROM documents ORDER BY path, id")
        .fetch_all(pool)
        .await
        .map_err(|e| IndexerError::index_db(index_path.clone(), e))?;
    let document_sources: BTreeSet<(String, PathBuf)> = all_document_source_rows
        .iter()
        .map(|row| {
            use sqlx::Row;
            (
                row.get::<String, _>("id"),
                PathBuf::from(row.get::<String, _>("path")),
            )
        })
        .collect();

    let markdown_source_rows = sqlx::query(
        "SELECT source_id, source_path FROM markdown_sources ORDER BY source_path, source_id",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| IndexerError::index_db(index_path.clone(), e))?;
    let markdown_sources: Vec<MarkdownSource> = markdown_source_rows
        .iter()
        .map(|row| row_to_markdown_source(row, &index_path))
        .collect::<IndexerResult<Vec<_>>>()?;
    let markdown_source_pairs: BTreeSet<(String, PathBuf)> = markdown_sources
        .iter()
        .map(|source| (source.source_id.clone(), source.source_path.clone()))
        .collect();

    validate_references(
        &references,
        &document_sources,
        &markdown_source_pairs,
        &index_path,
    )?;

    let diag_rows = sqlx::query(
        "SELECT severity, path, line, message FROM diagnostics ORDER BY path, line, message",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| IndexerError::index_db(index_path.clone(), e))?;
    let scan_diagnostics: Vec<Diagnostic> = diag_rows.iter().map(row_to_diagnostic).collect();

    Ok(ExplainResult {
        id: id.to_string(),
        documents,
        annotations,
        references,
        scan_diagnostics,
    })
}
