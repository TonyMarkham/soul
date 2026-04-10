---
name: implementation-planning
description: Produce implementation-grade plan files with exact target files, concrete code blocks, commands, ordering, and done criteria. Use when the user asks for a plan and wants the plan file itself to contain the real implementation detail, not a high-level roadmap.
---

# Implementation Planning

Use this skill when the user wants a real implementation plan, not an abstract outline.

## Core Rule

The plan file must contain the implementation detail.

Do not put the real code in chat while leaving the plan file vague.

If the user asks for an implementation plan, the plan file itself should be executable as a working blueprint.

## Default Output

Write the plan into a repo file.

Unless the user names a different target, ask where the plan should live before creating it.

## What A Good Plan Must Contain

An implementation-grade plan should include:

- the goal
- the concrete command or user-visible outcome that proves the slice works
- the exact files to add or edit
- full code blocks for new files
- concrete replacement content or inserted content for existing files
- dependency and implementation order
- commands to run
- expected output when relevant
- done criteria

## What To Avoid

Do not produce:

- abstract roadmaps
- vague step names with no code shape
- placeholder phrases like "wire up", "hook up", "scaffold", or "implement parser" without showing what that means in code
- plans where the useful implementation detail lives only in chat

## Planning Procedure

1. Read the current repo state first.
2. Read any existing plan file if one already exists.
3. Decide the smallest useful vertical slice.
4. Verify the file paths and crate/module layout against the actual repo.
5. Write the plan file with implementation-grade detail.

## Required Structure

Unless the user asks for a different format, use this structure:

```text
# Implementation Plan

Short statement of what will work when done.

## File Plan
- <path>
- <path>

## <file path>
```<language>
<exact code or exact replacement content>
```

## Commands
```bash
<command>
```

## Expected Output
```text
<output>
```

## Implementation Order
1. ...
2. ...

## Done Criteria
- ...
```

## Existing Files

For existing files:

- read the live file first
- base the plan on the actual current contents
- if replacing content, show the replacement content directly in the plan
- if inserting content, show the inserted content directly in the plan and name the insertion point

## New Files

For new files:

- include the full proposed file contents
- do not summarize the file in prose when the code itself belongs in the plan

## Chat Behavior

- keep chat short
- summarize what file you created or updated
- do not dump the whole plan into chat unless the user asks

## Quality Bar

If a developer could not implement directly from the plan file without asking "what exactly do you mean here?", the plan is not detailed enough.
