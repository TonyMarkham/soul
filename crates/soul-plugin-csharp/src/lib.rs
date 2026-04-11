use soul_plugin_sdk::{
    AnnotationError, AnnotationParser, AnnotationParser_TO, AnnotationResult, NormalizedAnnotation,
    SoulPlugin, SoulPluginRef,
    helpers::{
        insert_metadata_field, merge_metadata_json, normalized_annotation_from_fields,
        split_assignments,
    },
};

use abi_stable::{
    export_root_module,
    prefix_type::PrefixTypeTrait,
    sabi_trait::TD_Opaque,
    std_types::{RBox, ROption, RResult, RStr, RString},
};
use serde_json::{Map, Value};

// ---------------------------------------------------------------------------------------------- //

#[export_root_module]
pub fn get_root_module() -> SoulPluginRef {
    SoulPlugin { parser: new_parser }.leak_into_prefix()
}

extern "C" fn new_parser() -> AnnotationParser_TO<'static, RBox<()>> {
    AnnotationParser_TO::from_value(CSharpParser, TD_Opaque)
}

// ---------------------------------------------------------------------------------------------- //

struct CSharpParser;

impl AnnotationParser for CSharpParser {
    fn extension(&self) -> RString {
        "cs".into()
    }

    fn syntax(&self) -> RString {
        "csharp-attribute".into()
    }

    fn parse_line(
        &self,
        line: RStr<'_>,
    ) -> ROption<RResult<NormalizedAnnotation, AnnotationError>> {
        let trimmed = line.trim();
        if !trimmed.starts_with("[Soul(") {
            return ROption::RNone;
        }

        if !trimmed.ends_with(")]") {
            return ROption::RSome(RResult::RErr(AnnotationError::malformed()));
        }

        let payload = &trimmed["[Soul(".len()..trimmed.len() - 2];
        let result = parse_fields(payload)
            .and_then(|fields| normalized_annotation_from_fields(fields, trimmed));

        ROption::RSome(match result {
            Ok(ann) => RResult::ROk(ann),
            Err(e) => RResult::RErr(e),
        })
    }

    fn syntax_guidance(&self) -> RString {
        r#"C# soul annotation syntax

  Template:
    [Soul("<id>")]

  Placement: on the line immediately above the class, method, or type it annotates.

  Example:
    [Soul("interaction.checkout.create-order")]
    public Task<Order> CreateOrder(...) { ... }

  With metadata:
    [Soul("interaction.checkout.create-order", MetadataJson = "{\"layer\":\"frontend\"}")]

  Rules:
  - The id is required and must be the first argument, as a quoted string
  - Additional metadata is passed via MetadataJson as a JSON object string
  - MetadataJson may appear at most once"#
            .into()
    }
}

// ---------------------------------------------------------------------------------------------- //

fn parse_fields(payload: &str) -> AnnotationResult<Map<String, Value>> {
    let mut segments = split_assignments(payload)?.into_iter();
    let Some(first) = segments.next() else {
        return Err(AnnotationError::missing_id());
    };
    if !first.trim_start().starts_with('"') {
        return Err(AnnotationError::missing_id());
    }

    let mut fields = Map::new();
    let mut metadata_json_seen = false;
    insert_metadata_field(
        &mut fields,
        "id".to_string(),
        Value::String(parse_string_literal(&first)?),
    )?;

    for segment in segments {
        let (key, value) = parse_assignment(&segment)?;
        match key.as_str() {
            "MetadataJson" => {
                if metadata_json_seen {
                    return Err(AnnotationError::malformed());
                }
                metadata_json_seen = true;
                merge_metadata_json(&mut fields, &value)?;
            }
            _ => insert_metadata_field(&mut fields, key.to_lowercase(), Value::String(value))?,
        }
    }

    Ok(fields)
}

fn parse_assignment(segment: &str) -> AnnotationResult<(String, String)> {
    let (raw_key, raw_value) = segment
        .split_once('=')
        .ok_or_else(AnnotationError::malformed)?;

    let key = raw_key.trim();
    let value = raw_value.trim();

    if key.is_empty() || !value.starts_with('"') || !value.ends_with('"') || value.len() < 2 {
        return Err(AnnotationError::malformed());
    }

    Ok((key.to_string(), parse_string_literal(value)?))
}

fn parse_string_literal(raw_value: &str) -> AnnotationResult<String> {
    let raw_value = raw_value.trim();

    if !raw_value.starts_with('"') || !raw_value.ends_with('"') {
        return Err(AnnotationError::malformed());
    }

    decode_string_literal(raw_value)
}

fn decode_string_literal(raw_value: &str) -> AnnotationResult<String> {
    let inner = &raw_value[1..raw_value.len() - 1];
    let mut decoded = String::new();
    let mut chars = inner.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch != '\\' {
            decoded.push(ch);
            continue;
        }

        let Some(escape) = chars.next() else {
            return Err(AnnotationError::malformed());
        };

        match escape {
            '\\' => decoded.push('\\'),
            '"' => decoded.push('"'),
            '\'' => decoded.push('\''),
            '0' => decoded.push('\0'),
            'a' => decoded.push('\u{0007}'),
            'b' => decoded.push('\u{0008}'),
            'e' => decoded.push('\u{001B}'),
            'f' => decoded.push('\u{000C}'),
            'n' => decoded.push('\n'),
            'r' => decoded.push('\r'),
            't' => decoded.push('\t'),
            'v' => decoded.push('\u{000B}'),
            'x' => decoded.push(parse_hex_escape(&mut chars, 1, 4)?),
            'u' => decoded.push(parse_hex_escape(&mut chars, 4, 4)?),
            'U' => decoded.push(parse_hex_escape(&mut chars, 8, 8)?),
            _ => return Err(AnnotationError::malformed()),
        }
    }

    Ok(decoded)
}

fn parse_hex_escape(
    chars: &mut std::iter::Peekable<std::str::Chars<'_>>,
    min_digits: usize,
    max_digits: usize,
) -> AnnotationResult<char> {
    let mut digits = String::new();

    while digits.len() < max_digits {
        let Some(next) = chars.peek().copied() else {
            break;
        };

        if !next.is_ascii_hexdigit() {
            break;
        }

        digits.push(next);
        chars.next();
    }

    if digits.len() < min_digits {
        return Err(AnnotationError::malformed());
    }

    let value = u32::from_str_radix(&digits, 16).map_err(|_| AnnotationError::malformed())?;
    char::from_u32(value).ok_or_else(AnnotationError::malformed)
}
