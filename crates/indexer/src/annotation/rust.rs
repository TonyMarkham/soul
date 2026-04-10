use crate::{
    AnnotationError, AnnotationResult,
    annotation::{
        NormalizedAnnotation, insert_metadata_field, normalized_annotation_from_fields,
        parser::Parser, split_assignments,
    },
    model::AnnotationSyntax,
};

use serde_json::{Map, Value};

// ---------------------------------------------------------------------------------------------- //

pub(crate) struct RustParser;

impl Parser for RustParser {
    fn extension(&self) -> &'static str {
        "rs"
    }

    fn syntax(&self) -> AnnotationSyntax {
        AnnotationSyntax::RustAttribute
    }

    fn parse_line(&self, line: &str) -> Option<AnnotationResult<NormalizedAnnotation>> {
        let trimmed = line.trim();
        if !trimmed.starts_with("#[soul(") {
            return None;
        }

        if !trimmed.ends_with(")]") {
            return Some(Err(AnnotationError::malformed()));
        }

        let payload = &trimmed["#[soul(".len()..trimmed.len() - 2];
        Some(
            parse_fields(payload)
                .and_then(|fields| normalized_annotation_from_fields(fields, trimmed)),
        )
    }
}

fn parse_fields(payload: &str) -> AnnotationResult<Map<String, Value>> {
    let mut fields = Map::new();

    for segment in split_assignments(payload)? {
        let (key, value) = parse_assignment(&segment)?;
        insert_metadata_field(&mut fields, key, Value::String(value))?;
    }

    Ok(fields)
}

fn parse_assignment(segment: &str) -> AnnotationResult<(String, String)> {
    let (raw_key, raw_value) = segment
        .split_once('=')
        .ok_or_else(AnnotationError::malformed)?;

    let key = raw_key.trim();
    let value = raw_value.trim();

    if key.is_empty()
        || syn::parse_str::<syn::Ident>(key).is_err()
        || !value.starts_with('"')
        || !value.ends_with('"')
        || value.len() < 2
    {
        return Err(AnnotationError::malformed());
    }

    let inner = decode_string_literal(value)?;

    Ok((key.to_string(), inner))
}

fn decode_string_literal(raw_value: &str) -> AnnotationResult<String> {
    syn::parse_str::<syn::LitStr>(raw_value)
        .map(|literal| literal.value())
        .map_err(|_| AnnotationError::malformed())
}
