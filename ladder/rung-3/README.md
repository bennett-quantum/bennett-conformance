# Rung 3: formatted integer musl printf

North Star §8 acceptance remains:

> Forward output equal to native; run in the reversible tier with a console device model; un-run to the reset state; tape replay deterministic.

This rung uses the **whole-device entry profile** of dialect §19.2: reversible
`main`, the `bennett.argv` launch reply, i32 status and a console-only device
tape. Its native comparison checks the materialised device closure. These
requirements leave mixed fat objects reachable (bennett-4ht): an ordinary native
host can retain native main, argv/envp and host libc while marked device modules
and their linked closures become ISA objects. The comparison's no-host-libc
checks and the device tape rules do not apply to that ordinary native host.

The corpus contains ordinary C with `main(void)` and no marker additions, stubs,
freestanding flags or replacement entry points:

| Workload | Required output |
|---|---|
| [hello.c](fixtures/hello.c) | Original `hello, world\n`, status 0; clang may fold this call to puts. |
| [printf_fmt.c](fixtures/printf_fmt.c) | `%s %d %u %x %c`, several arguments per call, width/precision, signed/unsigned extremes, and an unterminated final buffered line, status 0. |

The formatted oracle is a literal ASCII byte table with a hand derivation for
each line in [printf_fmt.expect.json](fixtures/printf_fmt.expect.json). The final
line has no LF and no explicit flush; returning from main must flush it. Floating
formats are a later clause. Every required pipeline, native, executor, reset,
saved-tape replay and corrupt-tape row runs on **both workloads** and both majors;
hello alone cannot pass. Clause tags, the original hello oracle and diagnostic
provenance remain in [expect.json](fixtures/expect.json). Source/oracle hashes are
checked before use. Clause IDs resolve against this repository's graphs for the
selected contract versions (CLAUDE.md rule 12).

Run from any directory:

```sh
env -u DISPLAY -u WAYLAND_DISPLAY -u DBUS_SESSION_BUS_ADDRESS -u BROWSER \
  TMPDIR=/home/tobias/Projects/bennett-workspace/bennett-conformance/.tmp \
  /home/tobias/Projects/bennett-workspace/bennett-conformance/ladder/rung-3/gate.sh
```

`gate.sh --help` describes overrides. The wrapper and Python entry point preserve a supplied repository-local
TMPDIR, default to this repository's `.tmp`, and unset desktop variables. Each corpus
command has a host timeout of at most 300 seconds; plugin/VM/driver/reference build limits
are 1800/600/600/600 seconds. No network, push, display or LLVM view pass is used. Builds accept at most five
jobs; LLVM majors execute sequentially so heavy corpus commands do not overlap.
Dependencies: LLVM 18 and 23 at `/usr/lib/llvm-<M>/bin`, CMake, Git, Python 3.12+,
Rust/Cargo with cached dependencies, a C++ compiler, and readelf. No service starts.

The exit code is **0 only if every row passes**. Missing prerequisites fail;
consumer stages are BLOCKED and inherit the actual prerequisite bead. A known
bead never turns a failure into a pass. An unrecognised failure prints UNFILED;
the gate never files or edits issues. The pipeline-ordered list, quoted determining
sentences, exact observed commands, completion criteria and estimated sizes live
in [blockers.json](blockers.json). The conformance tracking bead is
**bennett-conformance-7bx**, kept open until rung acceptance passes.

## Frozen implementations

Defaults are immutable commits in [pins.json](pins.json), not moving HEADs:

| Input | Commit | Reason |
|---|---|---|
| Dialect 0.3-r1 | `9bbf6dee3ab6ce94030e01a1af98df28fe19e683` | Pushed dialect main; publishes the accepted classical R profile and its ISA 0.9 compatibility identity. |
| Plugin | `5a9d8a2f597a84ad66d2575b3850cf86329d0d9c` | Pushed producer main; adds widening-cast lowering for wide exit values (job/r3trunc) and reduced analysis work (job/r3cpu) to the earlier crash, byval and relocation fixes. |
| VM | `058dc2229bf7cabb25f10b16d816b6802a92a9fe` | Pushed executor main at the ladder launch; adopts the 0.9 headers and explicit ABI family providers, retaining region lifetime support. |
| Driver | `aeb01e1872df12689890598398e49f72851424c2` | Published isolated hosted provider startup and normalised exit cleanup. |
| Runtimes | `7559e3bb40e824352ba6afaa5ed37e6a9860faf3` | Pushed runtime main; supplies the frozen musl headers, runtime, native-support bitcode and native-check adapter. |
| Arith | `5e9b4509f3ee3f95ac9f7d22b43359e72cf9fdab` | Exact arithmetic publication declared and hash-pinned by the selected plugin. Current arith main `2670e894c77745275f9bffb92aa80e7c7c805c27` has different bitcode; substituting it would fail the required producer digest check. |
| ISA 0.9 / reference source | `aaa8f1887ef68541ba9ddb07dfbb223b93fe7b1e` | Pushed ISA main with the 0.9 headers and K86–K92 reference checks; builds the reference actually used by this rung. |

The selected contract is **dialect 0.3-r1 / ISA 0.9**, the council-accepted
classical R profile in the pinned `spec/Bennett-Dialect-Spec-0.3-draft.md` §1.7.
The remaining 0.3 draft is ungraded. The producer's `[profiles.r_contract]`
declares this identity independently of its legacy 0.2.x default. Mapping 0.3,
normaliser 0.2.8 and saved-tape-v1 are checked as part of the selected profile.
The full dialect clause graph under `clauses/` in this rung was generated from
that pinned publication; inclusion of a clause does not grade the rest of 0.3.
Implementation pins were observed on pushed mains on 2026-11-11; arithmetic
remains the exact publication required by the producer. The default reference
is built offline in release mode from the frozen canonical ISA repository at
`--isa-spec-rev` (default: the ISA pin). The report records the effective source
commit, executable path, binary hash and build profile. `--reference` selects
an explicit binary override and records that selection and its hash. No live
review-checkout executable is selected implicitly. A failed reference build
blocks its admission and diagnostic consumers through a separate build row.

Plugin and driver builds use verified Git archives under `.tmp/rung3-<repo>-<sha>/`.
Runtime/arith bitcode also comes from archived pinned commits, avoiding concurrent
publication edits; runtime SHA256SUMS and the plugin's arith SHA pins are checked.
Every driver invocation explicitly selects the LLVM major and these archived
plugin/runtime/arith paths; an archived driver's sibling-relative defaults cannot
locate them. `--runtimes18/23` overrides select an explicit runtime file whose hash
is recorded. Publication checks compare the selected arithmetic bytes against the plugin's
compiled publication pin; any mismatch remains a failure (bp-mq6). The gate preserves that
failure instead of substituting older arithmetic or bypassing its hash check.
VM builds offline in a clean **detached** `.tmp/vm-<sha>` worktree, never in its
potentially dirty sibling tree. `--plugin-rev`, `--vm-rev`, `--driver-rev` override
those revisions explicitly; `--isa-spec-rev` selects the frozen reference source. All effective
revisions, binary/library hashes, tool versions and commands are recorded.

## Required pipeline

1. Check fixtures and build the frozen plugin (both majors), VM, driver and reference.
2. Compile each ordinary corpus source with `clang -O2 -flto -emit-llvm`, target
   `x86_64-unknown-linux-musl`, explicit `--sysroot`, `-nostdinc`, and only the musl
   include directory. Default consumer layout is
   the frozen runtime publication's `dist/llvm<M>/musl-sysroot/{include,lib}`; this filename/layout
   is configurable via `--sysroot18/23`, not specified by the dialect. There is no
   fallback to host stdio.h or the legality checker's small seeds.
3. Link program + same-major runtimes + arith using llvm-link --only-needed,
   then `opt -passes=lto<O3>` (§19.1). Invoke the production Bennett pass in
   `--bennett-tier=reversible --bennett-entry=main` mode (§19.2). Require all
   launch/build/certificate/coverage/blocker artefacts, verify bitcode, reject
   nonempty blockers, and require an R body with `effect console.write`, a
   declared console model and the argv CC launch stub.
4. Build the **materialised** native comparison (§17.7) using the frozen driver
   integration described below, same LLVM major, runtimes and arith. Require both
   O0/O2 components to link statically with no host libc/libm, dynamic dependencies
   or undefined symbols. Execute their isolated hosted launchers and require stdout
   exactly equal to the workload's literal oracle, status 0 and no fault. The unmaterialised C
   source compiled against host libc is never a baseline for this gate.
5. Both executors must admit/hash the actual produced object, run main through
   its launch, report exit status 0, and match native console and tape bytes.
   Only the §19.2 argv host service is allowed; console/libc hostcalls fail.
6. Both `run-program --reverse` and retained-history `reverse` must report
   REVERSED, full-state equality, byte-memory equality, equal entry/final memory
   digests, no first difference, empty console and zero effect-tape length.
   Digests are compared within one executor, never across implementations.
7. Save the forward tape and give it back to the executor for replay with the
   same (empty) entropy. Require equal results, console, ordered tape and tape
   length; also corrupt a recorded byte and require a replay-mismatch verdict.
   A second fresh execution that ignores the input tape cannot pass.

## Consumer integration at blocked boundaries

The published specs fix semantics, not all command/file spellings. The gate
uses rung-2's launch artefact layout and these explicit **adapter conventions**;
missing adapters are tracked as integration blockers, not invented ISA defects:

* The native stage selects the frozen driver's published isolated hosted profile
  with explicit `--bennett-native-provider` and `--bennett-native-target`, plus
  matching-major plugin/runtime/arithmetic/checker/support/sysroot paths. The real
  C launch-provider DSO is built offline from VM publication
  `55cdc8c04b776684a44bec2b0058f309ed5a7d36`; the canonical JCS ideal target is
  `targets/ideal-all-to-all.json` from target publication
  `dd036711936c496f53f6f1a9bebeb686335bf7cc`. Both identities are in `pins.json`;
  executable/target hashes are recorded in the run manifest. The provider remains
  distinct from the VM executor pin. No host libc substitutes for the component.
* Producer, probe and comparison receive the same explicit data base **65536** and
  text base **4194304**, declared in `pins.json`. Existing ISA objects are never
  relocated. The original materialised module and producer launch/build identities
  remain the comparison subject, including original device-global IDs.
* [hosted_native.py](hosted_native.py) audits each existing O0/O2 **component** ELF,
  its exact static link/codegen argv, fixed global addresses/extents/alignment, heap
  boundary, input hashes, checker coverage and pinned runtime/arithmetic closure.
  [hosted_closure.py](hosted_closure.py) validates ordered prepare/check/finalise,
  library, layout, gateway and host-import transformations with a closed input
  vocabulary. The complete bitcode chain, including the pinned memory checker, is
  replayed in scratch storage and both checked/final output hashes are verified.
  No component is relinked as a new baseline and no ELF check targets a launcher
  script. Hosted launcher/bootstrap/provider imports are audited separately through
  their dependency inventories, hashes and actual loaded namespace closure.
* For each O0/O2 execution the gate removes any previous runtime receipt, then
  requires a fresh completed receipt, real isolated namespace, successful ISA load,
  one binding call, one entry call and successful unload. It checks actual process
  status/stdout against the literal oracle and rejects signal/provider faults.
  It rechecks original inputs and both dependency closures after execution.
  The source definedness and materialised fault obligations remain separate;
  runtime checker coverage does not claim source UB erased by optimisation.
  `native_audit.py` retains the legacy static adapter audit and its regressions.
* Direct `opt` production does not run the driver's process-build serializer.
  The gate acts as the launch adapter described by the pinned plugin's
  `docs/R_PROCESS_ARGUMENTS.md` and requests empty argv/envp, matching the pinned
  driver's default in `docs/PROCESS_ARGUMENTS.md`. It derives the protocol from
  `build.json.entry_initialisation.arguments`, checks the object's declared
  service/version and pointer width, and writes consumer `argv-replies.json`
  in the executors' published `--replies` format (`service`, `payload`, `reply`).
  A `bennett-argv-bytes-1` build receives the complete little-endian empty-vector
  payload twice, for the 24-byte header and full allocation requests, using
  `bennett.argv.bytes` v1. Other current builds retain one zeroed legacy
  `bennett.argv` v0.2.10 header (i32 argc and two target-width null pointers).
  Unknown protocols/versions fail. Reply, build and object hashes plus the
  selected process context are recorded; forward, reverse and replay use the
  same bytes and must report exactly those exchanges. No console/libc reply
  can be inserted. The driver still publishes reply generation for its own
  process-build route; no driver serializer defect is implied here.
* The proposed saved-tape CLI is `--replay-tape <JSON array>`. A mismatched tape
  must fail with a replay mismatch/divergence diagnostic or a structured FAILED
  result with `replay_mismatch`. Equivalent real adapters can replace this glue;
  the spelling is not mandated by ISA 0.8. Both binaries accept and pass the saved-tape
  control probe in the current run. Required hello-world replay and corrupted-tape
  rejection remain blocked by earlier pipeline stages; a control probe alone
  cannot establish those rows. The former unknown-option observations
  (b-isa-spec-04d, bennett-vm-u8m) are historical.

## Independent probes and evidence

Every run also audits pinned runtime exports, probes R-mode on the pinned
§11.6 console example, probes the driver for real outputs, checks the selected
R-contract prerequisite, and runs a byte-identical published ISA
`classical.bisa @effects` device/reset control on both executors. The control
uses console, entropy, clock and block effects. It is **not the hello-world
corpus** and cannot satisfy a required row. The runtime export audit checks
printf/puts/write/exit and __bennett_dev_console_write as publication coverage,
not a claim each symbol must survive hello.c optimisation. R uses ISA effects
and tapes (§11.2); L's __bennett_dev_console_* state (§11.3) is separately owed.

Each run creates `.tmp/rung-3/<UTC timestamp>/` with `report.json`, `table.md`,
per-command stdout/stderr, inputs and generated artefacts. `report.json` records
argv arrays, exits, timeouts, elapsed times, pins, hashes, environment and stage
errors. `.tmp/rung-3/latest` holds the newest path. `--output` changes the evidence
parent. [RESULTS.md](RESULTS.md) records the real run and distinguishes measured
frontiers from dependencies. Compact report/table/log copies and full evidence
hashes are committed in `evidence/`; raw build archives remain under `.tmp`.
Hello's original row names and artefact paths remain; formatted rows are prefixed
`printf_fmt:` with artefacts under `<major>/printf_fmt/`. Each executor has
separate saved-tape and corrupt-tape rows.

Driver glue checks (never corpus acceptance):

```sh
env -u DISPLAY -u WAYLAND_DISPLAY -u DBUS_SESSION_BUS_ADDRESS -u BROWSER \
  TMPDIR=/home/tobias/Projects/bennett-workspace/bennett-conformance/.tmp \
  timeout 60s python3 -B -m unittest discover \
    -s /home/tobias/Projects/bennett-workspace/bennett-conformance/ladder/rung-3/tests -v
env -u DISPLAY -u WAYLAND_DISPLAY -u DBUS_SESSION_BUS_ADDRESS -u BROWSER \
  TMPDIR=/home/tobias/Projects/bennett-workspace/bennett-conformance/.tmp \
  timeout 60s python3 -B /home/tobias/Projects/bennett-workspace/bennett-conformance/ladder/rung-3/check_gate.py
```

The new suite also replays a tiny generated LLVM/static-link receipt on both
majors when those tools are installed. It checks an independently derived return
value and rejects a spoofed pre-existing intermediate. It builds no plugin, VM
or driver. It materialises `scripts/native-check` and `native-check.cpp` from
`pins.json`'s exact runtime revision and runs the real adapter, including its
major-matched checker build. Its small arithmetic and ABI-memory modules are
test inputs, not the musl acceptance corpus. Missing LLVM tools skip this one
adapter integration test; the Python policy checks still run.

Only command recording, git/hash helpers and failure/dependency propagation were
factored verbatim from rung 2 into `ladder/common/gate_support.py`. Rung 2 retains
its stage definitions, fixture route, build paths, CLI defaults and execution
logic; its driver checks and an AST comparison verify the factoring.
