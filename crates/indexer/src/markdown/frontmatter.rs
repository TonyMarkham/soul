use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(crate) struct Frontmatter {
    pub(crate) id: Option<String>,
    pub(crate) kind: Option<String>,
    pub(crate) title: Option<String>,
}
