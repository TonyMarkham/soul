pub(crate) mod annotations;
pub(crate) mod fence_state;
pub(crate) mod frontmatter;
pub(crate) mod frontmatter_block;
pub(crate) mod line_scanner;
pub(crate) mod parse;
pub(crate) mod scanned_line;
pub(crate) mod wikilink_validation;
pub(crate) mod wikilinks;

// ---------------------------------------------------------------------------------------------- //

use crate::{
    IndexerError, IndexerResult,
    markdown::{frontmatter::Frontmatter, frontmatter_block::FrontmatterBlock, parse::Parse},
    model::{Diagnostic, DiagnosticSeverity, Document, ParseReport},
};

use serde::de::{Deserializer as _, MapAccess, Visitor};
use std::{fmt, path::Path};

pub(crate) fn parse_markdown(path: &Path, input: &str) -> IndexerResult<ParseReport<Parse>> {
    let normalized = input.replace("\r\n", "\n");
    let report = match extract_frontmatter(&normalized) {
        FrontmatterBlock::Absent {
            body,
            body_start_line,
        } => {
            let wiki_report = wikilinks::extract_wikilinks(path, body, body_start_line);
            ParseReport {
                value: Parse {
                    document: None,
                    wiki_links: wiki_report.value,
                    wiki_link_diagnostics: wiki_report.diagnostics,
                },
                diagnostics: Vec::new(),
            }
        }
        FrontmatterBlock::Unterminated => ParseReport {
            value: Parse {
                document: None,
                wiki_links: Vec::new(),
                wiki_link_diagnostics: Vec::new(),
            },
            diagnostics: vec![Diagnostic {
                severity: DiagnosticSeverity::Error,
                path: path.to_path_buf(),
                line: None,
                message: "frontmatter block is missing a closing `---` delimiter".to_string(),
            }],
        },
        FrontmatterBlock::Present {
            frontmatter,
            body,
            body_start_line,
        } => {
            let parsed = match parse_unique_frontmatter(frontmatter) {
                Ok(parsed) => parsed,
                Err(error) => {
                    return Ok(ParseReport {
                        value: Parse {
                            document: None,
                            wiki_links: Vec::new(),
                            wiki_link_diagnostics: Vec::new(),
                        },
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
                    let wiki_report = wikilinks::extract_wikilinks(path, body, body_start_line);
                    ParseReport {
                        value: Parse {
                            document: Some(Document {
                                id: id.trim().to_string(),
                                kind: kind.trim().to_string(),
                                title: parsed.title.and_then(|t| {
                                    let t = t.trim();
                                    (!t.is_empty()).then(|| t.to_string())
                                }),
                                path: path.to_path_buf(),
                            }),
                            wiki_links: wiki_report.value,
                            wiki_link_diagnostics: wiki_report.diagnostics,
                        },
                        diagnostics: Vec::new(),
                    }
                }
                _ => ParseReport {
                    value: Parse {
                        document: None,
                        wiki_links: Vec::new(),
                        wiki_link_diagnostics: Vec::new(),
                    },
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

pub fn wikilink_at_position(
    input: &str,
    line: usize,
    utf16_col: usize,
) -> Option<crate::model::WikiLinkToken> {
    let normalized = input.replace("\r\n", "\n");
    let report = match extract_frontmatter(&normalized) {
        FrontmatterBlock::Absent {
            body,
            body_start_line,
        } => wikilinks::extract_wikilinks(std::path::Path::new(""), body, body_start_line),
        FrontmatterBlock::Present {
            body,
            body_start_line,
            ..
        } => wikilinks::extract_wikilinks(std::path::Path::new(""), body, body_start_line),
        FrontmatterBlock::Unterminated => ParseReport {
            value: Vec::new(),
            diagnostics: Vec::new(),
        },
    };

    report.value.into_iter().find(|token| {
        token.start_line == line
            && token.end_line == line
            && token.start_col <= utf16_col
            && utf16_col < token.end_col
    })
}

fn extract_frontmatter(input: &str) -> FrontmatterBlock<'_> {
    let Some(rest) = input.strip_prefix("---\n") else {
        return FrontmatterBlock::Absent {
            body: input,
            body_start_line: 1,
        };
    };

    if let Some(end) = rest.find("\n---\n") {
        let frontmatter = &rest[..end];
        let body = &rest[end + "\n---\n".len()..];
        let body_start_line = input[..input.len() - body.len()]
            .chars()
            .filter(|ch| *ch == '\n')
            .count()
            + 1;

        return FrontmatterBlock::Present {
            frontmatter,
            body,
            body_start_line,
        };
    }

    if let Some(frontmatter) = rest.strip_suffix("\n---") {
        return FrontmatterBlock::Present {
            frontmatter,
            body: "",
            body_start_line: input.lines().count() + 1,
        };
    }

    FrontmatterBlock::Unterminated
}

fn parse_unique_frontmatter(frontmatter_text: &str) -> IndexerResult<Frontmatter> {
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
        .map_err(|e| IndexerError::frontmatter_parse(e.to_string()))?;

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
