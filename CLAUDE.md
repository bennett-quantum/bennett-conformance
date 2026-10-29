# bennett-conformance

## What this repo is

the demo ladder as executable gates (rungs 1-8), the certificate checker, the legality corpus runner, and the differential harness against native execution with instrumented classifiers (UBSan/ASan/MSan). A consumer of everything; nothing depends on it.

Part of the Bennett reboot: many small repositories, two published contracts
(`bennett-dialect-spec`, `bennett-isa-spec`). Depends on: everything, as a consumer only.

## Where the spec is

The published specs, at the versions in `SPEC_VERSIONS.toml`. Source documents
currently live in the Bennett.jl repo (`Bennett-NorthStar.md`,
`Bennett-ISA-Spec.md`) until the spec repos are split out. The spec wins over
any code, including code in this repo.

## Non-negotiables

0. WORKLOG, KEPT BY WHOEVER COMMITS THE SESSION. Each working session that changes the repo
   leaves one "## Session log - YYYY-MM-DD - ..." block at the top of the highest-numbered
   chunk under worklog/ (ls worklog | sort -r | head -1; past ~280 lines start the next NNN_
   chunk): what changed, gotchas, surprises, failures and root causes not derivable from the
   diff. When an orchestrator runs subagents, the orchestrator writes it; subagents keep their
   job's progress file and put surprises in bd (bd note / bd create) and never need to write
   outside their owned paths. A session that changes nothing writes nothing.
1. FAIL FAST, FAIL LOUD. Assertions and clear errors, never silent returns or
   quiet fallbacks. Unsupported means a crash with context.
2. ONE IMPLEMENTER PER CHANGE. Independent review is batched over several landed
   changes, not required per change.
3. RED-GREEN TDD. Write the test, watch it fail, make it pass, then refactor.
4. EXHAUSTIVE VERIFICATION against the spec conformance suites for the versions
   in `SPEC_VERSIONS.toml`. "Runs without errors" is not a pass.
5. COUPLED TO OTHER REPOS ONLY THROUGH THE PUBLISHED SPECS. No shared code across
   a contract boundary. This repo must be replaceable by a competitor's
   implementation of the same spec without the rest noticing.
6. NO GITHUB CI. Local gates only: `scripts/pre-push` (install with
   `scripts/install-hooks.sh`). No workflows, no remote automation.
7. TRACK TASKS WITH BEADS (bd): bd ready / bd show / bd create; bd init when work starts in a
   repo that has no .beads/ yet. No TODO.md files. Never gitignore .beads/. No Dolt remote sync.
8. SKEPTICISM. Verify subagent output, earlier work, and your own assumptions.
   Reproduce before believing.
9. GIT POLICY (maintainer, 2026-10-29). Any agent commits its own paths regularly
   (git commit -- <paths>; retry on index.lock); the orchestrator pushes after verified
   milestones and at least every couple of hours. This opts the repo into the Beads
   "team-maintainer" profile for commits only, not for pushes or Dolt sync.
10. NO NARROWING OF NORTH-STAR SCOPE (maintainer, 2026-10-29). Nothing may reject, refuse,
   fault, declare inert or omit a capability that bennett/docs/Bennett-NorthStar.md promises
   (thesis, sections 3-5, every rung of the section 8 demo ladder; Grover over unmodified
   SQLite is the core). A deferral may postpone mechanics only. If a contract truly lacks
   something, raise it as a blocker naming the exact change; never shrink scope to fit.
   See bennett/docs/plan/2026-10-29-path-to-north-star.md.
11. HEADLESS, NO /tmp ARTEFACTS. Never run LLVM -view-* / view-cfg / view-dom / view-callgraph
   passes or viewCFG-style helpers outside an interactive human session (use the dot-* variants).
   Batch and agent commands run with DISPLAY, WAYLAND_DISPLAY, DBUS_SESSION_BUS_ADDRESS and BROWSER
   unset and TMPDIR set to a project-local or job directory (2026-10-28: ~80,000 .dot files reached
   the maintainer's editor via xdg-open).