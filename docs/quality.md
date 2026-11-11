# Checker quality measurement

ISA inputs and the neutral-suite binary share gate pin
`b472bac28d9f69037d2cf89763c8c9603e7e4fb4`. Tests validate the detached
gate-cache HEAD and never read a live sibling. The current pin was materialized
and its release interpreter built with gate tool `a630f38` using `verify`
against the main checkout and pin-update commit `59dacc9`.

## Reproduction environment

```sh
unset DISPLAY WAYLAND_DISPLAY DBUS_SESSION_BUS_ADDRESS BROWSER
export TMPDIR="$PWD/.tmp/quality-checker/tmp"
export CARGO_TARGET_DIR="$PWD/.tmp/quality-checker/target"
export BENNETT_ISA_SPEC="$PWD/.tmp/gate/cache/bennett-isa-spec@b472bac28d9f/bennett-isa-spec"
export BENNETT_ISA_BINARY="$BENNETT_ISA_SPEC/reference-interpreter/target/release/bennett-isa-ref"
mkdir -p "$TMPDIR"
free -g
uptime
systemd-run --user --scope --quiet timeout 240 /usr/bin/time -p cargo test --offline --manifest-path checker/Cargo.toml -j3 --no-fail-fast -- --test-threads=4
cargo fmt --manifest-path checker/Cargo.toml -- --check
```

Builds require at least 6 GB available RAM and one-minute load below 8.

## Default checker tests

| Configuration | Result | Wall time |
| --- | --- | ---: |
| Before pin repair | 270 passed, 4 missing-fixture failures | 37.45 s |
| Updated pin, original binaries | 274 passed, 0 failed, 0 ignored | 79.86 s |
| Consolidated binary, line-table test profile | 276 passed, 0 failed, 0 ignored | 46.30 s |
| Quality-floor checkpoint, warm cache | 286 passed, 0 failed, 0 ignored | 28.23 s |
| Canonical-admission repair | 291 passed, 0 failed, 0 ignored | 26.05 s |
| Whole-object source admission | 330 passed, 0 failed, 0 ignored | 44.39 s |
| Rung-4 checker CLI integration | 331 passed, 0 failed, 0 ignored | 40.29 s |

The consolidation run includes its profile rebuild; the final timing uses a warm cache. Every integration case is included in
`tests/conformance.rs`; no slow tests or exclusions were needed. The two added
cases cover selected borrow-frame admission as explicit unchecked, malformed
frame fields and invocation bindings. Complete borrow-frame semantics remain an
unresolved obligation, with no proof/count/coverage assigned.

The rung-4 integration checkpoint ran `cargo test --offline --manifest-path
checker/Cargo.toml -j4 --no-fail-fast -- --test-threads=4` with the pinned ISA
environment above: 16 unit and 315 integration tests passed. The CLI slice test
derives eight tuples and coverage 1/16 from a two-cell image, checks JSON output,
and rejects producer requests for either proof label. `python3 -B
ladder/rung-4/check_gate.py -v` passed 56 tests, including published CLI arguments,
indexed oracle-claim selection, rejection/unchecked propagation, exact reduced
and unreduced fractions, float rejection, and the N=1024 image denominator.
The real malformed-certificate probe returns exit 1 with a JSON diagnostic.
These are integration checks, not a passing SQLite launch verdict: realised
oracle reconstruction remains unchecked. The published image-preparation
encoding is now pinned and decoded; supported logical action prefixes have
independent executable applicability checks. The gate keeps its required
evidence label and all (a), (b), (c) acceptance rows.

The image-preparation adoption checkpoint adds 13 focused tests, all passing.
The review repair raises that set to 16: nested claim arguments, budgets and
token identities must match the attachment and independently reached call;
memory interfaces without layout receive no proof, even without a preparation
descriptor. Positive logical/model fixtures are checked by the pinned interpreter.
The full checker suite passes 17 unit and 333 integration tests; the unchanged
differential has 3,331 candidates and zero disagreements. Only the preparation
`logical.rs` and `mod.rs` baseline rows change for these additional checks.
They cover full binding/address validation, exact realised-prefix factorisation,
conditional full-tuple restoration, rational native-model leakage, exact image
preservation at positive advertised epsilon, earlier restoration claim scope,
and static launch-copy template validation without runtime attestation. Expected
values include the published 9/25 leakage of the rational native X model and
the exact phase-sensitive identity `(-X)(-X)=I`. No floats enter these checks.

The loop predicate-stack admission repair is **green**: `cargo test --offline
--manifest-path checker/Cargo.toml --no-fail-fast -- --nocapture` passes 17 unit
and 330 integration tests, zero failed or ignored. The base run reproduced
`qpred c` in a zero-trip loop receiving an unrestricted four-tuple proof while
the pinned interpreter rejected it. ISA §4.6 requires a loop body to leave the
predicate stack exactly as at entry and refers to §3.5; admission now enforces
that rule even at zero trips. A focused unit regression failed before the fix.
New interpreter-backed cases reject three imbalance classes without proof and
retain four balanced classes (including entry pop/re-push), each at counts
0–3. The unchanged lexical generator runs 3,331 candidates with zero
disagreements; its seeds, count and pinned oracle are unchanged. Rung-4's
`python3 -B ladder/rung-4/check_gate.py -v` passes all 56 tests. Only the owned
`admission/predicates.rs` and `admission/wires.rs` structural rows were refreshed:
one additional unit test and a two-line clause citation; wire cognitive mass
decreased by one. No new coverage measurement is claimed. Supported
preparation checks do not implement the missing realised-oracle construction,
all source-adjoint/cleanup/memory expansions or every native-model variety.
Those paths retain explicit unchecked/unestablished results. No passing rung-4
SQLite certificate verdict is claimed.

The inherited nested-loop, qif-lifetime, sampled-table, signed-literal,
large-shift, whole-bundle MCX, lexical and recompute-cleanup regressions all pass.
A recompute schedule retains explicit unchecked obligations.

## Coverage

**Lines: 83.96% (16,808 / 20,019); functions: 81.60%; regions: 85.31%.**
Measured at source checkpoint 856d2d8: 330 passed (314 integration + 16 unit),
zero failures or ignored tests. The new source-admission modules are included.

~~~sh
systemd-run --user --scope --quiet timeout 300 cargo llvm-cov --offline --manifest-path checker/Cargo.toml -j3 --json --summary-only --output-path .tmp/quality-checker/whole-source-coverage.json -- --test-threads=4 --nocapture
~~~

This measures Rust checker execution, separately from clause coverage. No
threshold is imposed. `fault.rs` is now 100% lines: path prefixing, JSON fields,
Display and both ResolveError conversions are asserted exactly.

CLI line coverage is 86.05%; the extracted replay phases are 92.76%;
the grammar-based lexical reader is 97.13%.

The five weakest source files have explicit retained work in
bennett-conformance-ac5:

| File | Lines | Reason / next witness |
| --- | ---: | --- |
| eval/borrow_frames.rs | 56.41% | Scheduled-context and independently derived frame-entry mechanics remain under the adoption issue; no invented positive proof. |
| domain.rs | 69.02% | Large host-limit/component-shape branches need bounded witnesses that avoid materializing huge domains. |
| eval.rs | 71.31% | Remaining procedure/memory execution paths need independent bounded witnesses. |
| structural.rs | 74.43% | Adjoint/library and frame-provenance branches need contract-derived positive compositions. |
| object/admission/scalars.rs | 78.89% | Remaining scalar-opcode and width/role cross products need additional positive and negative witnesses; unresolved index and linked-source mechanics stay explicitly unchecked. |

## Review and remaining work

Fresh review and council review of verdict changes remain the orchestrator's
responsibility; green tests are not a CLEAR verdict. Remaining coverage,
structural, default-audit, clause-map and outside-owned litter work is filed
under the issues named below.

## Test oracles and clause mapping

Value mutations cover six same-type wrong binding hashes, all eight output bits
and four shifted input rows of the controlled-action table. Type sweeps still
visit every field; they no longer hard-code a snapshot's field count. The heap
store/load witness additionally checks the independently derived payload x and
zero fault for all sixteen x values, rather than only its verdict/count.
The test-quality checkpoint passed 269 integration cases; the current count is 314.

```sh
python3 clauses/clausegraph.py coverage clauses/ISA-0.9.json --tags checker/tests/clause_tags.json
```

Fifteen mapped tests were confirmed executed in the successful integration run.
This partial checker-only map reports 671 clauses: 6 strong, 23 weak section-only,
642 uncovered. It is separate from proof tags and line coverage. The clause graph
predates the current pinned contract (source hashes differ); the retained tags
refer to unchanged paragraph text. Updating the graph, K89 tags, Python tests and
neutral-case aggregation is retained as bennett-conformance-fv9.

## Semantic function phases

`check_replay_inner` was 700 physical lines after borrow-frame admission.
It now coordinates schema, expected relation, table correspondence, compatible
program binding, element selection and outcome assignment in 291 lines.
Direct unit tests cover schema and relation phases; existing mutation,
control-flow, fixed-image, domain, row-limit and replay integrations cover the
remaining phases. That phase checkpoint passed 269 integration cases and both new unit tests; the current total is 291.
Controlled-action CLI verdict bytes, for both verified and unchecked inputs,
are identical before and after the split.

The remaining oversized functions have one explicit checker owner in
bennett-conformance-8v5; this structural item is partially implemented and the
remaining work is filed, without a zero-giants claim.

| File | Function | Physical lines |
| --- | --- | ---: |
| `cert.rs` | check_certificate / check_region | 507 / 484 |
| `domain.rs` | resolve_domain | 408 |
| `eval/procedures.rs` | run | 342 |
| `eval.rs` | exec_row / exec_line / check_inverse_pair | 656 / 389 / 362 |
| `numeric.rs` | check_frame | 330 |
| `structural.rs` | translation | 328 |

The inventory masks strings/comments and handles array-type semicolons in
function signatures. Line spans include whitespace; they are structural
measurements, not semantic complexity claims.

## Fail-fast checks

Three failures were reproduced before their fixes: lifted calls invented zero
for missing target/fault values, help exited 3, and unknown internal verdict
states silently mapped to unchecked. Reads now propagate their errors; help
exits 0; invalid verdict states panic with context. Canonicalization errors
cannot become an empty hash or equal empty signature/bounds. Sampled replay
counts assert their established state rather than inventing zero.

The fail-fast checkpoint passed 286 tests; the current count is 330. CLI tests cover verified/rejected/
unchecked/usage statuses 0/1/2/3, deterministic JSON, missing options, IO,
non-UTF-8 and missing embedded certificates. Temporary inputs are removed by
Drop. Remaining optional-operand/component defaults need individual invariant
tests and are filed as bennett-conformance-hql.

## Structural baseline and push ratchet

`python3 -B scripts/quality_ratchet.py` checks the recorded per-file baseline
below. One gate check runs its self-tests and the ratchet. Initial missing
baseline failed first; recording it made the check pass. Self-tests also require
failure for each worsened metric and new tracked litter, and check raw strings,
nested comments and array-type signatures.

Scope: checker Rust source (including embedded unit tests) and repository Python
tooling outside test directories/files. C/IR/object fixtures and tests are not
implementation complexity. Function spans count physical lines. The cumulative
over-50/over-100/over-300 counts and maximum length ratchet the length distribution;
the four noncumulative buckets and function count are descriptive.

Cognitive mass is a reproducible branch-nesting estimate, not a semantic proof:
Rust control keywords cost 1 plus enclosing control-block depth, and logical
operators cost 1. Python uses the equivalent AST control nesting plus logical
operators. Nested Python functions are measured separately. Trivial wrappers
(single returned call, or a bare Rust call) are counted alongside mass so splits
cannot hide growth behind wrappers. The gate also rejects increased process-id
matches and newly tracked litter paths; removing inherited litter is allowed.
Known outside-owned litter is retained explicitly under bennett-conformance-pem
and bennett-conformance-bnk.

```sh
python3 -B scripts/quality_ratchet.py --self-test
python3 -B scripts/quality_ratchet.py
git ls-files '*.log' '*.pyc' '*.darc' '*LOCK*'
rg -n 'bennett-conformance-[a-z0-9]{3}\b|quality-checker|job/|shift [0-9]+' checker/src
```

The source grep has no matches. The JSON records every file's distribution,
complexity mass, trivial-wrapper count and process-identifier matches, plus the
18 inherited tracked litter paths. Ratchet values come only from this repo,
with no external grade, complexity ceiling or coverage threshold. Baseline
updates require a reviewed explanation; `--record` is an explicit maintenance
operation and is never invoked by the gate.

<!-- quality-baseline -->
```json
{"fields": ["functions", "length_le_50", "length_51_100", "length_101_300", "length_gt_300", "over_50", "over_100", "max_lines", "cognitive_mass", "trivial_wrappers", "process_identifiers"],
"files": {
"checker-replay/checker_replay/__init__.py": [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
"checker-replay/checker_replay/__main__.py": [3, 3, 0, 0, 0, 0, 0, 27, 14, 0, 0],
"checker-replay/checker_replay/action.py": [45, 44, 1, 0, 0, 1, 0, 86, 687, 3, 0],
"checker-replay/checker_replay/differential.py": [25, 23, 2, 0, 0, 2, 0, 94, 129, 0, 0],
"checker-replay/checker_replay/errors.py": [3, 3, 0, 0, 0, 0, 0, 5, 1, 0, 0],
"checker-replay/checker_replay/hashes.py": [16, 15, 1, 0, 0, 1, 0, 60, 138, 2, 0],
"checker-replay/checker_replay/jcs.py": [3, 3, 0, 0, 0, 0, 0, 24, 104, 0, 0],
"checker-replay/checker_replay/objparse.py": [10, 9, 1, 0, 0, 1, 0, 85, 256, 0, 0],
"checker-replay/checker_replay/oracle.py": [35, 33, 1, 1, 0, 2, 1, 106, 677, 2, 0],
"checker-replay/checker_replay/realised.py": [14, 14, 0, 0, 0, 0, 0, 31, 141, 1, 0],
"checker-replay/checker_replay/replay.py": [48, 45, 2, 1, 0, 3, 1, 104, 530, 2, 0],
"checker-replay/checker_replay/synthetic.py": [18, 17, 0, 1, 0, 1, 1, 109, 79, 2, 0],
"checker/src/accounting.rs": [15, 9, 3, 3, 0, 6, 3, 183, 169, 2, 0],
"checker/src/cert.rs": [31, 24, 4, 1, 2, 7, 3, 484, 507, 7, 0],
"checker/src/coherent.rs": [10, 9, 0, 1, 0, 1, 1, 142, 94, 2, 0],
"checker/src/coverage.rs": [5, 4, 0, 1, 0, 1, 1, 112, 24, 2, 0],
"checker/src/decimal_limits.rs": [9, 7, 2, 0, 0, 2, 0, 95, 275, 1, 0],
"checker/src/domain.rs": [19, 18, 0, 0, 1, 1, 1, 408, 125, 4, 0],
"checker/src/domain/fixed_images.rs": [1, 0, 0, 1, 0, 1, 1, 104, 42, 0, 0],
"checker/src/eval.rs": [75, 60, 7, 5, 3, 15, 8, 656, 1069, 5, 0],
"checker/src/eval/borrow_frames.rs": [1, 0, 1, 0, 0, 1, 0, 94, 28, 0, 0],
"checker/src/eval/procedures.rs": [10, 8, 1, 0, 1, 2, 1, 342, 163, 2, 0],
"checker/src/eval/procedures/bindings.rs": [4, 3, 1, 0, 0, 1, 0, 82, 76, 0, 0],
"checker/src/eval/replay_claim.rs": [8, 3, 3, 2, 0, 5, 2, 132, 89, 0, 0],
"checker/src/expressions.rs": [4, 3, 1, 0, 0, 1, 0, 78, 34, 1, 0],
"checker/src/fault.rs": [6, 6, 0, 0, 0, 0, 0, 13, 2, 3, 0],
"checker/src/frames.rs": [5, 3, 1, 1, 0, 2, 1, 145, 52, 1, 0],
"checker/src/inplace.rs": [1, 0, 1, 0, 0, 1, 0, 61, 35, 0, 0],
"checker/src/json.rs": [25, 23, 2, 0, 0, 2, 0, 71, 104, 7, 0],
"checker/src/lib.rs": [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
"checker/src/lifted_memory.rs": [24, 22, 1, 1, 0, 2, 1, 204, 90, 4, 0],
"checker/src/main.rs": [2, 1, 0, 1, 0, 1, 1, 147, 43, 0, 0],
"checker/src/matrix.rs": [26, 26, 0, 0, 0, 0, 0, 36, 65, 1, 0],
"checker/src/numeric.rs": [20, 8, 5, 6, 1, 12, 7, 330, 537, 1, 0],
"checker/src/preparation/launch.rs": [1, 0, 0, 1, 0, 1, 1, 122, 30, 0, 0],
"checker/src/preparation/mod.rs": [7, 4, 3, 0, 0, 3, 0, 88, 46, 1, 0],
"checker/src/preparation/schema.rs": [13, 11, 2, 0, 0, 2, 0, 57, 59, 4, 0],
"checker/src/preparation/logical.rs": [8, 6, 2, 0, 0, 2, 0, 80, 86, 0, 0],
"checker/src/construction/mod.rs": [9, 8, 1, 0, 0, 1, 0, 53, 13, 2, 0],
"checker/src/construction/source.rs": [2, 0, 1, 1, 0, 2, 1, 225, 84, 0, 0],
"checker/src/construction/state.rs": [5, 4, 1, 0, 0, 1, 0, 67, 49, 0, 0],
"checker/src/preparation/restoration.rs": [1, 0, 1, 0, 0, 1, 0, 57, 11, 0, 0],
"checker/src/object.rs": [41, 30, 7, 4, 0, 11, 4, 172, 482, 6, 0],
"checker/src/object/admission.rs": [6, 5, 1, 0, 0, 1, 0, 86, 23, 1, 0],
"checker/src/object/admission/calls.rs": [15, 9, 5, 1, 0, 6, 1, 101, 103, 2, 0],
"checker/src/object/admission/classical.rs": [4, 4, 0, 0, 0, 0, 0, 47, 31, 0, 0],
"checker/src/object/admission/operations.rs": [11, 6, 5, 0, 0, 5, 0, 78, 73, 3, 0],
"checker/src/object/admission/predicates.rs": [4, 4, 0, 0, 0, 0, 0, 28, 15, 0, 0],
"checker/src/object/admission/procedures.rs": [4, 4, 0, 0, 0, 0, 0, 49, 29, 0, 0],
"checker/src/object/admission/scalars.rs": [20, 16, 3, 1, 0, 4, 1, 213, 132, 1, 0],
"checker/src/object/admission/syntax.rs": [2, 0, 1, 1, 0, 2, 1, 119, 71, 0, 0],
"checker/src/object/admission/views.rs": [4, 4, 0, 0, 0, 0, 0, 48, 38, 0, 0],
"checker/src/object/admission/wires.rs": [20, 17, 3, 0, 0, 3, 0, 100, 100, 1, 0],
"checker/src/object/lexical.rs": [2, 1, 1, 0, 0, 1, 0, 88, 80, 0, 0],
"checker/src/object/signatures.rs": [5, 4, 0, 1, 0, 1, 1, 152, 60, 0, 0],
"checker/src/rat.rs": [37, 36, 1, 0, 0, 1, 0, 69, 65, 6, 0],
"checker/src/realisation.rs": [37, 26, 7, 4, 0, 11, 4, 265, 430, 5, 0],
"checker/src/realisation/native_model.rs": [17, 15, 0, 2, 0, 2, 2, 147, 156, 1, 0],
"checker/src/sampler.rs": [6, 6, 0, 0, 0, 0, 0, 25, 17, 0, 0],
"checker/src/sha256.rs": [3, 2, 1, 0, 0, 1, 0, 67, 12, 1, 0],
"checker/src/structural.rs": [10, 6, 2, 1, 1, 4, 2, 328, 217, 1, 0],
"checker/src/target.rs": [16, 12, 3, 1, 0, 4, 1, 217, 280, 1, 0],
"checker/src/tokens.rs": [1, 1, 0, 0, 0, 0, 0, 44, 12, 0, 0],
"checker/src/transcendental.rs": [8, 8, 0, 0, 0, 0, 0, 45, 25, 0, 0],
"clauses/clausegraph.py": [106, 94, 10, 2, 0, 12, 2, 151, 1529, 6, 0],
"harness/bdiff.py": [1, 0, 1, 0, 0, 1, 0, 62, 22, 0, 0],
"harness/lib/compare.py": [21, 21, 0, 0, 0, 0, 0, 43, 184, 1, 0],
"harness/lib/corpora.py": [9, 9, 0, 0, 0, 0, 0, 37, 75, 0, 0],
"harness/lib/drivers.py": [10, 10, 0, 0, 0, 0, 0, 22, 55, 2, 0],
"harness/lib/engine.py": [26, 26, 0, 0, 0, 0, 0, 33, 139, 2, 0],
"harness/lib/executors.py": [7, 7, 0, 0, 0, 0, 0, 15, 11, 0, 0],
"harness/lib/generate.py": [61, 56, 5, 0, 0, 5, 0, 81, 899, 2, 0],
"harness/lib/identity.py": [19, 18, 1, 0, 0, 1, 0, 72, 180, 1, 0],
"harness/lib/objfile.py": [13, 13, 0, 0, 0, 0, 0, 50, 79, 5, 0],
"harness/lib/prng.py": [7, 7, 0, 0, 0, 0, 0, 15, 7, 1, 0],
"harness/lib/shrink.py": [16, 15, 1, 0, 0, 1, 0, 51, 141, 0, 0],
"harness/tools/extract_vm_differential.py": [3, 3, 0, 0, 0, 0, 0, 30, 62, 0, 0],
"ladder/common/gate_support.py": [4, 4, 0, 0, 0, 0, 0, 21, 17, 2, 0],
"ladder/rung-1/classify_report.py": [4, 4, 0, 0, 0, 0, 0, 27, 68, 0, 0],
"ladder/rung-1/batch_reverse.py": [15, 15, 0, 0, 0, 0, 0, 33, 49, 1, 0],
"ladder/rung-1/full_domain_worker.py": [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
"ladder/rung-1/gate.py": [44, 43, 1, 0, 0, 1, 0, 55, 163, 2, 0],
"ladder/rung-1/input_worker.py": [1, 1, 0, 0, 0, 0, 0, 6, 0, 0, 0],
"ladder/rung-1/native_layout.py": [6, 6, 0, 0, 0, 0, 0, 24, 33, 0, 0],
"ladder/rung-1/progress.py": [1, 1, 0, 0, 0, 0, 0, 4, 0, 0, 0],
"ladder/rung-1/revision_sources.py": [11, 11, 0, 0, 0, 0, 0, 28, 52, 0, 0],
"ladder/rung-2/gate.py": [23, 23, 0, 0, 0, 0, 0, 31, 80, 0, 0],
"ladder/rung-3/contract.py": [2, 2, 0, 0, 0, 0, 0, 22, 23, 0, 0],
"ladder/rung-3/gate.py": [37, 37, 0, 0, 0, 0, 0, 30, 100, 0, 0],
"ladder/rung-3/native_audit.py": [12, 12, 0, 0, 0, 0, 0, 43, 70, 0, 0],
"ladder/rung-4/gate.py": [83, 83, 0, 0, 0, 0, 0, 48, 222, 2, 0],
"ladder/rung-4/revision_sources.py": [6, 6, 0, 0, 0, 0, 0, 28, 37, 0, 0],
"scripts/quality_ratchet.py": [10, 10, 0, 0, 0, 0, 0, 37, 101, 0, 0],
"suite/run.py": [14, 13, 1, 0, 0, 1, 0, 65, 189, 1, 0],
"suite/tools/import_fixtures.py": [31, 29, 2, 0, 0, 2, 0, 63, 175, 4, 0]
},
"litter": [
  "harness/runs/2026-11-01-seed1.log",
  "ladder/rung-1/evidence/20261101T135131.872696Z/coverage-audit.log",
  "ladder/rung-1/evidence/20261101T135131.872696Z/gate.log",
  "ladder/rung-1/evidence/20261101T135131.872696Z/rung1-rung2-checks.log",
  "ladder/rung-1/evidence/20261101T135131.872696Z/rung1-rung3-checks.log",
  "ladder/rung-1/evidence/20261101T135131.872696Z/rung1-tests-green.log",
  "ladder/rung-1/evidence/20261101T135131.872696Z/rung1-tests-red.log",
  "ladder/rung-1/evidence/20261101T141544.771577Z/pre-push.log",
  "ladder/rung-2/evidence/default-gate/gate.log",
  "ladder/rung-2/evidence/driver-smoke/gate.log",
  "ladder/rung-2/evidence/first-run/gate.log",
  "ladder/rung-2/evidence/required-pipeline/gate.log",
  "ladder/rung-3/evidence/20261101T125405.461872Z/gate.log",
  "ladder/rung-3/evidence/20261103T093332.236979Z/gate.log",
  "ladder/rung-3/evidence/20261103T093332.236979Z/recording.log",
  "ladder/rung-4/__pycache__/check_gate.cpython-312.pyc",
  "ladder/rung-4/__pycache__/gate.cpython-312.pyc",
  "ladder/rung-4/evidence/20261101T221646.155856Z/gate.log"
]
}
```
<!-- /quality-baseline -->

## Final component verification

At the earlier d089feb quality-floor checkpoint,
the scoped default command above passed 286 tests in 28.23 seconds. Formatting,
the ratchet and its self-tests passed. All 15 tagged test names occur as successful
executions in that checkpoint output.

With `BENNETT_ISA_BINARY` set to the release binary under
`.tmp/gate/cache/bennett-isa-spec@b472bac28d9f/bennett-isa-spec/reference-interpreter`:

```sh
timeout 120 python3 -B -I -m unittest discover -s clauses/tests
timeout 120 python3 -B -I -m unittest discover -s suite/tests
timeout 120 python3 -B -I suite/run.py --binary "$BENNETT_ISA_BINARY" --cases suite/cases
```

Results: 113 clause-graph tests, 23 suite tests, and 16/16 neutral cases pass.
The full push gate and council review are reserved for the orchestrator. No
critical review or CLEAR result is claimed here.

## Canonical-admission review repair

The two review witnesses were reproduced separately against the saved pre-fix
reader: both obtained overall verified and an identity-table proof with freshly
bound hashes. They now reject with E_CERT_OBJECT, section §16.1, and retain the
canonical verified control.

The generated black-box differential covers 52 numeric-atom sites and 183
punctuation sites of a canonical multi-function object: 1,933 nonidentity
mutations, four interpreter-accepted changes, zero disagreements. Before the
repair, the initial 1,662-case seed produced 755 discrepancies (including
proof grants). The later seed adds zero-valued literals/dimensions, typed block
parameters, both structured arms and wildcard punctuation. JSON metadata and
data HEX retain their separate grammars.

Source checks parse token kinds once and apply the printed punctuation and
integer/float productions in context. Integer lexical checks have no host-width
cap. The existing quality baseline is unchanged: lexical cognitive mass remains
80 and its function-length distribution improves. No ratchet allowance was
raised to accommodate the repair.

The default checker command passed 291 tests in 26.05 seconds; the successful
coverage command above also passed all 291 tests. Focused reproduction:

~~~sh
systemd-run --user --scope --quiet timeout 180 cargo test --offline --manifest-path checker/Cargo.toml -j3 --test conformance lexical_differential:: -- --test-threads=4 --nocapture
~~~

The test uses the gate-pinned release interpreter (and validates any configured
BENNETT_ISA_BINARY against that path). It independently refreshes metadata via
the binary's hash command before check, compares object admission, then
ensures interpreter-rejected inputs cannot receive overall verification or a
claim proof. Correcting older -0 region/index positives follows the pinned
interpreter's §16.1 rejections; canonical positive tests remain, and internal
MCX defense tests inject malformed tokens after parsing.


The new whole-object admission modules have their first structural baseline.
All pre-existing baseline entries and litter allowances remain byte-for-byte
equal as parsed values; no existing ceiling was raised. The object symbol
reader and certificate source loader are separate phases, and the two bounds
consumers share the arbitrary-precision comparison.

The loop-boundary ruling is implemented as named source rejection. Audit
deferrals are tracked by bennett-conformance-a4n (repeated definitions and pin
admission), -020 (complete indirect-label index spaces), and -0xm (imported
closure admission). These forms yield unchecked with no claim proof.
The default command passed 330 tests in 44.39 seconds; coverage passed the same
330 tests. The generated matrix now contains 62 numeric sites, 258 punctuation
sites and 3,331 nonidentity candidates, with 46 accepted alternatives and zero
admission or rejected-source-proof disagreements. It includes indexed ctl,
whole-bundle mcx, numeric/opcode word substitutions, structural mutations,
NAME-only list checks, both qif arms and invalid zero-trip operands.

The loop-boundary negative is paired with a balanced positive over the same
independent t XOR c truth table. Every saved review/council witness rejects with
E_CERT_OBJECT; canonical and indexed-control positives retain verified/proof.
The whole-object audit also covers global forward definitions, CFG view
ownership, dynamic register transfers, slot/control disjointness, procedure
signature and fault-width checks, and classical result/memory operand types.

Fresh export verification uses the committed head and explicit pin environment:

~~~sh
checker_export="$(mktemp -d "$PWD/.tmp/quality-checker/fresh.XXXXXX")"
git archive HEAD | tar -x -C "$checker_export"
cd "$checker_export"
systemd-run --user --scope --quiet timeout 240 /usr/bin/time -p cargo test --offline --manifest-path checker/Cargo.toml -j3 --no-fail-fast -- --test-threads=4
~~~

The export contains no .git or gate cache. BENNETT_ISA_SPEC and
BENNETT_ISA_BINARY remain the explicit detached gate-pin paths shown above.
Exact final-head results are recorded in the job resume evidence; the
orchestrator owns the full isolated push gate and independent review.

Preparation uint decoding enforces the §8.3 number/string boundary for the
§12.3 records. Its schema baseline includes the encoding check and a boundary
regression. Attachment image A/W dimensions use the same strict decoder
before image resolution; the preparation module baseline includes this
boundary check. The general replay-domain decoder is unchanged.

## Rung-3 diagnostic input fixture

The console R fixture now declares the musl triple and DataLayout emitted by
both installed Clang 18 and 23 frontends. The pinned dialect publication
`687284fa28a79c536b3885cabec3adf5de26e031`,
`spec/Bennett-Dialect-Spec-0.3-draft.md` §1.7 includes §19.8 in the
accepted `0.3-r1` profile; §19.8 requires each input's target declarations.
Both frontends emit identical headers, so the existing shared fixture serves
both majors. Its console payload, effect call and zero return are unchanged.

```sh
python3 -B -m unittest discover -s ladder/rung-3/tests -p test_effect_console_fixture.py -v
```

This self-check ran four tests: the unchanged `84f8ecb` fixture failed the
target check for both majors; the updated fixture passes all four. The checks
compare declarations against each real frontend, verify the IR with each LLVM
major, retain the literal console body and validate the fixture manifest hash.
The isolated driver probes using `9809c88176c96a047887136562795f4ab259c41b`
both advance past target admission and exit 1 at the plugin diagnostic
`§19.2 argv/envp byte storage in R launch (milestone 4) at ptr %1`.
This is fixture evidence; no full rung-3 run or acceptance claim follows.

Rung 3 builds its selected frozen reference explicitly, records the executable
hash, and propagates reference-build failures to consumers. Its gate baseline
includes that input-selection branch and the added build stage; native
comparison also receives the pinned runtime native-support publication.

Rung-1 progress publication writes a complete sibling file before atomic rename.
The new four-line publisher is shared by both workers and included in their
recorded implementation hashes. Interruption tests kill each actual worker
during checkpoint writing and require the previous complete record to survive.

Rung 4 supports a diagnostic selection that records all full-SQLite rows as
not run and never reports full acceptance. Its gate baseline includes the
selection branch and explicit stage guard; regression tests assert that the
excluded SQLite methods cannot execute and the default gate stays complete.

Rung-1 full-D execution can use the published streamed action-reversal
interface. The new independent consumer validates binary framing, pinned
identities and invocation/layout, every forward/native and reversed tuple,
all phase/preservation/cleanup flags, padding, footer, process success and
exact shard coverage. Process lifetime and UNKNOWN timeout cleanup have
separate bounded helpers. The gate baseline includes range scheduling and
capability dispatch; no domain, native comparison or reversal assertion is
removed. New-module baseline metrics include its file-hash helper.

Rung-1 constant-layout admission retains the named read-only symbol address
check. Its anonymous fallback additionally proves the device image bytes and
size within read-only merged section data, exact file/address correspondence,
and a unique read-only file-backed LOAD range; a named symbol in that section
cannot use this fallback. The new helper baseline covers those checks. Real
LLVM ELF regressions retain named-global failures and reject byte, size,
address and writable-load mutations.
