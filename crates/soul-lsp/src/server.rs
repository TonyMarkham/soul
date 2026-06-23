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
        DocumentSymbol, DocumentSymbolParams, DocumentSymbolResponse, GotoDefinitionParams,
        GotoDefinitionResponse, Hover, HoverContents, HoverParams, HoverProviderCapability,
        InitializeParams, InitializeResult, InitializedParams, Location, MarkupContent, MarkupKind,
        MessageType, OneOf, Position, Range, ReferenceParams, ServerCapabilities, SymbolKind,
        TextDocumentSyncCapability, TextDocumentSyncKind, TextDocumentSyncOptions, Uri,
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
    let target = (line + 1) as usize;
    graph
        .annotations
        .iter()
        .find(|a| path_matches_uri(root, uri, &a.path) && a.line == target)
}

fn linked_doc<'g>(graph: &'g SemanticGraph, id: &str) -> Option<&'g Document> {
    graph.documents.iter().find(|d| d.id == id)
}

fn document_at<'g>(graph: &'g SemanticGraph, root: &Path, uri: &Uri) -> Option<&'g Document> {
    graph
        .documents
        .iter()
        .find(|d| path_matches_uri(root, uri, &d.path))
}

fn path_matches_uri(root: &Path, uri: &Uri, path: &Path) -> bool {
    let Some(req_path) = uri.to_file_path() else {
        return false;
    };
    let req_path = req_path.canonicalize().unwrap_or(req_path.to_path_buf());
    let abs = if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    };
    let canon = abs.canonicalize().unwrap_or(abs);
    canon == req_path
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

fn to_u32_saturating(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

fn indexed_line(line: usize) -> u32 {
    to_u32_saturating(line.saturating_sub(1))
}

fn point_range(line: u32) -> Range {
    Range {
        start: Position { line, character: 0 },
        end: Position { line, character: 0 },
    }
}

fn indexed_line_range(line: usize) -> Range {
    point_range(indexed_line(line))
}

#[allow(deprecated)]
fn document_symbol_for_document(
    document: &Document,
    children: Vec<DocumentSymbol>,
) -> DocumentSymbol {
    let range = point_range(0);
    DocumentSymbol {
        name: document.id.clone(),
        detail: document
            .title
            .clone()
            .or_else(|| Some(document.kind.clone())),
        kind: SymbolKind::FILE,
        tags: None,
        deprecated: None,
        range,
        selection_range: range,
        children: if children.is_empty() {
            None
        } else {
            Some(children)
        },
    }
}

#[allow(deprecated)]
fn document_symbol_for_annotation(annotation: &CodeAnnotation) -> DocumentSymbol {
    let range = indexed_line_range(annotation.line);
    DocumentSymbol {
        name: annotation.id.clone(),
        detail: Some(annotation.syntax.to_string()),
        kind: SymbolKind::OBJECT,
        tags: None,
        deprecated: None,
        range,
        selection_range: range,
        children: None,
    }
}

fn range_for_reference(reference: &indexer::Reference) -> Option<Range> {
    if reference.source_end_line < reference.source_start_line {
        return None;
    }
    if reference.source_end_line == reference.source_start_line
        && reference.source_end_col < reference.source_start_col
    {
        return None;
    }

    Some(Range {
        start: Position {
            line: indexed_line(reference.source_start_line),
            character: to_u32_saturating(reference.source_start_col),
        },
        end: Position {
            line: indexed_line(reference.source_end_line),
            character: to_u32_saturating(reference.source_end_col),
        },
    })
}

fn document_symbol_for_reference(reference: &indexer::Reference) -> Option<DocumentSymbol> {
    let range = range_for_reference(reference)?;
    let has_display_text = reference.display_text.is_some();
    #[allow(deprecated)]
    Some(DocumentSymbol {
        name: reference
            .display_text
            .clone()
            .unwrap_or_else(|| reference.target_id.clone()),
        detail: has_display_text.then(|| reference.target_id.clone()),
        kind: SymbolKind::STRING,
        tags: None,
        deprecated: None,
        range,
        selection_range: range,
        children: None,
    })
}

fn document_symbols_for_uri(graph: &SemanticGraph, root: &Path, uri: &Uri) -> Vec<DocumentSymbol> {
    let annotations: Vec<DocumentSymbol> = graph
        .annotations
        .iter()
        .filter(|annotation| path_matches_uri(root, uri, &annotation.path))
        .map(document_symbol_for_annotation)
        .collect();

    let references: Vec<DocumentSymbol> = graph
        .references
        .iter()
        .filter(|reference| path_matches_uri(root, uri, &reference.source_path))
        .filter_map(document_symbol_for_reference)
        .collect();

    if let Some(document) = document_at(graph, root, uri) {
        let mut children = annotations;
        children.extend(references);
        vec![document_symbol_for_document(document, children)]
    } else {
        annotations.into_iter().chain(references).collect()
    }
}

impl LanguageServer for Server {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                definition_provider: Some(OneOf::Left(true)),
                references_provider: Some(OneOf::Left(true)),
                document_symbol_provider: Some(OneOf::Left(true)),
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

    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> Result<Option<DocumentSymbolResponse>> {
        let guard = self.graph.read().await;
        let Some(graph) = guard.as_ref() else {
            return Ok(None);
        };

        Ok(Some(DocumentSymbolResponse::Nested(
            document_symbols_for_uri(graph, &self.root, &params.text_document.uri),
        )))
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

#[cfg(test)]
mod tests {
    use super::*;

    use indexer::model::AnnotationSyntax;
    use std::path::PathBuf;
    use tower_lsp_server::{
        LspService,
        ls_types::{PartialResultParams, TextDocumentIdentifier, WorkDoneProgressParams},
    };

    fn test_root() -> PathBuf {
        std::env::current_dir().expect("current dir")
    }

    fn lsp_service(root: PathBuf) -> LspService<Server> {
        let (service, _) = LspService::new(|client| Server::new(client, root));
        service
    }

    async fn lsp_service_with_graph(root: PathBuf, graph: SemanticGraph) -> LspService<Server> {
        let service = lsp_service(root);
        {
            let mut guard = service.inner().graph.write().await;
            *guard = Some(graph);
        }
        service
    }

    fn document(id: &str, path: &str) -> Document {
        Document {
            id: id.to_string(),
            kind: "interaction".to_string(),
            title: None,
            path: PathBuf::from(path),
        }
    }

    fn annotation(id: &str, path: &str, line: usize) -> CodeAnnotation {
        CodeAnnotation {
            id: id.to_string(),
            metadata: Default::default(),
            path: PathBuf::from(path),
            line,
            syntax: AnnotationSyntax("rust-attribute".to_string()),
            raw: format!("#[soul(id = \"{id}\")]"),
        }
    }

    fn document_symbol_params(root: &Path, path: &str) -> DocumentSymbolParams {
        DocumentSymbolParams {
            text_document: TextDocumentIdentifier {
                uri: Uri::from_file_path(root.join(path)).expect("file uri"),
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        }
    }

    fn nested_symbols(response: Option<DocumentSymbolResponse>) -> Vec<DocumentSymbol> {
        match response.expect("document symbol response") {
            DocumentSymbolResponse::Nested(symbols) => symbols,
            DocumentSymbolResponse::Flat(_) => panic!("expected nested document symbols"),
        }
    }

    async fn symbols_for(root: &Path, graph: SemanticGraph, path: &str) -> Vec<DocumentSymbol> {
        let service = lsp_service_with_graph(root.to_path_buf(), graph).await;
        nested_symbols(
            service
                .inner()
                .document_symbol(document_symbol_params(root, path))
                .await
                .expect("document symbol request"),
        )
    }

    #[tokio::test]
    async fn initialize_advertises_document_symbols() {
        let service = lsp_service(test_root());
        let result = service
            .inner()
            .initialize(InitializeParams::default())
            .await
            .expect("initialize");

        assert!(matches!(
            result.capabilities.document_symbol_provider,
            Some(OneOf::Left(true))
        ));
    }

    #[tokio::test]
    async fn document_file_returns_its_soul_id() {
        let root = test_root();
        let graph = SemanticGraph {
            documents: vec![document(
                "interaction.checkout.create-order",
                "docs/checkout.md",
            )],
            ..Default::default()
        };

        let symbols = symbols_for(&root, graph, "docs/checkout.md").await;

        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "interaction.checkout.create-order");
        assert_eq!(symbols[0].kind, SymbolKind::FILE);
    }

    #[tokio::test]
    async fn code_annotation_file_returns_annotation_ids() {
        let root = test_root();
        let graph = SemanticGraph {
            annotations: vec![
                annotation("interaction.checkout.create-order", "src/checkout.rs", 3),
                annotation("policy.audit.log", "src/checkout.rs", 9),
            ],
            ..Default::default()
        };

        let symbols = symbols_for(&root, graph, "src/checkout.rs").await;
        let names: Vec<&str> = symbols.iter().map(|symbol| symbol.name.as_str()).collect();

        assert_eq!(
            names,
            vec!["interaction.checkout.create-order", "policy.audit.log"]
        );
        assert!(
            symbols
                .iter()
                .all(|symbol| symbol.kind == SymbolKind::OBJECT)
        );
    }

    #[tokio::test]
    async fn unrelated_file_returns_empty_document_symbol_result() {
        let root = test_root();
        let graph = SemanticGraph {
            documents: vec![document(
                "interaction.checkout.create-order",
                "docs/checkout.md",
            )],
            annotations: vec![annotation(
                "interaction.checkout.create-order",
                "src/checkout.rs",
                3,
            )],
            ..Default::default()
        };

        let symbols = symbols_for(&root, graph, "src/unrelated.rs").await;

        assert!(symbols.is_empty());
    }
}
