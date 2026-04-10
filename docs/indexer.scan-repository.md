---
id: indexer.scan-repository
kind: function
title: scan_repository
---

Walks a repository from `root`, applying exclusion rules from `SoulConfig`, and produces a `SemanticGraph` containing all discovered documents, annotations, and diagnostics.

Files without valid frontmatter are silently skipped. Annotation files with parse errors produce diagnostics rather than aborting the scan.
