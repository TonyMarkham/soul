use indexer::{
    CodeAnnotation, Document, SemanticGraph, WikiLinkToken,
    index::{load_graph, open_index},
    markdown::wikilink_at_position,
};

use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};
use tokio::sync::RwLock;
use tower_lsp_server::{
    Client, LanguageServer,
    jsonrpc::Result,
    ls_types::{
        DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
        GotoDefinitionParams, GotoDefinitionResponse, Hover, HoverContents, HoverParams,
        HoverProviderCapability, InitializeParams, InitializeResult, InitializedParams, Location,
        MarkupContent, MarkupKind, MessageType, OneOf, Position, Range, ReferenceParams,
        ServerCapabilities, TextDocumentSyncCapability, TextDocumentSyncKind,
        TextDocumentSyncOptions, Uri,
    },
};

pub struct Server {
    client: Client,
    root: PathBuf,
    graph: RwLock<Option<SemanticGraph>>,
    open_documents: RwLock<BTreeMap<Uri, String>>,
}

impl Server {
    pub fn new(client: Client, root: PathBuf) -> Self {
        Self {
            client,
            root,
            graph: RwLock::new(None),
            open_documents: RwLock::new(BTreeMap::new()),
        }
    }

    async fn wikilink_token_at_position(
        &self,
        uri: &Uri,
        position: Position,
    ) -> Option<WikiLinkToken> {
        let open_documents = self.open_documents.read().await;
        let text = open_documents.get(uri)?;
        wikilink_at_position(
            text,
            (position.line as usize) + 1,
            position.character as usize,
        )
    }

    async fn resolved_id_at_position(
        &self,
        graph: &SemanticGraph,
        uri: &Uri,
        position: Position,
    ) -> Option<String> {
        let wikilink_token = self.wikilink_token_at_position(uri, position).await;
        resolved_id_from_position_inputs(graph, &self.root, uri, position, wikilink_token)
    }
}

fn annotation_at<'g>(
    graph: &'g SemanticGraph,
    root: &Path,
    uri: &Uri,
    line: u32,
) -> Option<&'g CodeAnnotation> {
    let req_path = uri.to_file_path()?;
    let req_path = req_path.canonicalize().unwrap_or(req_path.to_path_buf());
    let target = (line + 1) as usize;
    graph.annotations.iter().find(|a| {
        let abs = if a.path.is_absolute() {
            a.path.clone()
        } else {
            root.join(&a.path)
        };
        let canon = abs.canonicalize().unwrap_or(abs);
        canon == req_path && a.line == target
    })
}

fn linked_doc<'g>(graph: &'g SemanticGraph, id: &str) -> Option<&'g Document> {
    graph.documents.iter().find(|d| d.id == id)
}

fn document_at<'g>(graph: &'g SemanticGraph, root: &Path, uri: &Uri) -> Option<&'g Document> {
    let req_path = uri.to_file_path()?;
    let req_path = req_path.canonicalize().unwrap_or(req_path.to_path_buf());
    graph.documents.iter().find(|d| {
        let abs = if d.path.is_absolute() {
            d.path.clone()
        } else {
            root.join(&d.path)
        };
        let canon = abs.canonicalize().unwrap_or(abs);
        canon == req_path
    })
}

pub(crate) fn resolved_id_from_position_inputs(
    graph: &SemanticGraph,
    root: &Path,
    uri: &Uri,
    position: Position,
    wikilink_token: Option<WikiLinkToken>,
) -> Option<String> {
    if let Some(token) = wikilink_token {
        return Some(token.target_id);
    }

    if let Some(annotation) = annotation_at(graph, root, uri, position.line) {
        return Some(annotation.id.clone());
    }

    document_at(graph, root, uri).map(|document| document.id.clone())
}

/// On Windows, `Path::canonicalize` returns extended-length UNC paths
/// (`\\?\C:\…`).  `Uri::from_file_path` then percent-encodes the `?` and
/// produces `file://///%3F/C%3A/…` instead of `file:///C:/…`.  Strip the
/// prefix so the URI is well-formed before handing it back to the client.
fn strip_unc_prefix(path: PathBuf) -> PathBuf {
    #[cfg(windows)]
    {
        let s = path.to_string_lossy();
        if let Some(stripped) = s.strip_prefix(r"\\?\") {
            return PathBuf::from(stripped);
        }
    }
    path
}

fn to_uri(root: &Path, path: &Path) -> Option<Uri> {
    let abs = if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    };
    let canon = strip_unc_prefix(abs.canonicalize().unwrap_or(abs));
    Uri::from_file_path(canon)
}

impl LanguageServer for Server {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                definition_provider: Some(OneOf::Left(true)),
                references_provider: Some(OneOf::Left(true)),
                text_document_sync: Some(TextDocumentSyncCapability::Options(
                    TextDocumentSyncOptions {
                        open_close: Some(true),
                        change: Some(TextDocumentSyncKind::FULL),
                        ..Default::default()
                    },
                )),
                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        match open_index(&self.root).await {
            Ok(Some(pool)) => match load_graph(&pool).await {
                Ok(g) => {
                    *self.graph.write().await = Some(g);
                }
                Err(e) => {
                    self.client
                        .log_message(
                            MessageType::ERROR,
                            format!("soul-lsp: load_graph failed: {e}"),
                        )
                        .await;
                }
            },
            Ok(None) => {
                self.client
                    .log_message(
                        MessageType::WARNING,
                        "soul-lsp: no index found — run `indexer index` first",
                    )
                    .await;
            }
            Err(e) => {
                self.client
                    .log_message(
                        MessageType::ERROR,
                        format!("soul-lsp: open_index failed: {e}"),
                    )
                    .await;
            }
        }
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let mut open_documents = self.open_documents.write().await;
        open_documents.insert(params.text_document.uri, params.text_document.text);
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let mut open_documents = self.open_documents.write().await;
        if let Some(change) = params.content_changes.last() {
            open_documents.insert(params.text_document.uri, change.text.clone());
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let mut open_documents = self.open_documents.write().await;
        open_documents.remove(&params.text_document.uri);
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let guard = self.graph.read().await;
        let Some(graph) = guard.as_ref() else {
            return Ok(None);
        };
        let pos = params.text_document_position_params;
        let md = if let Some(ann) =
            annotation_at(graph, &self.root, &pos.text_document.uri, pos.position.line)
        {
            let mut md = format!("**{}**", ann.id);
            if let Some(doc) = linked_doc(graph, &ann.id) {
                if let Some(title) = &doc.title {
                    md.push_str(&format!("\n\n*{title}*"));
                }
                md.push_str(&format!("\n\n`{}`", doc.path.display()));
            }
            md
        } else if let Some(doc) = document_at(graph, &self.root, &pos.text_document.uri) {
            let mut md = format!("**{}**", doc.id);
            if let Some(title) = &doc.title {
                md.push_str(&format!("\n\n*{title}*"));
            }
            let locs: Vec<_> = graph
                .annotations
                .iter()
                .filter(|a| a.id == doc.id)
                .collect();
            if !locs.is_empty() {
                md.push_str(&format!("\n\n{} code location(s):", locs.len()));
                for a in &locs {
                    md.push_str(&format!("\n- `{}:{}`", a.path.display(), a.line));
                }
            }
            md
        } else {
            return Ok(None);
        };
        Ok(Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: md,
            }),
            range: None,
        }))
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let guard = self.graph.read().await;
        let Some(graph) = guard.as_ref() else {
            return Ok(None);
        };
        let pos = params.text_document_position_params;
        let locations_from_id = |id: &str| -> Option<GotoDefinitionResponse> {
            let locs: Vec<Location> = graph
                .annotations
                .iter()
                .filter(|a| a.id == id)
                .filter_map(|a| {
                    let uri = to_uri(&self.root, &a.path)?;
                    let line = (a.line as u32).saturating_sub(1);
                    Some(Location {
                        uri,
                        range: Range {
                            start: Position { line, character: 0 },
                            end: Position { line, character: 0 },
                        },
                    })
                })
                .collect();
            if locs.is_empty() {
                None
            } else {
                Some(GotoDefinitionResponse::Array(locs))
            }
        };

        if let Some(ann) =
            annotation_at(graph, &self.root, &pos.text_document.uri, pos.position.line)
        {
            let Some(doc) = linked_doc(graph, &ann.id) else {
                return Ok(None);
            };
            let Some(uri) = to_uri(&self.root, &doc.path) else {
                return Ok(None);
            };
            Ok(Some(GotoDefinitionResponse::Scalar(Location {
                uri,
                range: Range::default(),
            })))
        } else if let Some(doc) = document_at(graph, &self.root, &pos.text_document.uri) {
            Ok(locations_from_id(&doc.id))
        } else {
            Ok(None)
        }
    }

    async fn references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        let guard = self.graph.read().await;
        let Some(graph) = guard.as_ref() else {
            return Ok(None);
        };

        let pos = params.text_document_position;
        let Some(id) = self
            .resolved_id_at_position(graph, &pos.text_document.uri, pos.position)
            .await
        else {
            return Ok(None);
        };

        let mut locations: Vec<Location> = graph
            .annotations
            .iter()
            .filter(|a| a.id == id)
            .filter_map(|a| {
                let uri = to_uri(&self.root, &a.path)?;
                let line = (a.line as u32).saturating_sub(1);
                Some(Location {
                    uri,
                    range: Range {
                        start: Position { line, character: 0 },
                        end: Position { line, character: 0 },
                    },
                })
            })
            .collect();

        locations.extend(
            graph
                .references
                .iter()
                .filter(|r| r.target_id == id)
                .filter_map(|reference| {
                    let uri = to_uri(&self.root, &reference.source_path)?;
                    let start_line = (reference.source_start_line as u32).saturating_sub(1);
                    let end_line = (reference.source_end_line as u32).saturating_sub(1);
                    Some(Location {
                        uri,
                        range: Range {
                            start: Position {
                                line: start_line,
                                character: reference.source_start_col as u32,
                            },
                            end: Position {
                                line: end_line,
                                character: reference.source_end_col as u32,
                            },
                        },
                    })
                }),
        );

        Ok(if locations.is_empty() {
            None
        } else {
            Some(locations)
        })
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}
