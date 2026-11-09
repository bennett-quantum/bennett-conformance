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

`gate.sh --help` describes overrides. The wrapper and Python entry point force
TMPDIR to this repository's `.tmp` and unset desktop variables. Each corpus
command has a host timeout of at most 300 seconds; plugin/VM/driver build limits
are 1800/600/600 seconds. No network, push, display or LLVM view pass is used.
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

| Input | Commit |
|---|---|
| Dialect 0.3-r1 | `830bd7f2a328556af983f2c3aa4c75ef9eb29b65` |
| Plugin | `abe5557977e9e9a38d8922aff69cbfc638995798` |
| VM | `83b5f7f0248fe1c36ebfe3acd7fb187ed301affa` |
| Driver | `8b1b4a1814ce3c99f4357d2e11f048b1ff2a090b` |
| Runtimes | `40f243e2a2128eb61d0d2ab4786aa082a8e2c90f` |
| Arith | `cd245bcb07ad38da629d9b47f642e46e3c25cae9` |
| ISA 0.9 / reference source | `695cb4b7c330742820c6b315337a260e56de8bce` |

The selected contract is **dialect 0.3-r1 / ISA 0.9**, the council-accepted
classical R profile in the pinned `spec/Bennett-Dialect-Spec-0.3-draft.md` §1.7.
The remaining 0.3 draft is ungraded. The producer's `[profiles.r_contract]`
declares this identity independently of its legacy 0.2.x default. Mapping 0.3,
normaliser 0.2.8 and saved-tape-v1 are checked as part of the selected profile.
The full dialect clause graph under `clauses/` in this rung was generated from
that pinned publication; inclusion of a clause does not grade the rest of 0.3.
Implementation pins are the pushed mains observed on 2026-11-09; the exact
remote observations and execution result are appended to RESULTS.md.
The reference executable can be selected with `--reference`; the promotion
run builds it from the pinned ISA source and records its binary hash.

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
those revisions explicitly; `--reference` overrides the binary. All effective
revisions, binary/library hashes, tool versions and commands are recorded.

## Required pipeline

1. Check fixtures and build the frozen plugin (both majors), VM and driver.
2. Compile each ordinary corpus source with `clang -O2 -flto -emit-llvm`, target
   `x86_64-unknown-linux-musl`, explicit `--sysroot`, `-nostdinc`, and only the musl
   include directory. Default consumer layout is
   `bennett-runtimes/dist/llvm<M>/musl-sysroot/{include,lib}`; this filename/layout
   is configurable via `--sysroot18/23`, not specified by the dialect. There is no
   fallback to host stdio.h or the legality checker's small seeds.
3. Link program + same-major runtimes + arith using llvm-link --only-needed,
   then `opt -passes=lto<O3>` (§19.1). Invoke the production Bennett pass in
   `--bennett-tier=reversible --bennett-entry=main` mode (§19.2). Require all
   launch/build/certificate/coverage/blocker artefacts, verify bitcode, reject
   nonempty blockers, and require an R body with `effect console.write`, a
   declared console model and the argv CC launch stub.
4. Build the **materialised** native comparison (§17.7) using the frozen driver
   integration described below, same LLVM major, runtimes and arith. Require a
   static link with no host libc/libm, no dynamic dependencies or undefined
   symbols, stdout exactly the workload's literal oracle and status 0. The unmaterialised C
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

* The native stage requests the frozen driver with `--bennett-materialised`,
  `--bennett-llvm-major`, `--bennett-plugin`, `--bennett-runtimes`, `--bennett-arith`,
  `--bennett-sysroot`, the frozen runtime `--bennett-native-check`,
  `--bennett-comparison-record` and
  `--bennett-comparison-output`, explicit `-bennett-out=out/native-work`, and the
  selected versions/R mode/entry. `out/native-comparison.json` must contain
  `llvm_major`, `materialised_sha256`, `runtimes_sha256`, `arith_sha256` and
  `link_argv`. A direct link must use those three exact files. A multi-stage
  link supplies `bitcode_link_argv`, `linked_sha256` of the **final**
  `comparison.bc` after support/layout adaptation and cleanup, and ordered
  successful `commands` (`argv`, `success`) in `out/native-work/manifest.json`.
  That manifest also records `native_check_adapter` and `checked_materialised`
  artifacts (`path`, `sha256`).
  [native_audit.py](native_audit.py) audits and **replays** the bitcode chain
  from those hash-checked leaves, rather than trusting a merged filename. It
  accepts same-major llvm-link merges, runtime memcpy/memmove/memset extraction,
  assembly of the published layout constant, and strip-dead-prototypes/verify.
  The mandatory `native-check MAJOR INPUT OUTPUT` stage must use the pinned
  runtime adapter, whose complete source publication is verified when frozen.
  Replay runs that adapter, verifies its recorded output hash and unchanged
  input, and requires the checked output to feed the final closure. The final
  closure hash and all three original input hashes are checked after replay.
  Native support must be hashed and come from the pinned runtime publication;
  sysroot native support must match that publication byte for byte. Other
  leaves/transforms fail loudly. Final clang can consume the reproduced merged
  bitcode with the driver's linker script, `_start`, map and no-undefined/fatal
  warnings options. Scripts cannot introduce file/library inputs; comma/response
  and library-search injection is forbidden. `-O2 -nostdlib -nostdinc -static`
  and the exact output path are required, followed by actual ELF dynamic-library,
  interpreter and undefined-symbol audits. No host libc/libm is accepted at any
  merge or link stage. The driver still receives the explicit musl sysroot for
  preparation; a final bitcode-only clang command need not repeat header options.
  This integration remains unmeasured for the musl corpus because the frozen
  prerequisite pins are red; passing synthetic adapter tests is not acceptance.
* The driver must supply `out/main/argv-replies.json` in the executors' existing
  `--replies` array format (`service`, `payload`, `reply`), with exactly one
  `bennett.argv` reply containing the selected argv/envp. The gate sends the same
  file during forward, reverse and replay. No console reply can be inserted.
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
