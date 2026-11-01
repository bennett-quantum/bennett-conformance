# Rung 2 COMPLETE — 2026-11-01T13:06Z, gate exit 0 on both LLVM majors and both executors

Pinned revisions: plugin `157d4bd`, VM `13f5a9e` (O(1) per-step cost, bennett-vm-fw8; pointer compares admitted,
bennett-vm-ol3), reference interpreter `ba7d13f` release (O(1) per-step checkpoint, b-isa-spec-n8j), runtimes `6808622`
dist, arith `249e619` dist (passed via `--arith18/--arith23` because the live arith dist was mid-edit).
Evidence: `.tmp/rung-2/20261101T130556.518962Z/` (report.json, table.md, per-major dirs with every command and output).

Every row PASS on LLVM 18 and 23: fixtures, plugin build, VM build, native reference (ten cases, O0+ASan/UBSan and O2),
C front end, runtime bitcode link, post-link LTO, S1–S9 production, launch artefacts, reference admission, VM admission,
reference forward = native, VM forward = native, reference byte-exact reverse, VM byte-exact reverse.

North Star §8 rung 2 acceptance ("a C hashtable (the existing 258-line corpus): forward result equal to native on every
tested input (independent check), then reversed byte-exactly, both executors") is met on this evidence. The VM runs
n=0 in 1.1 s and n=1000 in 1.8 s (release); the reference 2.0 s and 2.5 s.

# Rung 2 rerun 2026-11-01T11:47Z — reference executor passes end to end; VM forward times out

Pinned revisions: plugin `157d4bd` (switch lowering bp-bux reinstated), VM `02236d9` (pointer compares admitted,
bennett-vm-ol3), reference interpreter `ba7d13f` release build (O(1) per-step checkpoint, b-isa-spec-n8j), runtimes
`6808622` dist, arith SHA-pinned. Evidence: `.tmp/rung-2/20261101T114719.085382Z/` (report.json, table.md, per-major dirs).

| Stage | LLVM 18 | LLVM 23 |
|---|---|---|
| fixtures, plugin build, VM build, native reference, C front end | PASS | PASS |
| runtime bitcode link, post-link LTO | PASS | PASS |
| S1–S9 production, launch artefacts | PASS | PASS |
| reference admission, VM admission | PASS | PASS |
| reference forward = native (ten cases) | PASS | PASS |
| reference byte-exact reverse | PASS | PASS |
| VM forward = native | FAIL · bennett-vm-fw8 (300 s timeout, basic-0) | FAIL · bennett-vm-fw8 |
| VM byte-exact reverse | BLOCKED · bennett-vm-fw8 | BLOCKED · bennett-vm-fw8 |

**What this establishes.** The unmodified hashtable corpus, compiled by the plugin against the published runtimes and
arith bitcode, is admitted by both executors and runs forward on the accepted reference interpreter with results equal
to native on all ten cases, and reverses byte-exactly, on both LLVM majors. Rung 2 holds on the reference executor.
The blockers closed to get here, in order: brt-3sa (runtimes bitcode), bp-au6 (§5.2 analysis time), bp-1us (§16.4
aggregates), bp-tav (§17.4 malloc), bp-9fo (§14.1 calls), bp-bux (§14.8 switch), bennett-vm-ol3 (icmp on ptr),
b-isa-spec-n8j (reference per-step clone). Remaining: bennett-vm-fw8 (the VM's time per step grows with steps taken:
debug build 5k steps 3.3 s, 10k 11.3 s, 20k 55.9 s). bp-eof stays open as the plugin-side tracking record until the
VM rows pass.

# Rung 2 measured pipeline and corpus probes — 2026-10-31

**Gate exit: 1. Rung 2 is not complete.** The first required pipeline prerequisite is missing runtimes bitcode (brt-3sa). Both earlier corpus-only production probes exceeded the 300-second command limit (bp-au6); those are diagnostic evidence, not a full §19.1 link. The native baseline, fixture route and both builds work; the hashtable has not yet executed on either executor.

## Corrected required-pipeline gate

The initial probe omitted the §19.1 runtime link. The driver audit caught that glue omission; the final gate requires the link and has **no fallback** to an unlinked module. The runtimes checkout is a skeleton: only `src/allocator/bump.c` exists as implementation, defining `bennett_alloc`/`bennett_free`; there is no published `.bc` distribution or `malloc`/`calloc`/`free` shim closure. No shim was implemented in this slice.

**brt-3sa**, owning repo `bennett-runtimes`, estimated size **medium** for the allocator/libc slice. Determining dialect §19.1 sentence (formatting normalized):

> The driver links all bitcode inputs (§19.8), links `bennett-runtimes` and `bennett-arith` bitcode of the link's major M, runs the post-link pipeline (`lto<O3>` by default, or any envelope pipeline of §21.1 the user selects), then runs the plugin on the whole program (NS §3.3).

The final gate's exact failures are:

```text
missing runtimes bitcode for LLVM 18: /home/tobias/Projects/bennett-workspace/bennett-runtimes/dist/llvm18/bennett-runtimes.bc (dialect §19.1)
missing runtimes bitcode for LLVM 23: /home/tobias/Projects/bennett-workspace/bennett-runtimes/dist/llvm23/bennett-runtimes.bc (dialect §19.1)
```

The path is a configurable consumer default, not a spec-mandated filename. **Fixed means:** publish major-matched bitcode with the allocation/libc closure used by the exact corpus (clang also forms `calloc` and `llvm.memset`), preserving the specified allocator/zero-fill/liveness semantics and corresponding native comparison support. Link it with pinned arith before the plugin. Do not replace heap operations by a scalar summary or host-libc fallback. Then retest bp-au6 against that correctly linked module.

The corrected gate's first run used `--probe-unlinked --timeout 5` to validate the known diagnostic mapping without repeating the 300-second experiment. Required link failure is immediate and unaffected by that timeout. [Manifest](evidence/required-pipeline/report.json), [exact table](evidence/required-pipeline/table.md), and [log](evidence/required-pipeline/gate.log):

| Stage | LLVM 18 | LLVM 23 |
|---|---|---|
| fixtures | PASS | PASS |
| plugin build | PASS | PASS |
| VM build | PASS | PASS |
| native reference | PASS | PASS |
| C front end | PASS | PASS |
| runtime bitcode link | FAIL · brt-3sa | FAIL · brt-3sa |
| post-link LTO | BLOCKED · brt-3sa | BLOCKED · brt-3sa |
| S1–S9 production | BLOCKED · brt-3sa | BLOCKED · brt-3sa |
| launch artefacts | BLOCKED · brt-3sa | BLOCKED · brt-3sa |
| reference admission | BLOCKED · brt-3sa | BLOCKED · brt-3sa |
| VM admission | BLOCKED · brt-3sa | BLOCKED · brt-3sa |
| reference forward = native | BLOCKED · brt-3sa | BLOCKED · brt-3sa |
| VM forward = native | BLOCKED · brt-3sa | BLOCKED · brt-3sa |
| reference byte-exact reverse | BLOCKED · brt-3sa | BLOCKED · brt-3sa |
| VM byte-exact reverse | BLOCKED · brt-3sa | BLOCKED · brt-3sa |
| unlinked diagnostic probe | FAIL · bp-au6 | FAIL · bp-au6 |


All required downstream stages inherit **brt-3sa**, not an executor failure. The optional probe row reports **bp-au6** separately; a probe can never satisfy required production, even if it later passes. No linked-pipeline S2 verdict is claimed. The default gate was also run with its 300-second default and no diagnostic probe; it exits 1 at the identical missing-runtime errors ([report](evidence/default-gate/report.json), [table](evidence/default-gate/table.md)). The first raw 300-second corpus-probe table follows for provenance.

## Initial corpus-only probe table

The first corpus-only production probe started at `2026-10-31T21:49:26Z`, directory `20261031T214926.056811Z`. [Raw command manifest and stage errors](evidence/first-run/raw-report.json), [original table](evidence/first-run/raw-table.md), [console log](evidence/first-run/gate.log), exact optimized inputs ([18](evidence/first-run/18/postlink.ll), [23](evidence/first-run/23/postlink.ll)) and native results are committed here. [Evidence hashes](evidence/first-run/sha256.json) pin that evidence. The raw table says `UNFILED` because the bead was filed after observing red; this table annotates the same outcomes with its resulting ID.

| Stage | LLVM 18 | LLVM 23 | Blocker bead |
|---|---|---|---|
| Fixture integrity / marker-only route | PASS | PASS | — |
| Plugin build | PASS | PASS | — |
| Detached VM build | PASS | PASS | — |
| Native reference, ten cases | PASS | PASS | — |
| C front end | PASS | PASS | — |
| Post-link `lto<O3>` | PASS | PASS | — |
| Production S1–S9 | FAIL (300s timeout) | FAIL (300s timeout) | bp-au6 |
| Device / host / launch artefacts | BLOCKED | BLOCKED | bp-au6 |
| Reference admission (`check`, `hash`) | BLOCKED | BLOCKED | bp-au6 |
| VM admission (`check`, `hash`) | BLOCKED | BLOCKED | bp-au6 |
| Reference forward = native | BLOCKED | BLOCKED | bp-au6 |
| VM forward = native | BLOCKED | BLOCKED | bp-au6 |
| Reference byte-exact reverse | BLOCKED | BLOCKED | bp-au6 |
| VM byte-exact reverse | BLOCKED | BLOCKED | bp-au6 |


## Inputs and frozen implementations

- Corpus: 258 original lines, SHA-256 `8cbd5a3698aea0da1f7187b97be34a0ed33743394f045c49c943323f94abd4b9`. The marked source and exact additions are documented in [README.md](README.md) and [marker-route.diff](fixtures/marker-route.diff).
- Domain: `basic` and `grow`, each at `n = 0, 1, 7, 64, 1000`, exactly the original `main_golden.c` cases. No smaller surrogate and no exclusions.
- Plugin: `fafc40508598c34173aa2c295213536d13078a16`, built from a frozen Git archive outside its working tree. Dialect **0.2.10 @ 023f5a771a814270634db3be9cfde88ded69f975**, ISA **0.8**. The build's `.so` hashes are in the manifest.
- LLVM: clang/opt **18.1.3**, and **23.1.5** development package; exact tool-version strings are in the evidence. Both plugin shared libraries build with the repo's CMake superbuild.
- VM: `eb7ca3ab3bf21eef9d418b037ecaeed3a471cf82`, built offline from the detached worktree at `bennett-conformance/.tmp/vm-eb7ca3ab3bf21eef9d418b037ecaeed3a471cf82`. No VM source changed.
- Reference executable: `/home/tobias/Projects/bennett-workspace/bennett-isa-spec-review/reference-interpreter/target/debug/bennett-isa-ref`. Admission/execution of the hashtable was not reached; no executor agreement is inferred from build success.
- Device budget: `(steps,heap,stack,depth,sstack) = (10000000,1048576,65536,256,65536)`. Host timeout: 300 wall-clock seconds per corpus command, independent of the ISA/dialect budget.
- All runs unset `DISPLAY`, `WAYLAND_DISPLAY`, `DBUS_SESSION_BUS_ADDRESS`, and `BROWSER`, and use `TMPDIR=/home/tobias/Projects/bennett-workspace/bennett-conformance/.tmp`.

## Corpus-only production probe failure / S2 localization

**Bead `bp-au6`, `bennett-plugin`, priority 1, estimated size medium.**

For LLVM 18 the exact driver error was:

```text
18/production: exit 124

timeout
```

For LLVM 23:

```text
23/production: exit 124

timeout
```

The `timeout` line and exit 124 are the driver's observation after `subprocess.run(..., timeout=300)` expired and terminated the child. **The compiler emitted no diagnostic at all**: stdout was empty and stderr was empty until the driver appended that line. No `out/` or production artefact was emitted. This is an observed resource bound failure, not proof of nontermination, a particular wrong points-to result, or a spec-defined rejection. A separate [LLVM 18 analysis-only preflight](evidence/preflight/README.md) also expired at 300 seconds before emitting its S1–S5 report, localizing the earliest measured frontier to the reference-analysis path. The superseded LLVM 23 analysis-only preflight was cancelled; only its full-production timeout supplies a verdict.

The failing input is the **entire** optimized marked corpus in each major's `postlink.ll`. It includes the pointer-rich `ht` structure, dynamic probe/growth loops, heap allocation/free and checksum arithmetic. For example the LLVM 18 input begins its basic entry with:

```llvm
define dso_local i64 @ht_demo_basic(i64 noundef %0) local_unnamed_addr #0 {
  %2 = tail call ptr @__bennett_qalloc(i64 noundef 0, i64 noundef 1) #7
  tail call void @__bennett_qfree(ptr noundef %2) #7
  %3 = tail call noalias dereferenceable_or_null(32) ptr @malloc(i64 noundef 32) #8
```

The determining accepted dialect §5.2 sentence is:

> The analysis is the least fixpoint of the following rules over AV.

§5.1 also fixes saturation:

> When A holds more than **K = 4096** `INT` atoms they are dropped and t is set

The remainder of that sentence says K is normative and decides dispatch/blocker verdicts. §4.1 orders S2 (the shared device-analysis fixpoint) before the later pipeline steps and says “the first failing rule gives the verdict.” §3.8's direct-C-call route and §4.2's marker-containing CC closure make these driver entries device code. No spec permits replacing the corpus by an arithmetic summary, dropping the grow case, or changing reference-analysis distinctions to make this gate green.

**Fixed means:** complete the reference analysis for this exact corpus on LLVM 18 and 23, preserving PT1–PT12, K and explicit handle provenance. Pass S2, then either emit complete production artefacts or report the next determining rule. Rerun the gate and file the next *measured* gap. Raising a timeout alone supplies no semantic completion evidence. This slice does not fix the plugin.

## Initial probe dependent stages

For the corpus-only probe, each row below inherits **bp-au6**; none supplies an independent plugin/VM/spec defect or an executor pass.

| Blocked stage(s) | Exact reason | Determining contract sentence |
|---|---|---|
| Launch artefacts | `Not run; requires S1–S9 production` | Dialect §19.9 outputs include “the materialised device variants as bitcode”, “the host module with device entries replaced by launch stubs” and “the ISA objects”. |
| Reference admission; VM admission | `Not run; requires launch artefacts` (VM also requires `VM build`, which passed) | ISA §9.1: “Order fixed: `symbols imports functions data devices layout schedule certificate coverage`; each at most once; unknown sections or fields are errors.” Admission requires the actual produced object. |
| Reference forward; VM forward | `Not run; requires native reference, <executor> admission` | Dialect §18.7 native-comparable success: “the artefact's results equal native”. |
| Reference reverse; VM reverse | `Not run; requires <executor> forward = native` | ISA §7.4: “A range of frames is reversed by visiting its frames in LIFO order”; §14.3 T4: “For every finite valid CC/R trace with retained run history and tapes, LIFO reversal per §7.4 restores the classical environment, memory, allocation table, device-model state and history to entry values; observed external events are not retracted.” |

The native baseline uses the original corpus and host libc; it is independent of both executors. It is not a completed §17.7 materialised comparison-link build. The earlier probe invocation consumed only the corpus module. The corrected gate now fails explicitly at the missing runtime link (brt-3sa), before production. Runtime-linked analysis, materialised comparison and any next lowering obligation must be evaluated after that prerequisite; this report does not assert those boundaries work or manufacture downstream defects. No VM or spec bead was filed because the hashtable never reached either executor and no spec defect was established.

## Native results already established

Both majors at `-O2` and `-O0` emitted these identical bytes (also [18](evidence/first-run/18/native.txt), [23](evidence/first-run/23/native.txt)); the gate records the ten little-endian i64 checksums in `native.bin` too:

```text
basic 0 0
basic 1 23131069394
basic 7 138063414195
basic 64 1083288249908
basic 1000 54380828142828
grow 0 4
grow 1 23131069404
grow 7 138063414247
grow 64 1083288250354
grow 1000 54380828149874
```

A follow-up driver check added `-fsanitize=address,undefined -fno-omit-frame-pointer` to the O0 native build, matching the original harness's documented intent. All ten inputs still pass on both majors, with no sanitizer diagnostic, and match O2 byte-for-byte. The final gate includes this check. The follow-up uses `--timeout 5` only to verify updated driver reporting and bead propagation quickly; it is **not** another 300-second production investigation and does not replace the first-run evidence. Its report/table/log are in [evidence/driver-smoke](evidence/driver-smoke/).

Five driver tests pass ([log](evidence/driver-checks.txt)): the marker-only transformation, real published clean-allocation reversal outputs from both executors with deliberate restoration-field corruption rejected, result/fault/hostcall handling, red-status/dependency/bead propagation, and rejection of a missing runtime without silently invoking an unlinked fallback. The clean-allocation control is five ISA instructions and **not the hashtable**. The only hashtable end-to-end path established is source → native build → all ten native answers (both majors), plus source → optimized LLVM IR. Source → device object → executor forward → reversal remains blocked.

## Ordered measured work plan

| Order | Bead | Owning repo | Stage | Determining sentence | Size / completion |
|---|---|---|---|---|---|
| 1 | brt-3sa | bennett-runtimes | Required runtime bitcode link | Dialect §19.1 requires linking runtimes/arith bitcode of major M before post-link optimization and the plugin. | **Medium**: publish allocator/libc closure for both majors (including optimizer-produced calloc); pass the actual link, then rerun. |
| 2 | bp-au6 | bennett-plugin | S2 reference-analysis path, observed on corpus-only probes; retest after runtime link | Dialect §5.2: “The analysis is the least fixpoint of the following rules over AV.” | **Medium**: preserve exact reference rules, complete both majors, then rerun this gate to expose the next rule. |

Run the required pipeline gate (currently fails immediately at runtime linking):

```sh
env -u DISPLAY -u WAYLAND_DISPLAY -u DBUS_SESSION_BUS_ADDRESS -u BROWSER \
  TMPDIR=/home/tobias/Projects/bennett-workspace/bennett-conformance/.tmp \
  /home/tobias/Projects/bennett-workspace/bennett-conformance/ladder/rung-2/gate.sh \
  --plugin-rev fafc40508598c34173aa2c295213536d13078a16 \
  --vm-rev eb7ca3ab3bf21eef9d418b037ecaeed3a471cf82
```
