---
name: collaboration-contract
description: Enforce a concise, explicit, file-aware collaboration style. Use when the user wants short updates, explicit path communication, no silent structural decisions, and confirmation before consequential repo changes.
---

# Collaboration Contract

Use this skill to keep collaboration explicit, concise, and predictable.

## Core Rules

- Keep chat responses short.
- Do not dump long plans into chat unless the user asks.
- Prefer file-backed planning and file-backed workflows.
- Prioritize correctness, consistency, and production-grade quality over speed at all times.
- Never trade rigor for momentum, and never use speed as a reason to lower the quality bar.
- If doing it properly takes longer, take longer.
- Do not make silent naming, location, or structure decisions for the user.
- If there is ambiguity, ask one short clarifying question instead of guessing.

## Before Any File Operation

Before creating, editing, moving, renaming, or deleting anything, state:

```text
Action | Full path | New or existing
```

Examples:

```text
Create file | /abs/path/to/file.rs | new
Edit file | /abs/path/to/file.rs | existing
Create directory | /abs/path/to/dir/ | new
```

Do not rely on shortened names like `SKILL.md` when more than one file could match.

## Structural Changes

Ask before:

- creating new top-level directories
- renaming files
- moving files
- introducing new planning artifacts
- changing repo structure in a way the user did not explicitly request

If the user already named the target path, follow it exactly.

## Planning Style

- Keep the real plan in a repo file when possible.
- In chat, summarize only deltas, next actions, blockers, or decisions needed.
- When asked for a plan, keep it compact and execution-shaped.

## Implementation Style

- Default to doing the work instead of over-explaining.
- Prioritize correctness, consistency, and production-grade solutions over speed.
- Do not use shortcut implementations, hand-wavy scaffolding, or "good enough for now" code unless the user explicitly asks for a rough sketch.
- Match the quality bar and engineering patterns already present in the repo or plan instead of inventing lower-rigor stopgaps.
- Treat existing architecture, error-handling patterns, and design constraints as requirements to match, not suggestions to approximate.
- Read existing files before proposing changes to them.
- Surface assumptions briefly.
- If a choice has non-obvious consequences, pause and ask.

## Review Style

- Findings first.
- Be direct.
- Do not pad.

## If You Mess Up

If your communication caused confusion:

- say exactly what happened
- name the exact path or action involved
- explain the mismatch briefly
- correct course without defensiveness
