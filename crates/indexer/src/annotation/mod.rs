pub(crate) mod c_sharp;
pub(crate) mod normalized_annotation;
pub(crate) mod parser;
pub(crate) mod rust;

pub(crate) use c_sharp::CSharpParser;
pub(crate) use normalized_annotation::NormalizedAnnotation;
pub(crate) use parser::Parser;
pub(crate) use rust::RustParser;

use crate::{
    AnnotationError, AnnotationResult, IndexerResult,
    model::{CodeAnnotation, Diagnostic, DiagnosticSeverity, ParseReport},
};

use serde::de::{Deserializer, MapAccess, Visitor};
use serde_json::{Map, Value};
use std::{fmt, path::Path};

static RUST_ATTRIBUTE_PARSER: RustParser = RustParser;
static CSHARP_ATTRIBUTE_PARSER: CSharpParser = CSharpParser;
static PARSERS: &[&(dyn Parser + Sync)] = &[&RUST_ATTRIBUTE_PARSER, &CSHARP_ATTRIBUTE_PARSER];

pub fn parse_annotations(
    path: &Path,
    input: &str,
) -> IndexerResult<ParseReport<Vec<CodeAnnotation>>> {
    let Some(parser) = parser_for_path(path) else {
        return Ok(ParseReport {
            value: Vec::new(),
            diagnostics: Vec::new(),
        });
    };

    let mut annotations = Vec::new();
    let mut diagnostics = Vec::new();

    for (index, line) in input.lines().enumerate() {
        let Some(parsed) = parser.parse_line(line) else {
            continue;
        };

        match parsed {
            Ok(annotation) => annotations.push(CodeAnnotation {
                id: annotation.id,
                role: annotation.role,
                metadata: annotation.metadata,
                path: path.to_path_buf(),
                line: index + 1,
                syntax: parser.syntax(),
                raw: annotation.raw,
            }),
            Err(error) => diagnostics.push(Diagnostic {
                severity: DiagnosticSeverity::Error,
                path: path.to_path_buf(),
                line: Some(index + 1),
                message: error.to_string(),
            }),
        }
    }

    Ok(ParseReport {
        value: annotations,
        diagnostics,
    })
}

fn parser_for_path(path: &Path) -> Option<&'static (dyn Parser + Sync)> {
    let ext = path.extension().and_then(|ext| ext.to_str())?;
    PARSERS.iter().copied().find(|p| p.extension() == ext)
}

fn split_assignments(payload: &str) -> AnnotationResult<Vec<String>> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut in_string = false;
    let mut escape = false;

    for ch in payload.chars() {
        if escape {
            current.push(ch);
            escape = false;
            continue;
        }

        match ch {
            '\\' if in_string => {
                current.push(ch);
                escape = true;
            }
            '"' => {
                current.push(ch);
                in_string = !in_string;
            }
            ',' if !in_string => {
                let segment = current.trim();
                if segment.is_empty() {
                    return Err(AnnotationError::malformed());
                }
                parts.push(segment.to_string());
                current = String::new();
            }
            _ => current.push(ch),
        }
    }

    if in_string {
        return Err(AnnotationError::malformed());
    }

    let tail = current.trim();
    if tail.is_empty() {
        if payload.trim().is_empty() {
            return Ok(Vec::new());
        }
        if payload.trim_end().ends_with(',') {
            return Ok(parts);
        }
        return Err(AnnotationError::malformed());
    }

    parts.push(tail.to_string());
    Ok(parts)
}

fn insert_metadata_field(
    fields: &mut Map<String, Value>,
    key: String,
    value: Value,
) -> AnnotationResult<()> {
    if fields.insert(key, value).is_some() {
        return Err(AnnotationError::malformed());
    }
    Ok(())
}

fn merge_metadata_json(fields: &mut Map<String, Value>, json: &str) -> AnnotationResult<()> {
    let metadata = parse_unique_json_object(json)?;

    for (key, value) in metadata {
        if key == "id" || fields.contains_key(&key) {
            return Err(AnnotationError::malformed());
        }
        fields.insert(key, value);
    }

    Ok(())
}

fn normalized_annotation_from_fields(
    mut fields: Map<String, Value>,
    raw: &str,
) -> AnnotationResult<NormalizedAnnotation> {
    let id = take_trimmed_string(&mut fields, "id").ok_or_else(AnnotationError::missing_id)?;

    let role = take_trimmed_string(&mut fields, "role");

    Ok(NormalizedAnnotation {
        id,
        role,
        metadata: fields,
        raw: raw.to_string(),
    })
}

fn parse_unique_json_object(json: &str) -> AnnotationResult<Map<String, Value>> {
    struct UniqueObjectVisitor;

    impl<'de> Visitor<'de> for UniqueObjectVisitor {
        type Value = Map<String, Value>;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("a JSON object with unique property names")
        }

        fn visit_map<A>(self, mut map: A) -> std::result::Result<Self::Value, A::Error>
        where
            A: MapAccess<'de>,
        {
            let mut fields = Map::new();
            while let Some((key, value)) = map.next_entry::<String, Value>()? {
                if fields.insert(key, value).is_some() {
                    return Err(serde::de::Error::custom("duplicate property name"));
                }
            }
            Ok(fields)
        }
    }

    let mut deserializer = serde_json::Deserializer::from_str(json);
    let parsed = deserializer
        .deserialize_map(UniqueObjectVisitor)
        .map_err(|_| AnnotationError::malformed())?;
    deserializer
        .end()
        .map_err(|_| AnnotationError::malformed())?;
    Ok(parsed)
}

fn take_trimmed_string(fields: &mut Map<String, Value>, key: &str) -> Option<String> {
    match fields.remove(key) {
        Some(Value::String(value)) => {
            let trimmed = value.trim();
            (!trimmed.is_empty()).then(|| trimmed.to_string())
        }
        Some(_) | None => None,
    }
}
