---
name: optimize-plan
description: Optimize a plan document through iterative fresh-context review/edit passes until stable. Use when the user wants a file-backed implementation plan audited and refined by fresh subagents instead of long chat iterations.
---

# Optimize Plan

Run a multi-pass plan optimization loop against a plan file.

Primary objective:

- produce a production-grade implementation plan for the stated goal at the top of the plan file
- the resulting plan must drive code that is correct, secure, performant, clean, maintainable, and well-tested

This is a hard requirement, not a preference.
The loop fails if it improves wording while leaving the plan unable to drive that production-grade implementation outcome.
Treat code blocks and file-specific implementation sections inside the plan as staged code, not illustrative prose.

## Arguments

Accept one of:

- `<file>`: review and edit the file in place each pass
- `<input_file> <output_file>`: read from input, write a complete refined plan to output each pass

Default to the in-place mode unless the user explicitly asks for separate input/output files.

## Codex Mapping

Use Codex subagent tools directly:

- use `spawn_agent` for each review pass
- use `wait_agent` to collect the pass result
- use `send_input` only if you need to clarify or continue an existing pass
- use `close_agent` after each completed pass before spawning the next one
- prefer `fork_context: false` so each reviewer starts fresh
- pass only the plan path, mode, pass number, and pass log unless more context is strictly required

Use repo files and shell reads to verify claims:

- use `rg`, `sed`, `find`, and targeted reads against the codebase
- avoid broad context dumps into the main thread

## Review Standard

This skill is not a polish pass.
It is a production-readiness review loop for plan documents.
The plan's stated goal is the governing contract for every pass.

Reviewers must first identify the feature or outcome promised at the top of the plan, then judge every section by one question:

- does this materially help produce a production-grade implementation of that stated goal?

Anything that does not serve that goal is noise and must be removed or rewritten.

Production-grade implementation quality means the plan should lead to code that is:

- correct in behavior for the stated feature
- secure at trust boundaries and input boundaries
- performant enough for the intended use, with assumptions and limits made explicit
- cleanly structured into understandable units with sane responsibility boundaries
- maintainable by future engineers without heroic context recovery
- well-tested beyond the happy path
- explicit about failures, diagnostics, and operational behavior

Communication quality also matters.
The plan must explain what each major implementation section does, why it exists, and how it contributes to the top-of-plan goal.
The plan fails if it degenerates into disconnected code blocks or implementation fragments without enough explanation to guide a competent implementer.

Single-authority rule:

- the plan must have exactly one authoritative implementation direction for each module, file, or interface it describes
- stale drafts, deprecated sections, superseded blocks, alternative implementations left in place, and "use the later section" markers are defects
- if a section is no longer authoritative, delete it; do not preserve it in the plan as history
- duplicate authority is a production-grade failure because it makes the staged implementation ambiguous

Reviewers must aggressively look for:

- missing implementation detail required to build the stated feature correctly
- weak explanation that would leave an implementer unsure what a section is for, why it is shaped that way, or when it should be executed
- code shapes that would likely produce tangled, oversized, or hard-to-maintain implementations
- responsibilities that are packed into god-functions, vague helpers, or poorly-bounded modules
- interfaces that would make testing, error handling, or future change harder than necessary
- missing security constraints around untrusted input, secrets, permissions, identity, or externally visible behavior
- missing performance constraints, scale assumptions, or algorithmic guardrails where they matter
- steps that would likely produce brittle code even if the plan is internally consistent
- interfaces or outputs that hide important semantics behind vague shapes, overloaded fields, or loosely-related values
- places where the plan invents local patterns instead of using the project's explicit abstractions, conventions, or architecture
- interfaces that technically work but make correctness, ownership, intent, or failure propagation ambiguous
- steps that are locally consistent but architecturally wrong
- missing invariants, missing failure-mode handling, and unclear boundaries between different classes of outcomes
- missing production concerns such as observability, rollout safety, regression containment, security boundaries, capacity assumptions, operational recovery, and data lifecycle handling

Reviewers must prefer catching deep design mistakes over preserving the current shape of the plan.
If a plan is superficially coherent but expresses the wrong architecture, that is a real issue and must be fixed.

The standard for "good" is not "plausibly implementable."
The standard is "specific enough that a competent implementer is unlikely to build something fragile, unsafe, opaque, or operationally painful."
The plan should read like executable implementation intent in Markdown form, not like design notes or accumulated commentary.
It should also make the clean code shape apparent: module boundaries, interface boundaries, error boundaries, and test boundaries should be easy to follow from the plan itself.

## Role

You are the orchestrator.

- Do not review the plan yourself.
- Do not directly rewrite the plan based on your own opinions.
- Spawn one fresh-context subagent per pass.
- Use the subagent's structured report to decide whether to continue.
- Do not change the review objective from pass to pass.
- Do not add custom pass intent, custom emphasis areas, or any other orchestrator-authored steering beyond what this skill explicitly defines.
- Treat the plan file and this skill as the only review mandate. Do not inject ADRs, provenance cleanup, or side objectives unless the current plan explicitly depends on them.

## Pass Log

Maintain a pass log in working memory:

```text
Pass 1: ISSUES_FOUND: [...] | CHANGES_MADE: [...]
Pass 2: ISSUES_FOUND: [...] | CHANGES_MADE: [...]
```

## Subagent Instructions

For each pass, spawn a fresh subagent with only the minimum context needed. Use `spawn_agent` with `agent_type: "default"` or `agent_type: "explorer"` and `fork_context: false`.

The orchestrator input to each subagent is tightly constrained. Pass only:

- mode
- plan path(s)
- pass number
- pass log
- locked flip-flop items when the pass procedure explicitly requires them

The orchestrator must not:

- rewrite the review objective
- add custom pass intent
- add new emphasis areas beyond the template below
- steer the reviewer toward a new direction unless resolving a locked flip-flop exactly as described in the pass procedure

Give it this task:

```text
You are performing pass N of a plan optimization loop. Do one thorough review-and-fix pass on the plan document, then report back with a structured summary.

Mode:
- FILE_LOOP: Read <file>, audit it, edit it in place to resolve all issues found.
- PLAN_MODE: Read <input_file>. Review it holistically first. Then write the full revised plan to <output_file>.

Previous pass history:
<full pass log, or "This is pass 1 - no prior history.">

Review instructions:
1. Read the plan thoroughly before forming an opinion.
2. State the top-of-plan goal in one sentence before evaluating anything else.
3. State what production-grade code for that goal requires in this repo: architecture, interfaces, error handling, tests, security, performance, observability, and operational behavior as applicable.
4. Audit every concrete claim against the actual codebase. Verify paths, APIs, file names, dependency ordering, assumptions, and tool availability.
5. Identify genuine issues only. Do not invent problems or style nits.
6. Evaluate whether the plan communicates clearly enough for an implementer to understand what each major section does, why it exists, and how it connects to the goal.
7. Pay special attention to dependency ordering. The implementation order must let a competent engineer execute the plan step by step without hidden prerequisites.
8. Treat every code block and file-specific implementation section as staged implementation, not explanatory prose.
9. Fail any stale draft, deprecated block, duplicate module description, or superseded implementation path you find. Delete or consolidate it according to the mode.
10. Do not re-raise anything already fixed in prior passes.
11. Find and fix as many genuine, material deficiencies as possible in this pass. Do not stop after one issue if more are visible.
12. A pass is only "clean" if it requires no file edits at all.
13. Return only the structured report below.

Production-grade review requirements:
- Do not stop at "the plan is internally consistent." Also ask whether the plan is using the right abstractions.
- Ask whether the plan would lead to code that is clean to implement and maintain, not just code that could be made to work once.
- Ask whether module boundaries, helper boundaries, and ownership splits would keep functions and files understandable instead of turning them into dumping grounds.
- Ask whether the surrounding prose is doing real implementation-guidance work instead of forcing the reader to reverse-engineer intent from code blocks alone.
- Challenge interfaces and contracts. If an interface hides semantics that should be explicit, that is a bug in the plan.
- Challenge outcome handling. Distinguish different classes of outcomes such as success, absence, invalid input, partial success, and operational failure. If those states are conflated, that is a bug in the plan.
- Challenge ownership boundaries. If one component is doing work that belongs in a different layer or responsibility boundary, that is a bug in the plan.
- Challenge data modeling. If the plan uses vague or weakly-typed structures where a domain concept should exist, that is a bug in the plan unless the simplification is explicitly justified.
- Challenge inclusion, exclusion, and selection rules. If they are likely to accidentally include noise or exclude real intended inputs, that is a bug in the plan.
- Challenge external and user-facing contracts. If behavior is underspecified, ambiguous, or impossible to verify cleanly, that is a bug in the plan.
- Challenge test coverage. If tests only prove happy-path behavior or mirror the implementation too closely, that is a bug in the plan.
- Challenge observability. If the plan gives no clear path to understanding behavior in real environments through logs, metrics, traces, audit events, or inspectable outputs, that is a bug in the plan.
- Challenge operability. If the plan does not say how operators or maintainers will detect failure, diagnose issues, recover safely, or understand system state, that is a bug in the plan.
- Challenge rollout and rollback safety. If the plan changes behavior, contracts, storage, or workflows without saying how release risk is contained, that is a bug in the plan.
- Challenge security and trust boundaries. If the plan touches inputs, permissions, secrets, identity, untrusted content, destructive capabilities, or external systems without explicitly constraining risk, that is a bug in the plan.
- Challenge performance and scale assumptions. If the plan assumes current size, traffic, latency, throughput, or single-user behavior without stating limits or safeguards, that is a bug in the plan.
- Challenge state and data lifecycle handling. If the plan creates, mutates, caches, migrates, retries, replays, or deletes state without defining ownership and recovery behavior, that is a bug in the plan.
- Challenge idempotency and retry behavior where applicable. If repeated execution could corrupt state, duplicate work, or produce divergent outcomes without a stated guardrail, that is a bug in the plan.
- Challenge dependency and integration assumptions. If the plan relies on external systems, background jobs, platform capabilities, or ordering guarantees without verifying them, that is a bug in the plan.
- Challenge regression containment. If the blast radius of mistakes is broad and the plan does not include limiting mechanisms, that is a bug in the plan.
- Prefer "what breaks under real use, change, scale, or misuse?" over "can I imagine this working once?"

At minimum, always verify:
- the plan's top-of-file goal is explicit enough to test the rest of the plan against
- the plan would lead to clean code structure rather than oversized functions, muddled ownership, or weak module boundaries
- the plan makes testing practical by defining seams, outcomes, and responsibilities clearly enough to exercise them
- the plan explicitly covers the security properties and performance expectations that matter for the stated feature
- the plan communicates each major implementation section clearly enough that a competent implementer would understand what it does, why it exists, and how to execute it
- every major section directly contributes to implementing that goal rather than documenting side history
- the plan contains exactly one authoritative implementation path for each file, module, interface, and workflow it describes
- no stale drafts, deprecated sections, superseded implementations, or "canonical later section" markers remain in the plan
- every file or directory path named in the plan actually matches the repo layout, or is clearly marked as a to-be-created path
- include/exclude rules are precise enough to avoid pulling in junk while still keeping intended content in scope
- any broad scan step does not accidentally cover generated output, metadata, caches, or editor state unless the plan explicitly intends that
- any filtering rule is defined by purpose, not by a vague shortcut that could exclude real content
- any interface that returns multiple meaningful outcomes makes those outcomes explicit instead of collapsing them into ad hoc shapes
- any operation with multiple failure or non-success modes has a clear and explicit contract for how those modes are represented and handled
- any nullable, optional, omitted, empty, or defaulted field truly means absence rather than "maybe success, maybe invalid, maybe skipped"
- any project-specific outcome or error model described elsewhere in the plan is actually used by the interfaces in the plan
- the plan justifies any deliberate simplification that bypasses the main architecture
- the plan states what must be observable in production and where that visibility comes from
- the plan identifies the likely blast radius of failure and how that blast radius is limited
- the plan names any irreversible, stateful, externally visible, or security-relevant operation and explains the guardrails around it
- the plan includes verification beyond happy-path execution when the feature affects real state, external integrations, or user-visible reliability
- the plan makes explicit any assumptions about scale, ordering, timing, retries, concurrency, or eventual consistency
- the plan defines what safe rollback, retry, replay, or re-run behavior should be where relevant

When reviewing, explicitly ask these questions:
- "What assumptions is this interface making, and are those assumptions visible in the contract?"
- "If this step does not succeed cleanly, where does that outcome go, and is that represented clearly?"
- "What state combinations are possible here, and does the chosen design make invalid states easy to represent?"
- "Is this component owning logic that should belong to another layer?"
- "Would a new implementer follow this plan and accidentally build the wrong architecture?"
- "If this works incorrectly in production, how would anyone know?"
- "What is the failure blast radius, and what in the plan contains it?"
- "What happens on retry, replay, duplication, partial completion, timeout, or dependency failure?"
- "What assumptions about volume, timing, concurrency, or external systems are currently implicit?"
- "If this rollout goes badly, what is the safe retreat path?"

If the plan touches state, users, external integrations, or operations, reviewers should expect the plan to address:
- how correctness will be observed after deployment
- what the rollback or mitigation path is
- how failures will be detected and triaged
- what happens under partial failure or repeated execution
- how regressions will be constrained and verified

If the plan intentionally omits a production concern for scope reasons, it must say so explicitly and bound the risk clearly.

PASS_FAIL_GATES:
- FAIL if the pass does not first anchor itself to the top-of-plan goal.
- FAIL if the pass does not state what production-grade code for that goal requires.
- FAIL if any section does not materially contribute to a production-grade implementation of that goal.
- FAIL if the plan is not clear enough to act as an implementation guide in addition to being technically specific.
- FAIL if the plan still contains stale draft text, deprecated sections, superseded implementations, or duplicate authority after the pass.
- FAIL if the plan could still lead to insecure, unperformant, untestable, overly-complex, or hard-to-maintain code.
- FAIL if the plan could still lead a competent implementer to build the wrong architecture, wrong interface, or wrong operational behavior.
- PASS only if the pass required no edits and the plan is already a single authoritative, clear, staged implementation surface for the stated goal.

TOP_GOAL:
- <one sentence>

PROD_REQUIREMENTS:
- <requirement>
- <requirement>
or "none"

IMPLEMENTATION_GAPS:
- <gap>
- <gap>
or "none"

CODE_QUALITY_RISKS:
- <risk>
- <risk>
or "none"

SECURITY_RISKS:
- <risk>
- <risk>
or "none"

PERFORMANCE_RISKS:
- <risk>
- <risk>
or "none"

TEST_GAPS:
- <gap>
- <gap>
or "none"

ORDERING_GAPS:
- <gap>
- <gap>
or "none"

COMMUNICATION_GAPS:
- <gap>
- <gap>
or "none"

ISSUES_FOUND:
- <issue>
- <issue>
or "none"

CHANGES_MADE:
- <change>
- <change>
or "none"

CHANGE_RATIONALE:
- <what changed, why it changed, and what deficiency it resolves>
- <what changed, why it changed, and what deficiency it resolves>
or "none"

REMAINING_CONCERNS:
- <concern>
or "none"
```

## Pass Procedure

1. Determine mode from the user arguments.
2. Initialize the pass log.
3. Spawn a fresh subagent for the current pass with only the plan path, mode, pass number, and pass log.
4. Wait for the structured report.
5. Close the completed subagent.
6. Append the result to the pass log.
7. Surface the pass results to the user with a detailed summary of what changed and why. Do not compress CHANGES_MADE or CHANGE_RATIONALE into a vague one-line update.
8. Check whether the pass explicitly anchored to the top-of-plan goal, derived production requirements, covered communication quality, covered dependency-aware execution order, and enforced the single-authority rule. If not, treat the pass as insufficient even if it found issues.
9. If the pass appears to flip-flop with an earlier pass, do not stop immediately. Inspect the current file state yourself and identify the oscillating item.
10. Choose one concrete direction as the orchestrator and record that decision in the pass log as a locked item.
11. In the next subagent prompt, explicitly tell the reviewer that the item is locked and must not be reopened unless the current file still has a concrete correctness problem.
12. Use pass intent deliberately. Early passes should check top-of-plan goal alignment, implementation sufficiency, communication clarity, dependency-aware order, clean code structure, and authority cleanup. Later passes should shift toward adversarial production-readiness review, including security, performance, testing depth, operations, safety, reliability, and regression risk.
13. If a pass edits the plan, it is not a clean pass. Continue.
14. If a pass returns `ISSUES_FOUND: none`, do not treat that as sufficient by itself unless `CHANGES_MADE` is also `none`. The next pass must perform an adversarial review aimed specifically at hidden architectural problems, code-structure problems, communication gaps, dependency-order gaps, testability gaps, security gaps, performance assumptions, outcome-model mismatches, stale authority, and production-readiness gaps.
15. Check stop conditions.
16. If not stopping, spawn the next fresh subagent.

## Stop Conditions

Check in this order after each pass:

1. Converged: `ISSUES_FOUND` is `none` and `CHANGES_MADE` is `none` for 3 consecutive passes -> stop
2. Locked flip-flop persists: an item that was explicitly locked by the orchestrator is re-raised again in a later pass without a new concrete file-state justification -> stop
3. Safety limit: 12 passes reached -> stop

Additional convergence guidance:

- Consecutive `none` passes only count if at least one of those passes was explicitly adversarial and still found no issues.
- If earlier passes only checked consistency, the orchestrator should not declare convergence yet.
- If earlier passes did not explicitly verify top-of-plan goal alignment and single authority, the orchestrator should not declare convergence yet.
- If earlier passes did not explicitly verify communication quality and dependency-aware execution order, the orchestrator should not declare convergence yet.
- Before declaring convergence on plans with meaningful production impact, ensure at least one late pass explicitly targeted observability, safety, rollout, failure handling, and regression containment.

Notes on flip-flops:

- A plain flip-flop is not a stopping condition.
- Flip-flops are resolved by the orchestrator choosing a direction and carrying that decision forward.
- Only stop if reviewers keep re-raising the same locked item after the lock has already been made explicit.

## Final Output

When stopping, report only:

- mode used
- number of passes
- stop reason
- if locked flip-flop persists, the oscillating item
- output location, or that the file was edited in place
- per-pass change summaries, including what changed and why
- remaining concerns from the final pass, if any

## Notes

- Prefer repo-local plan files over chat-heavy planning.
- Keep the user's token budget in mind: do the iterative work in files and subagents, then summarize briefly.
- Use fresh-context reviewers to reduce self-justifying edits.
- If the reviewer edits a file, it should edit the file directly in its workspace and report only the structured result back.
- In FILE_LOOP mode, the target file is the source of truth. Do not paste large rewritten plans back into chat.
- Keep only one review-pass subagent open at a time unless parallel review is explicitly part of the workflow.
- The goal is not to make the plan longer. The goal is to make it harder to build the wrong thing and easier to operate the right thing.
- The plan is a safe staging surface for real implementation decisions. Review it with the same rigor you would apply to production code under review.
