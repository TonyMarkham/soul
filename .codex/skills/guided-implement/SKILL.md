---
name: guided-implement
description: Execute a plan in a user-guided, stepwise way without editing files directly. Use when the user wants one plan step at a time, with proposed code shown in chat for manual implementation.
---

# Guided Implement

Use this skill when the user wants to implement from a plan without direct file edits by Codex.

## Core Rules

- Do not write code into files.
- Do not modify the repository.
- Present one plan step at a time.
- Keep the user focused on the current step only.
- Always read the latest version of any existing file before proposing edits to it.

## Default Workflow

1. Read the specified plan file.
2. Audit the current step against the current repository state before proposing anything.
3. Present only the next step to implement.
4. Show the exact code or edit instruction for that step.
5. Stop and wait for the user before moving to the next step.

## How To Present Edits

When proposing edits:

- include the target file path relative to the repo root
- prefer a Find / Replace strategy for existing code
- for additions to an existing file, provide:
  - `Insert After`
  - `Insert This`
  - `Insert Before`
- use exact, unaltered landmarks from the current file
- if a file does not exist yet, present the full proposed file contents

## File Handling

- Always read an existing file immediately before presenting edits for it.
- If the file changed since the last step, base your instructions on the new contents.
- Never rely on stale file context.

## Output Style

- Keep responses short.
- Focus only on the current step.
- Present code before explanation when possible.
- Do not include future steps unless the user asks.

## Suggested Structure

Use this structure unless the user asks for something else:

```text
Step N: <title>

Target file:
<relative/path>

Find:
<exact text>

Replace with:
<exact text>
```

For insertions:

```text
Step N: <title>

Target file:
<relative/path>

Insert After:
<exact text>

Insert This:
<exact text>

Insert Before:
<exact text if needed>
```

For new files:

```text
Step N: <title>

Target file:
<relative/path>

Create with:
<full file contents>
```

## Plan Auditing

Before presenting a step:

- confirm the step still matches the current repo state
- call out any mismatch between the plan and the repo
- adjust the implementation suggestion to fit the live code
- do not silently follow stale plan assumptions
