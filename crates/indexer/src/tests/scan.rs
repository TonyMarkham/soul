use std::fs;
use std::path::PathBuf;

use std::os::unix::fs::PermissionsExt;
use tempfile::tempdir;

use crate::scan::scan_repository;

#[test]
fn scans_only_supported_paths_and_keeps_relative_display_paths_stable() {
    let root = tempdir().expect("tempdir");

    fs::create_dir_all(root.path().join(".docs/interactions")).expect("docs dir");
    fs::create_dir_all(root.path().join(".docs/tests")).expect("docs tests dir");
    fs::create_dir_all(root.path().join(".docs-old")).expect("docs sibling dir");
    fs::create_dir_all(root.path().join("fixtures")).expect("fixtures dir");
    fs::create_dir_all(root.path().join("packages/Soul.Attributes.Tests"))
        .expect("test package dir");
    fs::create_dir_all(root.path().join("src/bin")).expect("bin dir");
    fs::create_dir_all(root.path().join("target/generated")).expect("target dir");

    fs::write(
        root.path().join(".docs/interactions/checkout.md"),
        "\
---
id: interaction.checkout.create-order
kind: interaction
title: Create order
---
",
    )
    .expect("doc");

    fs::write(
        root.path().join(".docs/tests/keep.md"),
        "\
---
id: interaction.checkout.test-doc
kind: interaction
title: Test doc
---
",
    )
    .expect("docs test doc");

    fs::write(
        root.path().join("README.md"),
        "\
---
id: ignored.readme
kind: interaction
---
",
    )
    .expect("ignored readme");

    fs::write(
        root.path().join(".docs-old/rogue.md"),
        "\
---
id: ignored.docs.old
kind: interaction
---
",
    )
    .expect("ignored docs sibling");

    fs::write(
        root.path().join("fixtures/backend.rs"),
        r#"use soul_attributes::soul;

  #[soul(id = "interaction.checkout.create-order", role = "backend")]
  fn create_order() {}"#,
    )
    .expect("annotation");

    fs::write(
        root.path().join("fixtures/frontend.cs"),
        r#"[Soul("interaction.checkout.create-order", Role = "frontend")]
  public void CreateOrder() {}"#,
    )
    .expect("annotation");

    fs::write(
        root.path()
            .join("packages/Soul.Attributes.Tests/ignored.cs"),
        r#"[Soul("ignored.tree", Role = "test")]
  public void Ignored() {}"#,
    )
    .expect("ignored test package");

    fs::write(
        root.path().join("src/bin/keep.rs"),
        r#"use soul_attributes::soul;

  #[soul(id = "interaction.checkout.create-order", role = "tooling")]
  fn keep() {}"#,
    )
    .expect("bin annotation");

    fs::write(
        root.path().join("fixtures/bad.rs"),
        r#"#[soul(id = "interaction.checkout.create-order", role = )]"#,
    )
    .expect("bad annotation");

    fs::write(
        root.path().join("fixtures/invalid_utf8.rs"),
        [0xff, 0xfe, b'\n'],
    )
    .expect("invalid utf8");

    fs::write(
        root.path().join("target/generated/ignored.rs"),
        r#"#[soul(id = "ignored.target.file")]
  fn ignored() {}"#,
    )
    .expect("ignored");

    let graph = scan_repository(root.path()).expect("scan");

    assert_eq!(graph.documents.len(), 2);
    assert_eq!(graph.annotations.len(), 3);
    assert_eq!(graph.diagnostics.len(), 2);
    assert_eq!(
        graph.documents[0].path,
        PathBuf::from(".docs/interactions/checkout.md")
    );
    assert_eq!(
        graph.documents[1].path,
        PathBuf::from(".docs/tests/keep.md")
    );
    assert_eq!(
        graph.annotations[0].path,
        PathBuf::from("fixtures/backend.rs")
    );
    assert_eq!(graph.annotations[2].path, PathBuf::from("src/bin/keep.rs"));
    assert!(!graph.annotations.iter().any(|annotation| {
        annotation.path == PathBuf::from("packages/Soul.Attributes.Tests/ignored.cs")
    }));
}

#[test]
fn keeps_a_root_named_tests_indexable() {
    let workspace = tempdir().expect("tempdir");
    let root = workspace.path().join("tests");

    fs::create_dir_all(root.join(".docs/interactions")).expect("docs dir");
    fs::create_dir_all(root.join("fixtures")).expect("fixtures dir");

    fs::write(
        root.join(".docs/interactions/checkout.md"),
        "\
---
id: interaction.checkout.create-order
kind: interaction
title: Create order
---
  ",
    )
    .expect("doc");

    fs::write(
        root.join("fixtures/backend.rs"),
        r#"use soul_attributes::soul;

  #[soul(id = "interaction.checkout.create-order", role = "backend")]
  fn create_order() {}"#,
    )
    .expect("annotation");

    let graph = scan_repository(&root).expect("scan");

    assert_eq!(graph.documents.len(), 1);
    assert_eq!(graph.annotations.len(), 1);
    assert!(graph.diagnostics.is_empty());
}

#[test]
fn keeps_the_lexicographically_first_document_for_duplicate_ids() {
    let root = tempdir().expect("tempdir");

    fs::create_dir_all(root.path().join(".docs/a")).expect("docs a");
    fs::create_dir_all(root.path().join(".docs/b")).expect("docs b");

    fs::write(
        root.path().join(".docs/a/first.md"),
        "\
---
id: interaction.checkout.create-order
kind: interaction
title: First
---
",
    )
    .expect("first doc");

    fs::write(
        root.path().join(".docs/b/second.md"),
        "\
---
id: interaction.checkout.create-order
kind: interaction
title: Second
---
",
    )
    .expect("second doc");

    let graph = scan_repository(root.path()).expect("scan");

    assert_eq!(graph.documents.len(), 1);
    assert_eq!(graph.documents[0].path, PathBuf::from(".docs/a/first.md"));
    assert_eq!(graph.documents[0].title.as_deref(), Some("First"));
    assert!(
        graph
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains("duplicate markdown id"))
    );
}

#[cfg(unix)]
#[test]
fn returns_a_fatal_walk_error_for_inaccessible_nested_directories() {
    let root = tempdir().expect("tempdir");

    let blocked = root.path().join(".docs/blocked");
    fs::create_dir_all(&blocked).expect("blocked dir");
    fs::set_permissions(&blocked, fs::Permissions::from_mode(0o000)).expect("chmod");

    let error = scan_repository(root.path()).expect_err("walk error");

    match error {
        crate::IndexerError::WalkEntry { path, .. } => {
            assert_eq!(path, blocked);
        }
        other => panic!("expected walk entry error, got {other:?}"),
    }

    fs::set_permissions(&blocked, fs::Permissions::from_mode(0o755)).expect("restore permissions");
}

#[cfg(unix)]
#[test]
fn records_unreadable_files_as_diagnostics_without_aborting_scan() {
    let root = tempdir().expect("tempdir");

    fs::create_dir_all(root.path().join(".docs/interactions")).expect("docs dir");
    fs::create_dir_all(root.path().join("fixtures")).expect("fixtures dir");

    fs::write(
        root.path().join(".docs/interactions/checkout.md"),
        "\
---
id: interaction.checkout.create-order
kind: interaction
title: Create order
---
",
    )
    .expect("doc");

    let unreadable = root.path().join("fixtures/unreadable.rs");
    fs::write(
        &unreadable,
        r#"#[soul(id = "interaction.checkout.create-order", role = "backend")]
fn unreadable() {}"#,
    )
    .expect("unreadable file");
    fs::set_permissions(&unreadable, fs::Permissions::from_mode(0o000)).expect("chmod");

    let graph = scan_repository(root.path()).expect("scan");

    assert_eq!(graph.documents.len(), 1);
    assert!(graph.diagnostics.iter().any(|diagnostic| {
        diagnostic.path == PathBuf::from("fixtures/unreadable.rs")
            && diagnostic.message.contains("failed to read file")
    }));

    fs::set_permissions(&unreadable, fs::Permissions::from_mode(0o644))
        .expect("restore permissions");
}
