use crate::model::{Document, WikiLinkToken};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Parse {
    pub(crate) document: Option<Document>,
    pub(crate) wiki_links: Vec<WikiLinkToken>,
    pub(crate) wiki_link_diagnostics: Vec<crate::model::Diagnostic>,
}
