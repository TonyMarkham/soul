use crate::{
    IndexerError, IndexerResult, constants,
    graph::ExplainResult,
    model::{
        AnnotationSyntax, CodeAnnotation, Diagnostic, DiagnosticSeverity, Document, SemanticGraph,
    },
};

use sqlx::{SqlitePool, sqlite::SqliteConnectOptions};
use std::{
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
            "INSERT INTO annotations (id, role, metadata, path, line, syntax, raw) VALUES (?, ?, ?, ?, ?, ?, ?)"
        )
            .bind(&ann.id)
            .bind(&ann.role)
            .bind(&metadata)
            .bind(ann.path.to_string_lossy().as_ref())
            .bind(ann.line as i64)
            .bind(ann.syntax.to_string())
            .bind(&ann.raw)
            .execute(&mut *tx).await
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

pub async fn load_graph(pool: &SqlitePool) -> IndexerResult<SemanticGraph> {
    let dummy_path = std::path::PathBuf::new();

    let doc_rows = sqlx::query("SELECT id, kind, title, path FROM documents")
        .fetch_all(pool)
        .await
        .map_err(|e| IndexerError::index_db(dummy_path.clone(), e))?;

    let mut documents = Vec::new();
    for row in doc_rows {
        use sqlx::Row;
        documents.push(Document {
            id: row.get("id"),
            kind: row.get("kind"),
            title: row.get("title"),
            path: PathBuf::from(row.get::<String, _>("path")),
        });
    }

    let ann_rows =
        sqlx::query("SELECT id, role, metadata, path, line, syntax, raw FROM annotations")
            .fetch_all(pool)
            .await
            .map_err(|e| IndexerError::index_db(dummy_path.clone(), e))?;

    let mut annotations = Vec::new();
    for row in ann_rows {
        use sqlx::Row;
        let metadata_str = row.get::<String, _>("metadata");
        let metadata = serde_json::from_str(&metadata_str).unwrap_or_default();
        annotations.push(CodeAnnotation {
            id: row.get("id"),
            role: row.get("role"),
            metadata,
            path: PathBuf::from(row.get::<String, _>("path")),
            line: row.get::<i64, _>("line") as usize,
            syntax: row
                .get::<String, _>("syntax")
                .parse()
                .unwrap_or(AnnotationSyntax::RustAttribute),
            raw: row.get("raw"),
        });
    }

    let diag_rows = sqlx::query("SELECT severity, path, line, message FROM diagnostics")
        .fetch_all(pool)
        .await
        .map_err(|e| IndexerError::index_db(dummy_path.clone(), e))?;

    let mut diagnostics = Vec::new();
    for row in diag_rows {
        use sqlx::Row;
        diagnostics.push(Diagnostic {
            severity: row
                .get::<String, _>("severity")
                .parse()
                .unwrap_or(DiagnosticSeverity::Error),
            path: PathBuf::from(row.get::<String, _>("path")),
            line: row.get::<Option<i64>, _>("line").map(|n| n as usize),
            message: row.get("message"),
        });
    }

    Ok(SemanticGraph {
        documents,
        annotations,
        diagnostics,
    })
}

pub async fn explain_from_index(pool: &SqlitePool, id: &str) -> IndexerResult<ExplainResult> {
    let dummy_path = std::path::PathBuf::new();

    let doc_rows = sqlx::query("SELECT id, kind, title, path FROM documents WHERE id = ?")
        .bind(id)
        .fetch_all(pool)
        .await
        .map_err(|e| IndexerError::index_db(dummy_path.clone(), e))?;

    let mut documents = Vec::new();
    for row in doc_rows {
        use sqlx::Row;
        documents.push(Document {
            id: row.get("id"),
            kind: row.get("kind"),
            title: row.get("title"),
            path: PathBuf::from(row.get::<String, _>("path")),
        });
    }

    let ann_rows = sqlx::query(
        "SELECT id, role, metadata, path, line, syntax, raw FROM annotations WHERE id = ?",
    )
    .bind(id)
    .fetch_all(pool)
    .await
    .map_err(|e| IndexerError::index_db(dummy_path.clone(), e))?;

    let mut annotations = Vec::new();
    for row in ann_rows {
        use sqlx::Row;
        let metadata_str = row.get::<String, _>("metadata");
        let metadata = serde_json::from_str(&metadata_str).unwrap_or_default();
        annotations.push(CodeAnnotation {
            id: row.get("id"),
            role: row.get("role"),
            metadata,
            path: PathBuf::from(row.get::<String, _>("path")),
            line: row.get::<i64, _>("line") as usize,
            syntax: row
                .get::<String, _>("syntax")
                .parse()
                .unwrap_or(AnnotationSyntax::RustAttribute),
            raw: row.get("raw"),
        });
    }

    let diag_rows = sqlx::query("SELECT severity, path, line, message FROM diagnostics")
        .fetch_all(pool)
        .await
        .map_err(|e| IndexerError::index_db(dummy_path.clone(), e))?;

    let mut scan_diagnostics = Vec::new();
    for row in diag_rows {
        use sqlx::Row;
        scan_diagnostics.push(Diagnostic {
            severity: row
                .get::<String, _>("severity")
                .parse()
                .unwrap_or(DiagnosticSeverity::Error),
            path: PathBuf::from(row.get::<String, _>("path")),
            line: row.get::<Option<i64>, _>("line").map(|n| n as usize),
            message: row.get("message"),
        });
    }

    Ok(ExplainResult {
        id: id.to_string(),
        documents,
        annotations,
        scan_diagnostics,
    })
}
