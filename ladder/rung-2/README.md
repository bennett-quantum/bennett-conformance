# Rung 2: pointers, structs, byte memory

North Star §8 acceptance:

> A C hashtable (the existing 258-line corpus): forward result equal to native on every tested input (independent check), then reversed byte-exactly, both executors.

Run from any directory:

```sh
env -u DISPLAY -u WAYLAND_DISPLAY -u DBUS_SESSION_BUS_ADDRESS -u BROWSER \
  TMPDIR=/home/tobias/Projects/bennett-workspace/bennett-conformance/.tmp \
  /home/tobias/Projects/bennett-workspace/bennett-conformance/ladder/rung-2/gate.sh
```

`gate.sh --help` lists revision, reference-executable, output-directory, build-parallelism and timeout options. `--runtimes18` / `--runtimes23` select published runtimes bitcode (defaults: `bennett-runtimes/dist/llvm<M>/bennett-runtimes.bc`). `--probe-unlinked` additionally runs a diagnostic corpus-only pass; it can never satisfy any production stage. Default plugin/VM revisions are their HEADs when invoked. `--plugin-rev` and `--vm-rev` reproduce the revisions in [RESULTS.md](RESULTS.md). No push, network access, display, LLVM `-view-*`, or server is used. Builds require installed LLVM 18/23, CMake, a C++ compiler, Rust/Cargo, Git and Python 3.12+. The reference binary defaults to the accepted ISA-0.8 review interpreter named in the task, not the potentially moving interpreter in the ISA worktree.

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

1. Build the plugin from a Git archive of a frozen commit into `$TMPDIR/rung2-plugin-<sha>/build`, using its dual-major CMake superbuild. This leaves the concurrently edited plugin checkout untouched. Build the VM offline from a detached worktree of a frozen HEAD at `$TMPDIR/vm-<sha>`, as required; keep that worktree for repeat runs.
2. Compile the **original** corpus plus original `main_golden.c` with each major's clang at `-O2` and `-O0 -fsanitize=address,undefined -fno-omit-frame-pointer`. Require identical complete stdout, the exact ten-case order, and equality across majors. Record the stdout and all ten checksums as signed 64-bit little-endian bytes. Domain: `basic` and `grow`, each at `n = 0, 1, 7, 64, 1000`. This is an independent native baseline, not yet a claim of the materialised native comparison build of dialect §17.7 or universal UB-freedom.
3. Compile the marked corpus with `clang -O2 -flto -emit-llvm`, link the major-matched runtimes and pinned arith bitcode with `llvm-link --only-needed`, then run post-link `lto<O3>` and the **production** `bennett` pass at dialect 0.2.10 / ISA 0.8. No analysis-only, scalar-reference, partial-corpus, or expected-failure path can satisfy this gate. Budgets: `(steps, heap, stack, depth, sstack) = (10000000,1048576,65536,256,65536)`. Runtime publication is required before production; absent bitcode fails explicitly (brt-3sa). No allocator shim is fabricated. The earlier corpus-only probes are retained as diagnostic evidence (bp-au6), and must be repeated on the runtime-linked module before claiming that frontier persists. The §17.7 materialised native comparison boundary is also unverified; native corpus checks are an independent baseline.
4. Require `device.bc`, `host.bc`, `launches.json`, and per-entry object/build/certificate/coverage/blocker/launch-record files. Verify bitcode with LLVM; require both driver launch entries and empty blocker reports. Run both executors' `check` and `hash` on each launch object.
5. Run all ten cases with each executor's `run-program`. Require `COMPLETED`, `CLOSED`, exactly one i64 result, no fault and no host requests. Reconstruct `main_golden.c` stdout and the little-endian checksum bytes and compare **both byte-for-byte** with native.
6. Repeat every case with `run-program --reverse` and separately `reverse`. Require forward results still equal native, `REVERSED`, `equals_entry=true`, `memory_equals_entry=true`, `first_difference=null`, and equal entry/final memory digests **within that executor**. The executor equality operation checks complete state, including byte memory, allocation bookkeeping and handles. Digest spellings differ across implementations and are never cross-compared. This uses retained-history reversal, not a second execution of the C function.

`--timeout` (default 300 seconds) bounds each corpus command; plugin and VM builds have 1800/600-second limits. A timeout is an observed resource failure, not evidence of a semantic rejection or proven nontermination. Raising the limit is available for investigation; it does not turn the failure into a pass.

The initial corpus-only runs, corrected runtime-link gate and ordered measured blockers are in [RESULTS.md](RESULTS.md). The gate is deliberately red until the hashtable passes every stage on both executors and both majors. No plugin or VM source is owned by this rung.

Current state (2026-10-31): **red**, first blocked by missing published runtime bitcode (`brt-3sa`, `bennett-runtimes`, §19.1). Corpus-only production probes also timed out at 300 seconds (`bp-au6`, `bennett-plugin`), without a diagnostic or object; these do not claim a fully linked pipeline. Native results agree across majors; no hashtable execution or reversal has yet passed on an executor.

Driver checks, using the detached VM built by the gate:

```sh
env -u DISPLAY -u WAYLAND_DISPLAY -u DBUS_SESSION_BUS_ADDRESS -u BROWSER \
  TMPDIR=/home/tobias/Projects/bennett-workspace/bennett-conformance/.tmp \
  python3 /home/tobias/Projects/bennett-workspace/bennett-conformance/ladder/rung-2/check_driver.py \
  --vm /home/tobias/Projects/bennett-workspace/bennett-conformance/.tmp/vm-eb7ca3ab3bf21eef9d418b037ecaeed3a471cf82/target/debug/bennett-vm
```

These use a published allocation control through both executors, mutate the reported restoration flags/digest to check rejection, test integer/fault/host-request result handling, and verify that a known failure remains red and blocks dependants. They do not substitute for the hashtable run.
