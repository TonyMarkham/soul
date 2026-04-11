use std::{fs, process::Command};
use tempfile::tempdir;

fn write_test_config(root: &std::path::Path) {
    let soul_dir = root.join(".soul");
    fs::create_dir_all(&soul_dir).expect(".soul dir");

    let target_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap() // crates/
        .parent()
        .unwrap() // workspace root
        .join("target/debug");

    let rust_lib = target_dir.join(format!(
        "libsoul_plugin_rust{}",
        std::env::consts::DLL_SUFFIX
    ));
    let csharp_lib = target_dir.join(format!(
        "libsoul_plugin_csharp{}",
        std::env::consts::DLL_SUFFIX
    ));

    let config = format!(
        "[scan]\n\
        excluded_dirs = [\".git\", \".soul\", \"target\", \".idea\", \".vscode\", \".vs\", \".codex\", \"node_modules\", \"obj\"]\n\
        excluded_dir_suffixes = [\"Tests\", \".Tests\"]\n\
        excluded_bin_except_under = [\"src\"]\n\
        \n\
        [[plugins]]\n\
        language = \"rust\"\n\
        path = \"{rust_lib}\"\n\
        \n\
        [[plugins]]\n\
        language = \"csharp\"\n\
        path = \"{csharp_lib}\"\n",
        rust_lib = rust_lib.display(),
        csharp_lib = csharp_lib.display(),
    );

    fs::write(soul_dir.join("soul.toml"), config).expect("soul.toml");
}

#[test]
fn explain_command_prints_matches_and_diagnostics() {
    let root = tempdir().expect("tempdir");

    write_test_config(root.path());
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

    fs::write(
        root.path().join("fixtures/backend.rs"),
        r#"use soul_attributes::soul;

#[soul(id = "interaction.checkout.create-order")]
pub fn create_order() {
}
"#,
    )
    .expect("backend");

    fs::write(
        root.path().join("fixtures/frontend.cs"),
        r#"using Soul.Attributes;

public class CheckoutController
{
    [Soul("interaction.checkout.create-order")]
    public void CreateOrder()
    {
    }
}
"#,
    )
    .expect("frontend");

    fs::write(
        root.path().join("fixtures/bad.rs"),
        r#"#[soul(id = "interaction.checkout.create-order", junk = )]
pub fn broken_annotation() {
}
"#,
    )
    .expect("bad");

    fs::write(
        root.path().join("fixtures/invalid_utf8.rs"),
        [0xff, 0xfe, b'\n'],
    )
    .expect("invalid utf8");

    fs::write(
        root.path().join(".docs/interactions/bad.md"),
        "\
---
id: interaction.checkout.bad
kind: interaction
title: Broken doc
",
    )
    .expect("bad doc");

    let output = Command::new(env!("CARGO_BIN_EXE_indexer"))
        .args([
            "explain",
            "interaction.checkout.create-order",
            "--root",
            root.path().to_str().expect("root path"),
        ])
        .output()
        .expect("run indexer");

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("stdout");
    let expected = "\
ID: interaction.checkout.create-order

Documents:
- .docs/interactions/checkout.md [kind=interaction, title=Create order]

Annotations:
- fixtures/backend.rs:3
- fixtures/frontend.cs:5

Diagnostics:
- .docs/interactions/bad.md frontmatter block is missing a closing `---` delimiter
- fixtures/bad.rs:1 malformed soul attribute
- fixtures/invalid_utf8.rs file is not valid UTF-8
";
    assert_eq!(stdout, expected);
}

#[test]
fn explain_command_returns_zero_for_missing_id() {
    let root = tempdir().expect("tempdir");

    write_test_config(root.path());
    let output = Command::new(env!("CARGO_BIN_EXE_indexer"))
        .args([
            "explain",
            "interaction.checkout.missing",
            "--root",
            root.path().to_str().expect("root path"),
        ])
        .output()
        .expect("run indexer");

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("stdout");
    let expected = "\
ID: interaction.checkout.missing

Documents:
none

Annotations:
none
";
    assert_eq!(stdout, expected);
}

#[test]
fn explain_command_returns_nonzero_for_invalid_root() {
    let root = tempdir().expect("tempdir");
    let missing_root = root.path().join("does-not-exist");

    let output = Command::new(env!("CARGO_BIN_EXE_indexer"))
        .args([
            "explain",
            "interaction.checkout.create-order",
            "--root",
            missing_root.to_str().expect("root path"),
        ])
        .output()
        .expect("run indexer");

    assert!(!output.status.success());

    let stderr = String::from_utf8(output.stderr).expect("stderr");
    assert!(stderr.contains("root does not exist or is not a directory"));
}
