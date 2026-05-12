pub(crate) const MAX_REFERENCE_DISPLAY_BYTES: usize = 1024;

pub(crate) fn is_valid_reference_target_id(input: &str) -> bool {
    let target = input;
    !target.is_empty()
        && target == target.trim()
        && target
            .chars()
            .all(|ch| !ch.is_whitespace() && !ch.is_control() && !matches!(ch, '[' | ']' | '|'))
}

pub(crate) fn normalize_reference_display_text(input: &str) -> Option<String> {
    let display_text = input.trim();
    (!display_text.is_empty()).then(|| display_text.to_string())
}

pub(crate) fn is_valid_reference_display_text(input: &str) -> bool {
    input.trim().len() <= MAX_REFERENCE_DISPLAY_BYTES
}
