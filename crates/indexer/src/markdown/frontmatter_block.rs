pub(crate) enum FrontmatterBlock<'a> {
    Absent {
        body: &'a str,
        body_start_line: usize,
    },
    Unterminated,
    Present {
        frontmatter: &'a str,
        body: &'a str,
        body_start_line: usize,
    },
}
