use crate::{AnnotationError, AnnotationResult, NormalizedAnnotation};

use abi_stable::std_types::{RHashMap, RString};
use serde::de::{Deserializer, MapAccess, Visitor};
use serde_json::{Map, Value};
use std::fmt;

pub fn split_assignments(payload: &str) -> AnnotationResult<Vec<String>> {
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

pub fn insert_metadata_field(
    fields: &mut Map<String, Value>,
    key: String,
    value: Value,
) -> AnnotationResult<()> {
    if fields.insert(key, value).is_some() {
        return Err(AnnotationError::malformed());
    }
    Ok(())
}

pub fn merge_metadata_json(fields: &mut Map<String, Value>, json: &str) -> AnnotationResult<()> {
    let metadata = parse_unique_json_object(json)?;

    for (key, value) in metadata {
        if key == "id" || fields.contains_key(&key) {
            return Err(AnnotationError::malformed());
        }
        fields.insert(key, value);
    }

    Ok(())
}

/// Extracts `id` from `fields`; all remaining entries flow into metadata.
/// `role` is no longer extracted — if present it becomes an ordinary metadata key.
pub fn normalized_annotation_from_fields(
    mut fields: Map<String, Value>,
    raw: &str,
) -> AnnotationResult<NormalizedAnnotation> {
    let id = take_trimmed_string(&mut fields, "id").ok_or_else(AnnotationError::missing_id)?;

    let metadata: RHashMap<RString, RString> = fields
        .into_iter()
        .map(|(k, v)| {
            let s = match &v {
                Value::String(s) => s.clone(),
                other => other.to_string(),
            };
            (k.into(), s.into())
        })
        .collect();

    Ok(NormalizedAnnotation {
        id: id.into(),
        metadata,
        raw: raw.into(),
    })
}

pub fn parse_unique_json_object(json: &str) -> AnnotationResult<Map<String, Value>> {
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

pub fn take_trimmed_string(fields: &mut Map<String, Value>, key: &str) -> Option<String> {
    match fields.remove(key) {
        Some(Value::String(value)) => {
            let trimmed = value.trim();
            (!trimmed.is_empty()).then(|| trimmed.to_string())
        }
        Some(_) | None => None,
    }
}
