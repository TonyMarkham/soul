use indexer::{
    CodeAnnotation, Document, SemanticGraph,
    index::{load_graph, open_index},
};

use std::path::{Path, PathBuf};
use tokio::sync::RwLock;
use tower_lsp_server::{
    Client, LanguageServer,
    jsonrpc::Result,
    ls_types::{
        GotoDefinitionParams, GotoDefinitionResponse, Hover, HoverContents, HoverParams,
        HoverProviderCapability, InitializeParams, InitializeResult, InitializedParams, Location,
        MarkupContent, MarkupKind, MessageType, OneOf, Position, Range, ReferenceParams,
        ServerCapabilities, Uri,
    },
};

pub struct Server {
    client: Client,
    root: PathBuf,
    graph: RwLock<Option<SemanticGraph>>,
}

impl Server {
    pub fn new(client: Client, root: PathBuf) -> Self {
        Self {
            client,
            root,
            graph: RwLock::new(None),
        }
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

fn to_uri(root: &Path, path: &Path) -> Option<Uri> {
    let abs = if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    };
    let canon = abs.canonicalize().unwrap_or(abs);
    Uri::from_file_path(canon)
}

impl LanguageServer for Server {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                definition_provider: Some(OneOf::Left(true)),
                references_provider: Some(OneOf::Left(true)),
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

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let guard = self.graph.read().await;
        let Some(graph) = guard.as_ref() else {
            return Ok(None);
        };
        let pos = params.text_document_position_params;
        let Some(ann) = annotation_at(graph, &self.root, &pos.text_document.uri, pos.position.line)
        else {
            return Ok(None);
        };
        let mut md = format!("**{}**", ann.id);
        if let Some(doc) = linked_doc(graph, &ann.id) {
            if let Some(title) = &doc.title {
                md.push_str(&format!("\n\n*{title}*"));
            }
            md.push_str(&format!("\n\n`{}`", doc.path.display()));
        }
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
        let Some(ann) = annotation_at(graph, &self.root, &pos.text_document.uri, pos.position.line)
        else {
            return Ok(None);
        };
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
    }

    async fn references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        let guard = self.graph.read().await;
        let Some(graph) = guard.as_ref() else {
            return Ok(None);
        };
        let pos = params.text_document_position;
        let Some(ann) = annotation_at(graph, &self.root, &pos.text_document.uri, pos.position.line)
        else {
            return Ok(None);
        };
        let id = ann.id.clone();
        let locations: Vec<Location> = graph
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
