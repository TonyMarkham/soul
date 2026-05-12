use indexer::{
    CodeAnnotation, Diagnostic, DiagnosticSeverity, Document, IndexerError, MarkdownSource,
    Reference, SemanticGraph,
    index::{explain_from_index, load_graph, open_index, write_index},
    model::AnnotationSyntax,
};

use sqlx::SqlitePool;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

fn graph_for_index_tests() -> SemanticGraph {
    SemanticGraph {
        documents: vec![
            Document {
                id: "source.id".to_string(),
                kind: "concept".to_string(),
                title: Some("Source".to_string()),
                path: PathBuf::from("docs/source.md"),
            },
            Document {
                id: "target.id".to_string(),
                kind: "concept".to_string(),
                title: Some("Target".to_string()),
                path: PathBuf::from("docs/target.md"),
            },
        ],
        annotations: vec![CodeAnnotation {
            id: "target.id".to_string(),
            metadata: serde_json::Map::new(),
            path: PathBuf::from("src/lib.rs"),
            line: 12,
            syntax: AnnotationSyntax("rust-attribute".to_string()),
            raw: r#"#[soul(id = "target.id")]"#.to_string(),
        }],
        references: vec![
            Reference {
                source_id: "reference.id".to_string(),
                target_id: "target.id".to_string(),
                source_path: PathBuf::from("docs/reference.md"),
                source_start_line: 3,
                source_start_col: 0,
                source_end_line: 3,
                source_end_col: 13,
                display_text: Some("Target".to_string()),
            },
            Reference {
                source_id: "source.id".to_string(),
                target_id: "target.id".to_string(),
                source_path: PathBuf::from("docs/source.md"),
                source_start_line: 5,
                source_start_col: 4,
                source_end_line: 5,
                source_end_col: 17,
                display_text: None,
            },
        ],
        markdown_sources: vec![MarkdownSource {
            source_id: "reference.id".to_string(),
            source_path: PathBuf::from("docs/reference.md"),
        }],
        diagnostics: vec![Diagnostic {
            severity: DiagnosticSeverity::Error,
            path: PathBuf::from("docs/bad.md"),
            line: Some(1),
            message: "bad markdown".to_string(),
        }],
    }
}

async fn seeded_pool(root: &Path) -> SqlitePool {
    write_index(root, &graph_for_index_tests())
        .await
        .expect("write index");
    open_index(root)
        .await
        .expect("open index")
        .expect("index exists")
}

fn assert_index_corruption(error: IndexerError) {
    assert!(matches!(error, IndexerError::IndexCorruption { .. }));
}

async fn replace_with_corrupt_reference(
    pool: &SqlitePool,
    source_id: &str,
    target_id: &str,
    source_path: &str,
    start_line: i64,
    start_col: i64,
    end_line: i64,
    end_col: i64,
    display_text: Option<&str>,
) {
    let mut conn = pool.acquire().await.expect("acquire connection");

    sqlx::query("DELETE FROM doc_references")
        .execute(&mut *conn)
        .await
        .expect("delete references");
    sqlx::query("PRAGMA ignore_check_constraints = ON")
        .execute(&mut *conn)
        .await
        .expect("set pragma on");
    sqlx::query(
        "INSERT INTO doc_references \
         (source_id, target_id, source_path, source_start_line, source_start_col, source_end_line, source_end_col, display_text) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
        .bind(source_id)
        .bind(target_id)
        .bind(source_path)
        .bind(start_line)
        .bind(start_col)
        .bind(end_line)
        .bind(end_col)
        .bind(display_text)
        .execute(&mut *conn)
        .await
        .expect("insert corrupt reference");
    sqlx::query("PRAGMA ignore_check_constraints = OFF")
        .execute(&mut *conn)
        .await
        .expect("set pragma off");
}

#[tokio::test]
async fn write_index_and_load_graph_roundtrip_references_and_markdown_sources() {
    let root = tempdir().expect("tempdir");
    let pool = seeded_pool(root.path()).await;

    let loaded = load_graph(&pool).await.expect("load graph");

    assert_eq!(loaded.documents, graph_for_index_tests().documents);
    assert_eq!(loaded.annotations, graph_for_index_tests().annotations);
    assert_eq!(loaded.references, graph_for_index_tests().references);
    assert_eq!(
        loaded.markdown_sources,
        graph_for_index_tests().markdown_sources
    );
    assert_eq!(loaded.diagnostics, graph_for_index_tests().diagnostics);
}

#[tokio::test]
async fn explain_from_index_returns_references_for_target_id() {
    let root = tempdir().expect("tempdir");
    let pool = seeded_pool(root.path()).await;

    let result = explain_from_index(&pool, "target.id")
        .await
        .expect("explain");

    assert_eq!(result.documents.len(), 1);
    assert_eq!(result.annotations.len(), 1);
    assert_eq!(result.references.len(), 2);
    assert_eq!(
        result.references[0].source_path,
        PathBuf::from("docs/reference.md")
    );
    assert_eq!(
        result.references[1].source_path,
        PathBuf::from("docs/source.md")
    );
}

#[tokio::test]
async fn corrupt_reference_rows_fail_load_graph_and_explain_visibly() {
    let cases = [
        (
            "source.id",
            "bad target",
            "docs/source.md",
            5,
            4,
            5,
            17,
            None,
        ),
        (
            "source.id",
            "target.id",
            "docs/source.md",
            5,
            4,
            6,
            17,
            None,
        ),
        ("source.id", "target.id", "docs/source.md", 5, 4, 5, 4, None),
        (
            "source.id",
            "target.id",
            "docs/source.md",
            5,
            -1,
            5,
            17,
            None,
        ),
        (
            "source.id",
            "target.id",
            "docs/source.md",
            0,
            4,
            0,
            17,
            None,
        ),
        (
            "missing.id",
            "target.id",
            "docs/missing.md",
            5,
            4,
            5,
            17,
            None,
        ),
    ];

    for case in cases {
        let root = tempdir().expect("tempdir");
        let pool = seeded_pool(root.path()).await;
        replace_with_corrupt_reference(
            &pool, case.0, case.1, case.2, case.3, case.4, case.5, case.6, case.7,
        )
        .await;

        assert_index_corruption(load_graph(&pool).await.expect_err("load should fail"));
        assert_index_corruption(
            explain_from_index(&pool, case.1)
                .await
                .expect_err("explain should fail"),
        );
    }

    let root = tempdir().expect("tempdir");
    let pool = seeded_pool(root.path()).await;
    let absolute_source_path = root.path().join("abs-source.md");
    let absolute_source_path = absolute_source_path.display().to_string();

    replace_with_corrupt_reference(
        &pool,
        "source.id",
        "target.id",
        &absolute_source_path,
        5,
        4,
        5,
        17,
        None,
    )
    .await;

    assert_index_corruption(load_graph(&pool).await.expect_err("load should fail"));
    assert_index_corruption(
        explain_from_index(&pool, "target.id")
            .await
            .expect_err("explain should fail"),
    );
}

#[tokio::test]
async fn corrupt_display_text_markdown_source_and_duplicate_reference_fail_visibly() {
    let root = tempdir().expect("tempdir");
    let pool = seeded_pool(root.path()).await;
    let overlong_display = "x".repeat(1025);
    replace_with_corrupt_reference(
        &pool,
        "source.id",
        "target.id",
        "docs/source.md",
        5,
        4,
        5,
        17,
        Some(&overlong_display),
    )
    .await;
    assert_index_corruption(load_graph(&pool).await.expect_err("display text"));

    let root = tempdir().expect("tempdir");
    let pool = seeded_pool(root.path()).await;
    let mut conn = pool.acquire().await.expect("acquire connection");
    let absolute_bad_path = root.path().join("abs-bad.md");
    let absolute_bad_path = absolute_bad_path.display().to_string();
    sqlx::query("PRAGMA ignore_check_constraints = ON")
        .execute(&mut *conn)
        .await
        .expect("set pragma on");
    sqlx::query("INSERT INTO markdown_sources (source_id, source_path) VALUES (?, ?)")
        .bind("source.id")
        .bind(&absolute_bad_path)
        .execute(&mut *conn)
        .await
        .expect("insert corrupt source");
    sqlx::query("PRAGMA ignore_check_constraints = OFF")
        .execute(&mut *conn)
        .await
        .expect("set pragma off");
    assert_index_corruption(load_graph(&pool).await.expect_err("markdown source"));

    let root = tempdir().expect("tempdir");
    let pool = seeded_pool(root.path()).await;
    sqlx::query(
        "INSERT INTO doc_references \
         (source_id, target_id, source_path, source_start_line, source_start_col, source_end_line, source_end_col, display_text) \
         VALUES ('source.id', 'target.id', 'docs/source.md', 5, 4, 5, 17, NULL)",
    )
        .execute(&pool)
        .await
        .expect("insert duplicate");
    assert_index_corruption(load_graph(&pool).await.expect_err("duplicate reference"));
}
