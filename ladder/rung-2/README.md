# Rung 2: pointers, structs, byte memory

North Star §8 acceptance:

> A C hashtable (the existing 258-line corpus): forward result equal to native on every tested input (independent check), then reversed byte-exactly, both executors.

Run from any directory:

```sh
env -u DISPLAY -u WAYLAND_DISPLAY -u DBUS_SESSION_BUS_ADDRESS -u BROWSER \
  TMPDIR=/home/tobias/Projects/bennett-workspace/bennett-conformance/.tmp \
  /home/tobias/Projects/bennett-workspace/bennett-conformance/ladder/rung-2/gate.sh
```

`gate.sh --help` lists all selectors. With no revision overrides, the gate
uses [pins.json](pins.json), reproducing the recorded plugin, VM, runtime,
arithmetic and release-reference inputs in [RESULTS.md](RESULTS.md). Every
source is frozen; no live distribution or review checkout supplies defaults.
`--plugin-rev`, `--vm-rev`, `--runtimes-rev`, `--driver-rev`, `--isa-spec-rev`,
and `--arith-rev` select revisions explicitly. `--runtimes18/23`, `--arith18/23`,
and `--reference` provide explicit artefacts and conflict with the matching
revision selector. `--probe-unlinked` remains a diagnostic, never production
acceptance. Builds require LLVM 18/23, CMake, a C++ compiler, Rust/Cargo, Git
and Python 3.12+; there is no push, network access, display or server.

The exit status is **0 only when every stage passes**. A failure is red even when its diagnostic has a known bead. Dependent stages are `BLOCKED`, inherit the bead, and are never counted as executor passes. New failures print `UNFILED`; filing beads is a human/agent action, never a gate side effect. Each invocation writes a fresh timestamped directory under `$TMPDIR/rung-2/`; `$TMPDIR/rung-2/latest` contains its path. `report.json` records commands, exits, input domain, binary hashes, revisions and stage errors; `table.md` is the per-major table. Command stdout/stderr and generated IR/artefacts remain beside them. This directory can be changed with `--output`.

## Fixture and marker provenance

`fixtures/hashtable.c` and `fixtures/main_golden.c` are byte-for-byte copies from `/home/tobias/Projects/BennettVM.jl/test/reference/c/`. The former has 258 lines. `fixtures/sha256.json` pins both originals and the marked variant; the gate verifies those pins and reconstructs the marked variant to enforce marker-only changes. The original checkout is never edited.

The determining dialect 0.2.10 @ `023f5a7` §3.8 sentence is:

> **clang (C, C++)** and **rustc** emit markers as direct calls to the declaration (emission summary).

`hashtable_marked.c` adds, immediately after the standard includes:

```c
/* Dialect 0.2.10 §3.8: direct C calls; §4.2: mark both driver closures CC. */
extern void *__bennett_qalloc(uint64_t, uint64_t);
extern void __bennett_qfree(void *);
```

It adds these two statements at the beginning of **each** of `ht_demo_basic` and `ht_demo_grow`:

```c
void *rung2_marker = __bennett_qalloc(0, 1);
__bennett_qfree(rung2_marker);
```

The preceding blank lines are the only other additions. [The complete diff](fixtures/marker-route.diff) records them. §3.2 specifies these signatures; §4.2 assigns CC to “a function that contains a marker call” and its CC closure. The qubit is clean and immediately freed; there is no measurement or output change. This is the CC retained-history route (ISA §§3.4/7.4/10), not a dialect R-entry promise, which would encounter the independently specified ISA-7 dependency (§11.5). The hashtable algorithms, driver signatures, arithmetic, allocation calls and test domain are preserved. No native marker definitions are linked into production.

## What the gate checks

1. Build the plugin from a Git archive of a frozen commit into `$TMPDIR/rung2-plugin-<sha>/build`, using its dual-major CMake superbuild. This leaves the concurrently edited plugin checkout untouched. Build the VM offline from a detached worktree of the selected commit at `$TMPDIR/vm-<sha>`, as required; keep that worktree for repeat runs.
2. Compile the **original** corpus plus original `main_golden.c` with each major's clang at `-O2` and `-O0 -fsanitize=address,undefined -fno-omit-frame-pointer`. Require identical complete stdout, the exact ten-case order, and equality across majors. Record the stdout and all ten checksums as signed 64-bit little-endian bytes. Domain: `basic` and `grow`, each at `n = 0, 1, 7, 64, 1000`. This is an independent native baseline, not yet a claim of the materialised native comparison build of dialect §17.7 or universal UB-freedom.
3. Compile the marked corpus with `clang -O2 -flto -emit-llvm`, link the major-matched runtimes and pinned arith bitcode with `llvm-link --only-needed`, then run post-link `lto<O3>` and the **production** `bennett` pass at the versions declared by the frozen plugin (`0.2.10` / `0.8` for the recorded pin). No analysis-only, scalar-reference, partial-corpus, or expected-failure path can satisfy this gate. Budgets: `(steps, heap, stack, depth, sstack) = (10000000,1048576,65536,256,65536)`. Runtime publication is required before production; absent bitcode fails explicitly (brt-3sa). No allocator shim is fabricated. The earlier corpus-only probes are retained as diagnostic evidence (bp-au6), and must be repeated on the runtime-linked module before claiming that frontier persists. The §17.7 materialised native comparison boundary is also unverified; native corpus checks are an independent baseline.
4. Require `device.bc`, `host.bc`, `launches.json`, and per-entry object/build/certificate/coverage/blocker/launch-record files. Verify bitcode with LLVM; require both driver launch entries and empty blocker reports. Run both executors' `check` and `hash` on each launch object.
5. Run all ten cases with each executor's `run-program`. Require `COMPLETED`, `CLOSED`, exactly one i64 result, no fault and no host requests. Reconstruct `main_golden.c` stdout and the little-endian checksum bytes and compare **both byte-for-byte** with native.
6. Repeat every case with `run-program --reverse` and separately `reverse`. Require forward results still equal native, `REVERSED`, `equals_entry=true`, `memory_equals_entry=true`, `first_difference=null`, and equal entry/final memory digests **within that executor**. The executor equality operation checks complete state, including byte memory, allocation bookkeeping and handles. Digest spellings differ across implementations and are never cross-compared. This uses retained-history reversal, not a second execution of the C function.

`--timeout` (default 300 seconds) bounds each corpus command; plugin and VM builds have 1800/600-second limits. A timeout is an observed resource failure, not evidence of a semantic rejection or proven nontermination. Raising the limit is available for investigation; it does not turn the failure into a pass.

The initial corpus-only runs, corrected runtime-link gate and ordered measured blockers are in [RESULTS.md](RESULTS.md). The gate exits non-zero unless the hashtable passes every stage on both executors and both majors. No plugin or VM source is owned by this rung.

Current state: **complete**. The recorded gate run exited 0 on both LLVM majors and both executors; revisions, evidence directory and per-stage table are the top block of [RESULTS.md](RESULTS.md). Earlier red runs and their blockers are retained there as history.

Driver checks, using the detached VM built by the gate:

```sh
env -u DISPLAY -u WAYLAND_DISPLAY -u DBUS_SESSION_BUS_ADDRESS -u BROWSER \
  TMPDIR=/home/tobias/Projects/bennett-workspace/bennett-conformance/.tmp \
  python3 /home/tobias/Projects/bennett-workspace/bennett-conformance/ladder/rung-2/check_driver.py \
  --vm "$TMPDIR/vm-13f5a9ed7482b8645c2e37e2ed95e0a7df9dfa88/target/debug/bennett-vm" \
  --reference "$TMPDIR/bennett-isa-spec-ba7d13f76e73c2af4e035d3f51145d2fad76bd2f/target/release/bennett-isa-ref"
```

These use a published allocation control through both executors, mutate the reported restoration flags/digest to check rejection, test integer/fault/host-request result handling, and verify that a known failure remains red and blocks dependants. They do not substitute for the hashtable run.

## Frozen sibling revisions

`--runtimes-rev` uses tracked runtime bitcode from a clean detached cache;
`--isa-spec-rev` builds that frozen reference interpreter offline in release
mode. `--driver-rev` freezes and records driver provenance: the gate continues
to invoke the plugin directly and does not claim to execute a driver binary.
`--arith-rev` selects frozen arithmetic publications, still checked against the
plugin's digest pins. When `--plugin-rev` is supplied, its declared arithmetic
commit supplies the default bitcode, avoiding live arithmetic paths. Older
plugin publications with only path/digest pins use the recorded arithmetic pin
unless explicitly overridden. Selected arithmetic
is passed to both the bitcode linker and the production plugin.
Without revision selectors, all six primary inputs come from pins.json,
including a release reference built from canonical bennett-isa-spec.
Explicit `--runtimes18/23`, `--arith18/23`, and `--reference` paths conflict
with their sibling's revision option; conflicting input is rejected before
building. Resolved pins and reference profile/hash are recorded in report.json.

Both rungs' revision checks run via
`python3 -m unittest discover -s ladder/rung-1/tests -v`.
For a full rerun on `tobias-ThinkPad-X1-Carbon-Gen-11`, use the same
12000-second outer hang guard documented in [rung 1](../rung-1/README.md), with
machine factor 1 and exit 124 classified UNKNOWN. This exceeds 10x the last
durable rung-2 receipt's 11.531 summed command wall seconds
(`evidence/required-pipeline/report.json`, 2026-10-31), allowing substantial
room for complete builds and corpus execution beyond that early failing run.
Use a `bennett-heavy-ladder-rung-2-<UTC time>` user scope only when no other
heavy scope exists, and retain outer wall/user/system CPU seconds.

Production requests the dialect and ISA versions
declared by the frozen plugin's `SPEC_VERSIONS.toml`. Missing or empty version
declarations fail before its build. The report records the requested versions
and the declaration's SHA-256. With no revision selectors, the historical
production defaults remain unchanged. Fixture clause/provenance pins continue
to describe the original corpus; the production version fields describe the
selected plugin invocation.

The recorded input pins and the unused driver provenance pin are documented
in [rung 1](../rung-1/README.md#frozen-sibling-revisions). Each primary revision
selector overrides its recorded pin. An explicitly selected plugin supplies
its published arithmetic dependency when arithmetic is not explicit; the
manifest records this choice. Explicit arithmetic must match the plugin digest.

Before configuring/building an archived plugin, the gate materialises its
published gate.toml siblings in clean detached sources and exports
`BENNETT_GATE_LAYOUT` to a layout of symlinks beside the plugin archive. This
matches the gate tool's sibling lookup structure, without running gate_run or
a sibling gate. Historical plugins without gate.toml receive a layout with the
recorded arithmetic pin. The manifest distinguishes plugin build pins from
the runtime/publication inputs used for the actual corpus.
