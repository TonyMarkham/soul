# PRD: Repo Soul System (LSP + MCP + Semantic Indexer)

## 1. Overview

The Repo Soul System is a developer toolchain that gives a codebase **semantic awareness and memory**, enabling:

* Cross-language understanding (Rust backend ↔ C# frontend)
* Integration of documentation (Markdown with frontmatter)
* Alignment with coding standards
* LLM-assisted development with deep contextual understanding

The system creates a **unified semantic graph** of a repository that connects:

* Source code
* Documentation
* Architectural intent
* Developer-authored annotations

This graph is exposed through:

* **LSP** → IDE integration (hover, navigation, diagnostics)
* **MCP** → LLM integration (query, reasoning, suggestions)

---

## 2. Problem Statement

Modern codebases suffer from:

* Loss of architectural intent over time
* Weak connections between frontend, backend, and docs
* LLMs lacking deep repo-specific context
* Repetitive correction of LLM-generated code
* Documentation that is disconnected from actual code usage

Developers must manually maintain mental models that are:

* Not encoded
* Not searchable
* Not shared with tools or LLMs

---

## 3. Goals

### Primary Goals

* Provide a **single semantic layer (“repo soul”)** for the entire repository
* Enable **bidirectional linking** between:

    * frontend code
    * backend code
    * documentation
* Allow **LLMs to access structured repo intent**
* Improve **code quality and consistency** through standards enforcement
* Make **documentation actionable and connected**

### Secondary Goals

* Enable **refactor-safe linking**
* Provide **issue tracking with semantic awareness**
* Encourage **better documentation habits**
* Support **multi-tenant SaaS development workflows**

---

## 4. Non-Goals (Initial)

* Full static analysis replacement for language servers
* Runtime behavior injection
* Full-blown issue tracking system replacement (e.g., Jira)
* Heavy compile-time enforcement
* Distributed/shared indexing (local-first design)

---

## 5. Core Concepts

### 5.1 Repo Soul

The "soul" of a repo is the **semantic graph** connecting:

* interactions
* invariants
* concepts
* decisions
* code symbols
* documentation

---

### 5.2 Frontmatter DSL

Markdown documents define canonical entities using YAML:

```yaml
id: interaction.checkout.create-order
kind: interaction
```

These documents are the **primary source of truth** for meaning.

---

### 5.3 Code Annotations (`@soul`)

Code is linked into the semantic graph via structured comments:

```rust
// @soul {"id":"interaction.checkout.create-order","role":"backend"}
```

```csharp
// @soul {"id":"interaction.checkout.create-order","role":"frontend"}
```

---

### 5.4 Semantic Graph

The system builds a graph of:

* documents (nodes)
* code symbols (nodes)
* relationships (edges)

Examples:

* backend handler → interaction
* frontend page → interaction
* interaction → invariant
* doc → doc (wiki links)

---

### 5.5 Coding Standards Integration

A shared repository of coding standards is indexed alongside the project.

Example rule:

* “Use typed errors instead of `anyhow` in backend domain logic”

These rules:

* generate diagnostics
* inform LLM suggestions
* guide code consistency

---

### 5.6 Issue & Disposition Layer

Diagnostics can be tracked with dispositions:

* unreviewed
* fix_later
* won_t_fix
* resolved

Stored as version-controlled text.

---

## 6. System Components

### 6.1 Indexer (Rust)

Responsibilities:

* Parse:

    * Markdown + frontmatter
    * Rust source
    * C# source
* Extract:

    * IDs
    * annotations
    * links
* Build:

    * semantic graph
    * search index (e.g., Tantivy)

Properties:

* local-first
* incremental rebuild
* deterministic
* disposable cache

---

### 6.2 LSP Server

Provides IDE features:

* hover → show linked context
* go to definition → jump to doc
* find usages → across code + docs
* rename → refactor IDs safely
* diagnostics → unresolved links, standards violations

---

### 6.3 MCP Server

Provides LLM tools:

* `explain_interaction(id)`
* `find_related_symbols(id)`
* `list_issues()`
* `suggest_fix(issue_id)`

Enables LLM to:

* reason about repo intent
* generate aligned code
* audit code quality

---

## 7. Key Workflows

### 7.1 Navigation

* Developer hovers over a function
* LSP shows:

    * linked interaction
    * frontend/backend counterparts
    * relevant docs

---

### 7.2 LLM Query

* Developer asks:

    * “Explain how checkout works”
* MCP returns:

    * interaction doc
    * related code
    * constraints/invariants

---

### 7.3 Issue Audit

* Developer asks:

    * “Audit current issues”
* MCP returns:

    * unresolved issues
    * grouped by severity and disposition

---

### 7.4 Authoring

* Right-click symbol → create linked doc/issue
* Generates Markdown with frontmatter
* Reindexed automatically

---

## 8. Data Principles

### Source of Truth

All semantic meaning must live in:

* Markdown docs
* Code annotations
* Config files

### Derived Data

Local-only:

* indexes
* graphs
* caches

Must be:

* reproducible
* disposable

---

## 9. Design Principles

* **Local-first**
* **Text-first (git-friendly)**
* **Deterministic**
* **Language-agnostic**
* **Incremental**
* **Explainable (no hidden magic)**

---

## 10. Slice Strategy

### Slice 1 (MVP)

* Frontmatter ID parsing
* `@soul` annotations
* Basic graph linking
* LSP:

    * hover
    * go to definition
* MCP:

    * explain interaction

---

### Slice 2

* Wiki links
* backlinks
* rename support
* unresolved diagnostics

---

### Slice 3

* coding standards integration
* issue tracking + dispositions

---

### Slice 4

* IDE authoring tools
* LLM-assisted linking

---

## 11. Success Metrics

* Reduced LLM correction cycles
* Increased documentation coverage
* Faster onboarding to repo
* Fewer architectural regressions
* Developer trust in tooling

---

## 12. Risks

* Over-complex DSL
* Annotation fatigue
* Performance of indexing large repos
* LLM over-reliance
* Schema drift

---

## 13. Future Vision

* Multi-repo semantic linking
* Shared organizational knowledge graphs
* Automated architecture validation
* Deeper LLM integration (proactive suggestions)

---

## Summary

The Repo Soul System transforms a repository from a collection of files into a **living, connected knowledge system**, enabling:

* better developer understanding
* stronger architectural integrity
* more effective LLM assistance

All powered by **text-based, version-controlled semantic data**.
