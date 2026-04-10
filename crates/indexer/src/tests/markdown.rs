use std::path::Path;

use crate::markdown::parse_markdown;

#[test]
fn parses_valid_frontmatter() {
    let input = "\
---
id: interaction.checkout.create-order
kind: interaction
title: Create order
---

# Checkout
";
    let report = parse_markdown(Path::new("checkout.md"), input).expect("parse");

    assert!(report.diagnostics.is_empty());
    let document = report.value.expect("document");
    assert_eq!(document.id, "interaction.checkout.create-order");
    assert_eq!(document.kind, "interaction");
    assert_eq!(document.title.as_deref(), Some("Create order"));
}

#[test]
fn ignores_markdown_without_frontmatter() {
    let report = parse_markdown(Path::new("plain.md"), "# Hello").expect("parse");

    assert!(report.value.is_none());
    assert!(report.diagnostics.is_empty());
}

#[test]
fn reports_unterminated_frontmatter() {
    let input = "\
---
id: interaction.checkout.create-order
kind: interaction
";
    let report = parse_markdown(Path::new("bad.md"), input).expect("parse");

    assert!(report.value.is_none());
    assert_eq!(report.diagnostics.len(), 1);
    assert!(
        report.diagnostics[0]
            .message
            .contains("missing a closing `---` delimiter")
    );
}

#[test]
fn parses_frontmatter_closed_at_eof() {
    let input = "\
---
id: interaction.checkout.create-order
kind: interaction
title: Create order
---";
    let report = parse_markdown(Path::new("checkout.md"), input).expect("parse");

    assert!(report.diagnostics.is_empty());
    let document = report.value.expect("document");
    assert_eq!(document.id, "interaction.checkout.create-order");
    assert_eq!(document.kind, "interaction");
    assert_eq!(document.title.as_deref(), Some("Create order"));
}

#[test]
fn reports_invalid_yaml() {
    let input = "\
---
id: [
kind: interaction
---
";
    let report = parse_markdown(Path::new("bad.md"), input).expect("parse");

    assert!(report.value.is_none());
    assert_eq!(report.diagnostics.len(), 1);
    assert!(
        report.diagnostics[0]
            .message
            .contains("invalid frontmatter")
    );
}

#[test]
fn reports_duplicate_frontmatter_keys() {
    let input = "\
---
id: interaction.checkout.create-order
id: interaction.checkout.duplicate
kind: interaction
---
";
    let report = parse_markdown(Path::new("bad.md"), input).expect("parse");

    assert!(report.value.is_none());
    assert_eq!(report.diagnostics.len(), 1);
    assert!(report.diagnostics[0].message.contains("duplicate"));
}

#[test]
fn reports_missing_required_fields() {
    let input = "\
---
title: Missing id
---
";
    let report = parse_markdown(Path::new("bad.md"), input).expect("parse");

    assert!(report.value.is_none());
    assert_eq!(report.diagnostics.len(), 1);
    assert!(report.diagnostics[0].message.contains("id"));
}

#[test]
fn reports_empty_required_fields() {
    let input = "\
---
id: \" \"
kind: interaction
---
";
    let report = parse_markdown(Path::new("bad.md"), input).expect("parse");

    assert!(report.value.is_none());
    assert_eq!(report.diagnostics.len(), 1);
    assert!(
        report.diagnostics[0]
            .message
            .contains("non-empty required fields")
    );
}

#[test]
fn normalizes_blank_title_to_absent() {
    let input = "\
---
id: interaction.checkout.create-order
kind: interaction
title: \" \"
---
";
    let report = parse_markdown(Path::new("checkout.md"), input).expect("parse");

    assert!(report.diagnostics.is_empty());
    let document = report.value.expect("document");
    assert_eq!(document.title, None);
}
