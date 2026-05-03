Optimize a plan document through iterative fresh-context review/edit passes until stable.

## Arguments

`$ARGUMENTS` will be one of:
- `<file>` — **file loop mode**: review and edit `<file>` in place each pass
- `<input_file> <output_file>` — **plan mode**: read from `<input_file>`, write the complete refined plan to `<output_file>` each pass

Parse the arguments now and determine which mode applies before doing anything else.

## Your role

You are the **orchestrator**. You do not review or edit the plan yourself. You spawn fresh-context subagents using the `agent` tool with agent type `plan-reviewer` — one per pass — and evaluate their structured reports to decide whether to continue.

## Pass log

Maintain a pass log in your working memory throughout:

```
Pass 1: ISSUES_FOUND: [...] | CHANGES_MADE: [...]
Pass 2: ISSUES_FOUND: [...] | CHANGES_MADE: [...]
```

## Each pass

Spawn a `plan-reviewer` agent with the following prompt, substituting the pass number, file paths, mode instructions, and the current pass log:

---

You are performing pass N of a plan optimization loop. Do one thorough review-and-fix pass on the plan document, then report back with a structured summary.

**Mode:** FILE_LOOP or PLAN_MODE
- FILE_LOOP: Read `<file>`, audit it, edit it in place to resolve all issues found.
- PLAN_MODE: Read `<input_file>`, audit it holistically, then write the full revised plan to `<output_file>` (do not patch — rewrite the complete document).

**Previous pass history:**
(paste the full pass log here, or "This is pass 1 — no prior history." if first pass)

**Review instructions:**
1. Read the plan file thoroughly before forming any opinion.
2. **Code completeness gate (mandatory):** Scan every step in the plan. Any step that describes implementation work (parsing, constructing structs, adding fallbacks, modifying control flow, etc.) but contains **no compilable code snippet** must be flagged as `ISSUES_FOUND: missing code for step <N>`. Prose-only implementation steps are a failure of the plan — treat them as blocking issues. Do **not** proceed to step 3 until every step either has compilable code or has been flagged.
3. Audit every concrete claim against the actual codebase — use tool calls as needed to verify file paths, function signatures, type names, crate APIs, and any external SDK behaviour referenced in the plan.
4. Identify **genuine** issues only — do not invent problems or reach for make-work. Only flag something if it would actually cause a failure or incorrect behaviour during implementation. Genuine issues include: incorrect APIs, wrong file paths, missing steps, stale assumptions, internal contradictions, anything that would concretely cause problems. **Pay particular attention to dependency ordering** — verify that every step only relies on outputs from earlier steps.
5. Before flagging an issue, check the pass history — do NOT re-raise anything already listed in a previous pass's CHANGES_MADE. If it is already fixed, skip it.
6. Fix every issue you identified per the mode instructions above.
7. Return ONLY the structured report below — no other commentary.

ISSUES_FOUND:
- <issue 1>
- <issue 2>
  (or "none" if the plan is clean)

CHANGES_MADE:
- <change 1>
- <change 2>
  (or "none")

REMAINING_CONCERNS:
- <anything you could not verify or were uncertain about>
(or "none")

---

## Convergence check

After receiving each pass report, check these conditions in order:

1. **Converged**: `ISSUES_FOUND` is "none" for **three consecutive passes** → **STOP**.
2. **Flip-flop**: Any item in this pass's `ISSUES_FOUND` closely matches an item in any previous pass's `CHANGES_MADE` → **STOP**.
3. **Safety limit**: Pass count has reached 12 → **STOP**.
4. Otherwise: append to pass log and spawn the next pass.

## Final report

When stopping, tell the user:
- Mode used (file loop / plan mode)
- Number of passes run
- Stop reason: Converged / Flip-flop detected / Safety limit reached
- If flip-flop: quote the oscillating item
- The output file path, or "edited in place: `<file>`"
- `REMAINING_CONCERNS` from the final pass, if any
