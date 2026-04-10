pub mod explain_params;
pub mod format;

use crate::{
    IndexerResult, SemanticGraph,
    config::load_config,
    graph::explain,
    index::{explain_from_index, load_graph, open_index, write_index},
    mcp::explain_params::ExplainParams,
    scan_repository,
};

use rmcp::{
    ServerHandler,
    handler::server::wrapper::Parameters,
    model::{CallToolResult, Content},
    tool, tool_handler, tool_router,
};
use std::path::PathBuf;

fn err_result(e: impl std::fmt::Display) -> CallToolResult {
    CallToolResult::error(vec![Content::text(e.to_string())])
}

#[derive(Clone)]
pub struct SoulServer {
    root: PathBuf,
}

impl SoulServer {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }
}

#[tool_router]
impl SoulServer {
    #[tool(
        description = "Scan the repository and write the semantic index to .soul/index.db.

Soul works by linking Markdown documents and source code together via a shared `id`. Documents are Markdown files with frontmatter:

  ---
  id: interaction.checkout.create-order
  kind: interaction
  title: Create order
  ---
  (specification body follows)

Source files carry lightweight annotations that reference the same id:
  Rust:  #[soul(id = \"interaction.checkout.create-order\", role = \"backend\")]
  C#:    [Soul(\"interaction.checkout.create-order\", Role = \"frontend\")]

Because Soul is language-agnostic, a single ID links a specification document, a Rust backend handler, and a C# frontend component — all as facets of the same feature. Soul ties them together so you can see what is documented, what implements it across every layer and language, and what is missing.

Prerequisites: the repository must have been initialised, which creates `.soul/soul.toml`. If soul_index returns a config error, run `.soul/indexer init` as a shell command in the repository root first, then retry.

Run this tool at the start of a session, or any time documents or source files have changed. Returns a count of documents, annotations, and diagnostics found. Diagnostics are warnings about malformed annotations or unreadable files — they do not abort the scan, but a high count may indicate annotation syntax problems worth investigating.

Recommended workflow:
1. soul_index — build or refresh the index
2. soul_list_documents — discover what is documented and find IDs to explore
3. soul_list_gaps — find what is missing: undocumented code or unlinked specs
4. soul_explain <id> — read the detail for a specific ID and locate its code across all layers and languages
5. Read the document file path returned by soul_explain to get the full written specification"
    )]
    async fn soul_index(&self) -> CallToolResult {
        self.soul_index_impl().await.unwrap_or_else(err_result)
    }

    #[tool(description = "Return everything Soul knows about a given ID.

An ID is a dot-separated semantic identifier like `interaction.checkout.create-order`. The result has two sections:

Documents: metadata only — kind (e.g. interaction, concept, policy), title, and file path. This is NOT the full specification. Each document entry includes a prompt to read the file — follow it immediately using the Read tool to get the actual written specification. The specification body is the Markdown content after the YAML frontmatter block.

Annotations: every location in the codebase tagged with this ID — file path, line number, and role. Soul is language-agnostic: a Rust Axum route handler and a C# Blazor component can both carry the same ID with different roles, and soul_explain returns both. The role field (e.g. 'backend', 'frontend', 'worker') identifies which layer or component each annotation belongs to. Together, the full set of annotations for an ID gives you the complete cross-language, cross-layer picture of how a feature is realised — navigate to each file and line to read the implementation at each layer.

If the ID does not exist, the tool returns a message saying no documents or annotations were found — verify the ID using soul_list_documents first. If no index exists yet, it falls back to a live scan.")]
    async fn soul_explain(&self, Parameters(p): Parameters<ExplainParams>) -> CallToolResult {
        self.soul_explain_impl(p).await.unwrap_or_else(err_result)
    }

    #[tool(
        description = "List every document in the index with its ID, kind, title, and file path.

Output format per line: `<id> [<kind>] <title> — <path>`
Example: `interaction.checkout.create-order [interaction] Create order — docs/checkout/create-order.md`

Use this to explore what is documented in the repository and to discover IDs you can pass to soul_explain. The kind field describes the category of the document (e.g. interaction, concept, policy, decision). The ID is the shared key that links a document to its code annotations across all languages and layers — a single document can be the specification for a Rust backend, a C# frontend, and any other annotated layer simultaneously. Not all annotated IDs will have a document — use soul_list_gaps to find those."
    )]
    async fn soul_list_documents(&self) -> CallToolResult {
        self.soul_list_documents_impl()
            .await
            .unwrap_or_else(err_result)
    }

    #[tool(description = "Find mismatches between documentation and code.

Returns two lists:

Unlinked annotations: IDs that appear in source code annotations but have no corresponding document. The code claims to implement something that has never been written up. These need documentation created. Note that an ID may have annotations in multiple languages (e.g. a Rust backend and a C# frontend) — an unlinked annotation means none of those implementations have a matching document yet.

Undocumented IDs: IDs that appear in documents but have no corresponding annotation anywhere in the codebase. The specification exists but nothing in the code is linked to it — either the annotations are missing, or the code has not been written yet. Note: this tool detects complete absence only. If a feature has a Rust annotation but no C# annotation, it will NOT appear here. To check which layers are covered for a given ID, call soul_explain on it and inspect the roles present in the annotations.

Use this as your starting point for documentation and coverage work. Pick an ID from either list, call soul_explain on it to see what exists across all layers and languages, then write the missing document or add the missing annotations.")]
    async fn soul_list_gaps(&self) -> CallToolResult {
        self.soul_list_gaps_impl().await.unwrap_or_else(err_result)
    }
}

#[tool_handler(name = "soul")]
impl ServerHandler for SoulServer {}

impl SoulServer {
    async fn soul_index_impl(&self) -> IndexerResult<CallToolResult> {
        let config = load_config(&self.root)?;
        let graph = scan_repository(&self.root, &config)?;
        write_index(&self.root, &graph).await?;
        Ok(CallToolResult::success(vec![Content::text(format!(
            "Indexed {} documents, {} annotations, {} diagnostics.",
            graph.documents.len(),
            graph.annotations.len(),
            graph.diagnostics.len()
        ))]))
    }

    async fn soul_explain_impl(&self, p: ExplainParams) -> IndexerResult<CallToolResult> {
        let result = match open_index(&self.root).await? {
            Some(pool) => explain_from_index(&pool, &p.id).await?,
            None => {
                let config = load_config(&self.root)?;
                let graph = scan_repository(&self.root, &config)?;
                explain(&graph, &p.id)
            }
        };
        Ok(CallToolResult::success(vec![Content::text(
            format::explain_result(&result),
        )]))
    }

    async fn soul_list_documents_impl(&self) -> IndexerResult<CallToolResult> {
        let graph = self.load_graph_or_scan().await?;
        let lines: Vec<String> = graph
            .documents
            .iter()
            .map(|d| {
                format!(
                    "{} [{}] {} — {}",
                    d.id,
                    d.kind,
                    d.title.as_deref().unwrap_or("(untitled)"),
                    d.path.display()
                )
            })
            .collect();
        Ok(CallToolResult::success(vec![Content::text(
            lines.join("\n"),
        )]))
    }

    async fn soul_list_gaps_impl(&self) -> IndexerResult<CallToolResult> {
        let graph = self.load_graph_or_scan().await?;
        Ok(CallToolResult::success(vec![Content::text(format::gaps(
            &graph,
        ))]))
    }

    async fn load_graph_or_scan(&self) -> IndexerResult<SemanticGraph> {
        match open_index(&self.root).await? {
            Some(pool) => load_graph(&pool).await,
            None => {
                let config = load_config(&self.root)?;
                scan_repository(&self.root, &config)
            }
        }
    }
}
