# Rung 3: musl hello-world

North Star §8 acceptance remains:

> Forward output equal to native; run in the reversible tier with a console device model; un-run to the reset state; tape replay deterministic.

The corpus is exactly [fixtures/hello.c](fixtures/hello.c): `#include <stdio.h>`,
`printf("hello, world\n")`, return 0. There are no marker additions, stubs,
freestanding flags or replacement entry points. Clang may optimise printf to puts;
that libc closure is still required. Clause tags, expected bytes and diagnostic
fixture provenance are in [fixtures/expect.json](fixtures/expect.json); fixture
hashes are checked before use. The ISA and dialect clause IDs resolve against
this repository's published-contract graphs (CLAUDE.md rule 12).

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
| Dialect 0.2.10 | `023f5a771a814270634db3be9cfde88ded69f975` |
| Plugin | `00a5537f92af6b8b5d13e4422f3316e1e41005da` |
| VM | `9ac6d3e11b5b62f4e5f9a4c7294622a00db89bcd` |
| Driver | `306b6e90df85a6b0f59e1e57b04ff7fbcae37ce5` |
| Runtimes | `6808622d560b2e301c635c205eecde3d932b6df5` |
| Arith | `249e6192b95c6dbbb875510c32a677ba113e9af5` |
| ISA-spec inspection checkout | `997d70000eefba6e56ce0e4b1559a02757d15006` |
| Accepted reference checkout | `ba7d13f76e73c2af4e035d3f51145d2fad76bd2f` |

The contract is **ISA 0.8**, not the separately published 0.9. The reference
executable defaults to
`../bennett-isa-spec-review/reference-interpreter/target/release/bennett-isa-ref`.
Its actual binary hash is recorded; checkout identity alone is not a binary hash.
The gate does not rebuild or alter that checkout.

Plugin and driver builds use verified Git archives under `.tmp/rung3-<repo>-<sha>/`.
Runtime/arith bitcode also comes from archived pinned commits, avoiding concurrent
publication edits; runtime SHA256SUMS and the plugin's arith SHA pins are checked.
VM builds offline in a clean **detached** `.tmp/vm-<sha>` worktree, never in its
potentially dirty sibling tree. `--plugin-rev`, `--vm-rev`, `--driver-rev` override
those revisions explicitly; `--reference` overrides the binary. All effective
revisions, binary/library hashes, tool versions and commands are recorded.

## Required pipeline

1. Check fixtures and build the frozen plugin (both majors), VM and driver.
2. Compile ordinary hello.c with `clang -O2 -flto -emit-llvm`, target
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
   symbols, stdout exactly `hello, world\n` and status 0. The unmaterialised C
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
  `--bennett-llvm-major`, `--bennett-runtimes`, `--bennett-arith`,
  `--bennett-sysroot`, `--bennett-comparison-record` and
  `--bennett-comparison-output`, plus the specified R mode/entry. It must emit
  `out/native-comparison.json` containing `llvm_major`, `materialised_sha256`
  and `link_argv`. The gate executes that auditable same-major clang argv.
  It requires the actual device.bc, pinned runtimes/arith, explicit sysroot,
  `-nostdlib -nostdinc -static`, and an exact output path. Library-search flags,
  opaque linker/response inputs and file inputs outside the generated output,
  selected musl sysroot and pinned runtime publications are rejected. Startup
  and native effect backends must come from those publications. This integration
  is **not implemented by today's version-printing driver** (bennett-driver-cyg).
* The driver must supply `out/main/argv-replies.json` in the executors' existing
  `--replies` array format (`service`, `payload`, `reply`), with exactly one
  `bennett.argv` reply containing the selected argv/envp. The gate sends the same
  file during forward, reverse and replay. No console reply can be inserted.
* The proposed saved-tape CLI is `--replay-tape <JSON array>`. A mismatched tape
  must fail with a replay mismatch/divergence diagnostic or a structured FAILED
  result with `replay_mismatch`. Equivalent real adapters can replace this glue;
  the spelling is not mandated by ISA 0.8. Today's binaries reject the option
  (b-isa-spec-04d, bennett-vm-u8m), so no tape-consuming replay is claimed.

## Independent probes and evidence

Every run also audits pinned runtime exports, probes R-mode on the pinned
§11.6 console example, probes the driver for real outputs, checks the §11.5
ISA-0.8 typed-fault prerequisite, and runs a byte-identical published ISA
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

Driver glue checks (never corpus acceptance):

```sh
env -u DISPLAY -u WAYLAND_DISPLAY -u DBUS_SESSION_BUS_ADDRESS -u BROWSER \
  TMPDIR=/home/tobias/Projects/bennett-workspace/bennett-conformance/.tmp \
  python3 /home/tobias/Projects/bennett-workspace/bennett-conformance/ladder/rung-3/check_gate.py
```

Only command recording, git/hash helpers and failure/dependency propagation were
factored verbatim from rung 2 into `ladder/common/gate_support.py`. Rung 2 retains
its stage definitions, fixture route, build paths, CLI defaults and execution
logic; its driver checks and an AST comparison verify the factoring.
