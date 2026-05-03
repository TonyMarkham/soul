---
id: indexer.markdown-annotations
kind: function
title: Markdown HTML-comment annotations
---

## Overview

Extracts Soul annotations from `.md` files that lack YAML frontmatter, using `<!-- soul ... -->` HTML comments. This lets READMEs, ADRs, design docs, and other arbitrary Markdown files participate in the semantic graph alongside Soul documents and code annotations.

See `docs/doc-references.md` for the full technical specification and implementation details.

## Five W's

**Who** — A developer writing or maintaining documentation. Also the indexer, which processes these annotations during scans.

**What** — Scans every `.md` file for HTML-comment lines containing `soul id="..."` and registers them as code annotations in the semantic graph. Supports arbitrary metadata key-value pairs.

**When** — During every `soul_index` scan. The annotation extraction runs in the same pass as frontmatter parsing, on every `.md` file encountered.

**Where** — Covered by Soul annotations (see linked code locations).

**Why** — Previously, only files with YAML frontmatter could carry Soul IDs. Non-document Markdown files (README, ADR, playbook) had no way to declare their semantic identity. HTML-comment annotations give those files a lightweight, inline syntax without requiring frontmatter or a dedicated plugin for every annotation format.

### Needs elaboration

- (none)
