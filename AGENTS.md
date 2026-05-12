# Codex Rules For This Repo

## User Authority and Collaboration

- The user's explicit current-turn directive controls workflow, scope, and approval decisions unless it is impossible or conflicts with higher-priority system/tool safety constraints.
- Collaboration means preserving the user's chosen workflow while surfacing concrete evidence, risks, and tradeoffs.
- Do not silently reinterpret, narrow, broaden, replace, downgrade, or "make safer" the user's requested workflow.
- Do not treat repo rules, model judgment, best practice, caution, or convenience as permission to override the user's directive.
- If instructions appear to conflict, stop before acting, state the concrete conflict, and ask the user how to proceed.
- The assistant does not get the final word on disputed scope or workflow. Unresolved conflicts go back to the user.
- Do not use subagents, reviewer prompts, orchestration prompts, audits, or summaries to smuggle in constraints the user did not request.
- Before spawning a subagent for a named workflow, verify the prompt preserves the user's requested mode and does not add unauthorized restrictions.
- If the assistant's previous action violated the user's workflow, narrow the next action to the requested correction. Do not defend, reframe, or broaden the task.

## Before Editing

- For audits, reviews, and implementation-plan work: do not edit files until the user explicitly approves edits.
- First produce findings only.
- Findings must distinguish:
  - repo pattern violation
  - existing behavior compatibility risk
  - actual plan defect
  - intentional new behavior
- Do not list ordinary feature changes as pattern deviations.

## Named Plan Workflows

- A current-turn invocation of a named workflow that explicitly edits a specified plan file is edit approval for that workflow's stated target file only.
- Examples include:
  - `Use $optimize-plan on Wikilinks.md`
  - `Use $optimize-plan on input.md and write the result to output.md`
- For `$optimize-plan`, reviewer subagents may edit the specified plan file according to the skill's gated review-and-fix rules. The orchestrator must not downgrade the workflow to audit-only unless the user explicitly asks for audit-only.
- The general "findings first / do not edit" rule still applies to ordinary audits, reviews, implementation-plan checks, and unnamed plan work.
- If a named workflow's edit contract appears to conflict with repo instructions, stop and ask instead of silently changing the workflow.
- Do not reinterpret, narrow, broaden, or replace the user's requested workflow with a safer or preferred workflow unless the user approves that change.
- Do not inject stricter constraints into subagent prompts than the user, the named workflow, and these repo instructions require.

## Implementation Plans

- Preserve implementation detail.
- Do not replace concrete implementation instructions with prose.
- Do not add pseudo-code.
- Any code block must be real code intended to compile in this repo.
- Do not invent APIs, structs, modules, error types, or helper functions unless repo evidence supports them.
- Preserve existing behavior unless the plan explicitly changes it.
- If changing a plan, make the smallest patch that fixes the approved defect.

## Soul Repo Patterns

- One primary struct or enum per file.
- Parser APIs use `ParseReport<T>`.
- Recoverable parser problems become `Diagnostic`.
- Fatal operations return `IndexerResult<T>` / `IndexerError`.
- Markdown internals are crate-private unless a public API requires otherwise.
- `SemanticGraph` is the scan/index/explain data carrier.
- Stored model paths use repo-relative display paths.
- Existing document, annotation, diagnostic, index, explain, MCP, CLI, and LSP behavior must continue unless explicitly changed.

## Stop Conditions

Stop and ask before editing if:

- the change would remove implementation detail
- the change would replace implementation detail with prose
- the change needs a new abstraction not evidenced by the repo
- the change affects existing behavior not named in the plan
- user feedback challenges the current premise
