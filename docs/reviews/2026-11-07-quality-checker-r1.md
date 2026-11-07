# Quality-checker review — round 1

**BLOCK — P1-1 (oracle), bennett-conformance-mge.** Reviewed the entire
`main..job/quality-checker` batch (14 commits), base
`e39501e30bcbc0e67046fce24d3354f775a8f78f`, head
`5aa394f4ea6268cdc18f87a5831dc7f91488c289`, using
`git log --reverse main..job/quality-checker` and
`git diff main...job/quality-checker`. No implementation edits.

**P1-1 — incomplete lexical validation still grants proofs.**
`checker/src/object/lexical.rs:94` checks numeric spelling only when the atom
starts with a digit/sign, so the integer width in `i04` escapes validation.
At lines 66–78, single spaces around bounds punctuation also escape validation.
For the source below, with the complete four-row identity table for `(t,c)`
(`00,01,10,11`), full one-bit domains, explicit empty arguments/four zero budget
components, and rebound hashes, the checker returns **overall=verified,
claim=proof, no errors**:

```text
perm_action @a (t:wire mut) (c:wire in) () () ()
regs { unused:i04 } bounds {2}
^entry():
qloop 2 {
cx c t
}
```

The pinned ISA interpreter's `check` rejects the same bytes with
`§16.1: integer width must be a canonical unsigned INT`. Replacing the regs
line with `regs {  } bounds { 2 }` also obtains verified/proof while the
interpreter rejects its loop-bound spelling. Canonical
`regs {  } bounds {2}` passes both. This continues the eq2 oracle-admission bug:
an ill-formed source object receives a certificate proof. Add determining
rejections while retaining canonical positives. Filed **bennett-conformance-mge**
(priority 1, label review), including the signature and complete repro recipe.

The probe was compiled from stdin against the reviewed checker rlib, reused
only the repository's certificate-binding helper, and independently specified
the identity table (two CX applications). It invokes the published interpreter
as a black-box oracle. Exact rerun, from this worktree with the environment below:

```sh
REFBIN="$BENNETT_ISA_SPEC/reference-interpreter/target/release/bennett-isa-ref" timeout 10s .tmp/rev-quality-checker-r1/tmp/review-probe
```

**P2+ nits — bennett-conformance-5na (priority 3):** final-source test counts in
`docs/quality.md:32,47` predate two added tests (head lists 274 integration + 14
unit tests); repeated string dispatch in `eval/replay_claim.rs:6,27,70`; retained
oversized semantic functions (existing 8v5), partial/stale clause mapping (fv9)
and 18 inherited outside-owned litter paths (pem/bnk). No additional demonstrated
oracle/rung/gate failure. All COMMON anti-slop categories and plan items 9–11
were checked; the ratchet is wired and passes, HANDOFF is deleted and README
points to the workspace handoff.

Validation used this headless environment, ISA pin
`3b1090790d82a00ac95d30b8da6b42d1d7a9f5d1`, and focused tests only:

```sh
unset DISPLAY WAYLAND_DISPLAY DBUS_SESSION_BUS_ADDRESS BROWSER
export TMPDIR="$PWD/.tmp/rev-quality-checker-r1/tmp"
export CARGO_TARGET_DIR="$PWD/.tmp/rev-quality-checker-r1/target"
export BENNETT_ISA_SPEC="$PWD/.tmp/gate/cache/bennett-isa-spec@3b1090790d82/bennett-isa-spec"
mkdir -p "$TMPDIR"
timeout 180s cargo test --offline --manifest-path checker/Cargo.toml -j3 --test conformance lexical:: -- --test-threads=4
timeout 150s cargo test --offline --manifest-path checker/Cargo.toml -j3 --test conformance -- borrow_frames:: control_flow:: resource_contexts:: mutation_sweep:: cli:: pin_environment:: replay_differential:: fault_records:: row_limits:: fixed_images:: --test-threads=4
timeout 150s cargo test --offline --manifest-path checker/Cargo.toml -j3 --test conformance -- hashes:: domains:: core_witnesses:: published_certificates:: token_aliases:: cert_schema:: metadata:: mcx:: numerical::malformed_rotation --test-threads=4
timeout 120s cargo test --offline --manifest-path checker/Cargo.toml -j3 --lib -- --test-threads=4
```

Results: **2 + 88 + 60 + 14 = 164 passed**, no failures/ignored cases.
`cargo fmt --manifest-path checker/Cargo.toml -- --check` and
`git diff --check main...job/quality-checker` passed.
`timeout 60s python3 -B scripts/quality_ratchet.py --self-test` and the same
command without `--self-test` passed (70 files, 18 inherited litter paths).
The focused clause coverage query reproduced 6 strong/23 weak/642 uncovered.
No full gate, full-suite timing or fresh coverage measurement was run.
Reading `git show main:<path>` confirms the new lexical/borrow-frame/recompute,
missing-read and CLI-state regressions would fail against main's corresponding
paths; the existing control-flow regressions were exercised, not credited as
new red-first repairs.

Coverage: closely read the full owned-path diff, new lexical/borrow/replay
modules, replay/verdict aggregation and relevant parsing/realisation paths,
pin resolution and changed tests; skimmed remaining unchanged semantics and
large fixtures. Compared with pinned ISA §§12/16.1–16.2 and the binding laws,
policy, plan and audit. No verdict-changing replay-phase regression found.

Provenance: implementer **gpt-6.1-sol**; reviewer **gpt-6.1-sol, fresh session**, D9.
