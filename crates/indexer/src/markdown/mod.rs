pub(crate) mod annotations;
pub(crate) mod frontmatter;
pub(crate) mod frontmatter_block;

// ---------------------------------------------------------------------------------------------- //

use std::path::Path;

use serde::de::{Deserializer as _, MapAccess, Visitor};
use std::fmt;

use crate::{
    IndexerResult,
    markdown::{frontmatter::Frontmatter, frontmatter_block::FrontmatterBlock},
    model::{Diagnostic, DiagnosticSeverity, Document, ParseReport},
};

pub fn parse_markdown(path: &Path, input: &str) -> IndexerResult<ParseReport<Option<Document>>> {
    let normalized = input.replace("\r\n", "\n");
    let report = match extract_frontmatter(&normalized) {
        FrontmatterBlock::Absent => ParseReport {
            value: None,
            diagnostics: Vec::new(),
        },
        FrontmatterBlock::Unterminated => ParseReport {
            value: None,
            diagnostics: vec![Diagnostic {
                severity: DiagnosticSeverity::Error,
                path: path.to_path_buf(),
                line: None,
                message: "frontmatter block is missing a closing `---` delimiter".to_string(),
            }],
        },
        FrontmatterBlock::Present(frontmatter_text) => {
            let parsed = match parse_unique_frontmatter(frontmatter_text) {
                Ok(parsed) => parsed,
                Err(error) => {
                    return Ok(ParseReport {
                        value: None,
                        diagnostics: vec![Diagnostic {
                            severity: DiagnosticSeverity::Error,
                            path: path.to_path_buf(),
                            line: None,
                            message: format!("invalid frontmatter: {error}"),
                        }],
                    });
                }
            };

            match (parsed.id, parsed.kind) {
                (Some(id), Some(kind)) if !id.trim().is_empty() && !kind.trim().is_empty() => {
                    ParseReport {
                        value: Some(Document {
                            id: id.trim().to_string(),
                            kind: kind.trim().to_string(),
                            title: parsed.title.and_then(|t| {
                                let t = t.trim();
                                (!t.is_empty()).then(|| t.to_string())
                            }),
                            path: path.to_path_buf(),
                        }),
                        diagnostics: Vec::new(),
                    }
                }
                _ => ParseReport {
                    value: None,
                    diagnostics: vec![Diagnostic {
                        severity: DiagnosticSeverity::Error,
                        path: path.to_path_buf(),
                        line: None,
                        message:
                            "frontmatter must include non-empty required fields `id` and `kind`"
                                .to_string(),
                    }],
                },
            }
        }
    };

    Ok(report)
}

fn extract_frontmatter(input: &str) -> FrontmatterBlock<'_> {
    let Some(rest) = input.strip_prefix("---\n") else {
        return FrontmatterBlock::Absent;
    };

    if let Some(end) = rest.find("\n---\n") {
        return FrontmatterBlock::Present(&rest[..end]);
    }

    if let Some(frontmatter) = rest.strip_suffix("\n---") {
        return FrontmatterBlock::Present(frontmatter);
    }

    FrontmatterBlock::Unterminated
}

fn parse_unique_frontmatter(frontmatter_text: &str) -> std::result::Result<Frontmatter, String> {
    struct UniqueMapVisitor;

    impl<'de> Visitor<'de> for UniqueMapVisitor {
        type Value = std::collections::BTreeMap<String, serde_yaml::Value>;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("a YAML mapping with unique keys")
        }

        fn visit_map<A>(self, mut map: A) -> std::result::Result<Self::Value, A::Error>
        where
            A: MapAccess<'de>,
        {
            let mut fields = std::collections::BTreeMap::new();
            while let Some((key, value)) = map.next_entry::<String, serde_yaml::Value>()? {
                if fields.insert(key.clone(), value).is_some() {
                    return Err(serde::de::Error::custom(format!(
                        "duplicate frontmatter key `{key}`"
                    )));
                }
            }
            Ok(fields)
        }
    }

    let deserializer = serde_yaml::Deserializer::from_str(frontmatter_text);
    let raw_map = deserializer
        .deserialize_map(UniqueMapVisitor)
        .map_err(|e| e.to_string())?;

    let id = raw_map
        .get("id")
        .and_then(|v| v.as_str())
        .map(str::to_string);
    let kind = raw_map
        .get("kind")
        .and_then(|v| v.as_str())
        .map(str::to_string);
    let title = raw_map
        .get("title")
        .and_then(|v| v.as_str())
        .map(str::to_string);

    Ok(Frontmatter { id, kind, title })
}
