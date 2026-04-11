## `optimize-plan` call
```text
/online-entity-cc-plugin:optimize-plan VS-40.md
**NOTE** Assume that cargo check/clippy/build/test are all clean
**NOTE** Assume that dotnet build/test/publish are all clean

**CRITICAL** The plan NEEDS to contain a FULL IMPLEMENTATION PLAN with ZERO gaps
```

---

## Split
```text
- Read `VS-180.md`
- Audit it against the current repository code
- Split into 3 plans:
  VS-171.md: database migrations + Rust
  VS-172.md: C#
  VS-173.md: Infra

- **CRITICAL** Be diligent not to miss anything
```

---

## Audit
```text
- Read `VS-40.md`
- Audit it against the current repository code
```

---

## Init
```text
- I don't want you to write any code in any files.
- I want you to present me with all code and I will implement it myself, refactoring to my style.
- Present aspects of the plan 1 step at a time, the intent being to be able to maintain cognitive focus on the step
- Always read any existing file before presenting any edits to it to be sure you have the latest context
- When presenting code to me, include the path of the target file relative to the repo root
- Prefer a `Find` and `Replace` strategy when instructing an edit to an existing file
- For adding new elements to an existing file, provide un-altered `Insert After`, `Insert This` and `Insert Before` landmarks to make it obvious where you intend the addition to be inserted
```

---

## Doc
```text
- Review all MD's in `.docs/Hetzner/` (go no deeper)
- Review `.docs/__upload_to_hetzner__.md`
- Update them all with the new info without changing the style of the doc.
- Identify what aspects of the plan have no thematically obvious doc to be injected into.
```

---

## Commit
```text
I have staged everything I care about, commit without a byline
```