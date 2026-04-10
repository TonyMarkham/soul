pub(crate) enum FrontmatterBlock<'a> {
    Absent,
    Unterminated,
    Present(&'a str),
}
