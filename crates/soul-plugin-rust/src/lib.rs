use soul_plugin_sdk::{
    AnnotationError, AnnotationParser, AnnotationParser_TO, AnnotationResult, NormalizedAnnotation,
    SoulPlugin, SoulPluginRef,
    helpers::{insert_metadata_field, normalized_annotation_from_fields, split_assignments},
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
    AnnotationParser_TO::from_value(RustParser, TD_Opaque)
}

// ---------------------------------------------------------------------------------------------- //

struct RustParser;

impl AnnotationParser for RustParser {
    fn extension(&self) -> RString {
        "rs".into()
    }

    fn syntax(&self) -> RString {
        "rust-attribute".into()
    }

    fn parse_line(
        &self,
        line: RStr<'_>,
    ) -> ROption<RResult<NormalizedAnnotation, AnnotationError>> {
        let trimmed = line.trim();
        if !trimmed.starts_with("#[soul(") {
            return ROption::RNone;
        }

        if !trimmed.ends_with(")]") {
            return ROption::RSome(RResult::RErr(AnnotationError::malformed()));
        }

        let payload = &trimmed["#[soul(".len()..trimmed.len() - 2];
        let result = parse_fields(payload)
            .and_then(|fields| normalized_annotation_from_fields(fields, trimmed));

        ROption::RSome(match result {
            Ok(ann) => RResult::ROk(ann),
            Err(e) => RResult::RErr(e),
        })
    }

    fn syntax_guidance(&self) -> RString {
        r#"Rust soul annotation syntax

  Template:
    #[soul(id = "<id>")]

  Placement: on the line immediately above the function, method, or type it annotates.

  Example:
    #[soul(id = "interaction.checkout.create-order")]
    pub fn create_order(...) { ... }

  Rules:
  - `id` is required; all other keys are optional metadata
  - Values must be quoted strings: key = "value"
  - Multiple keys are comma-separated: #[soul(id = "x", layer = "backend")]
  - The annotation must be on its own line (not inline with other attributes on the same line)"#
            .into()
    }
}

// ---------------------------------------------------------------------------------------------- //

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
