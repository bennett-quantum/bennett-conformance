# Rung 3 measured gate — 2026-11-01T12:54:05.462604+00:00

**Exit 1. The gate implementation is complete; rung acceptance is blocked.**
Neither major compiled the ordinary hello.c against a musl sysroot. No native
comparison, hello-world object, executor forward equality, hello-world reset or
saved-tape replay has passed. No host-libc baseline or freestanding substitute
was used. All downstream required rows inherit the measured first blocker.

Raw evidence: `.tmp/rung-3/20261101T125405.461872Z/`.
[Recorded report](evidence/20261101T125405.461872Z/report.json),
[table](evidence/20261101T125405.461872Z/table.md),
[gate log](evidence/20261101T125405.461872Z/gate.log), and
[SHA-256 inventory of every raw evidence file](evidence/20261101T125405.461872Z/raw-evidence-sha256.json).
The report contains exact argv arrays, exits, elapsed times, host timeouts,
stage errors, hashes, pins, tool versions and the headless environment. All
corpus commands used the 300-second host limit. Cached source archives were
verified byte-for-byte before builds. No source in a sibling repository was edited.

## Both LLVM majors

| Stage | LLVM 18 | LLVM 23 |
|---|---|---|
| fixtures | PASS | PASS |
| plugin build | PASS | PASS |
| VM build | PASS | PASS |
| driver build | PASS | PASS |
| musl C front end | FAIL · brt-1ft | FAIL · brt-1ft |
| runtime bitcode link | BLOCKED · brt-1ft | BLOCKED · brt-1ft |
| post-link LTO | BLOCKED · brt-1ft | BLOCKED · brt-1ft |
| reversible production | BLOCKED · brt-1ft | BLOCKED · brt-1ft |
| launch artefacts | BLOCKED · brt-1ft | BLOCKED · brt-1ft |
| native comparison | BLOCKED · brt-1ft | BLOCKED · brt-1ft |
| reference admission | BLOCKED · brt-1ft | BLOCKED · brt-1ft |
| reference forward = native | BLOCKED · brt-1ft | BLOCKED · brt-1ft |
| reference reset | BLOCKED · brt-1ft | BLOCKED · brt-1ft |
| reference tape replay | BLOCKED · brt-1ft | BLOCKED · brt-1ft |
| VM admission | BLOCKED · brt-1ft | BLOCKED · brt-1ft |
| VM forward = native | BLOCKED · brt-1ft | BLOCKED · brt-1ft |
| VM reset | BLOCKED · brt-1ft | BLOCKED · brt-1ft |
| VM tape replay | BLOCKED · brt-1ft | BLOCKED · brt-1ft |
| runtime libc/console audit (diagnostic) | FAIL · brt-1ft, brt-cgb | FAIL · brt-1ft, brt-cgb |
| R-mode probe (diagnostic) | FAIL · bp-ivy | FAIL · bp-ivy |
| driver probe (diagnostic) | FAIL · bennett-driver-cyg | FAIL · bennett-driver-cyg |
| ISA-0.8 R contract (prerequisite) | FAIL · b-isa-spec-221 | FAIL · b-isa-spec-221 |
| reference device/reset control (diagnostic) | PASS | PASS |
| reference replay API (diagnostic) | FAIL · b-isa-spec-04d | FAIL · b-isa-spec-04d |
| VM device/reset control (diagnostic) | PASS | PASS |
| VM replay API (diagnostic) | FAIL · bennett-vm-u8m | FAIL · bennett-vm-u8m |

The rows labelled diagnostic exercise separate prerequisites, never replace
required hello-world rows. The ISA contract prerequisite is independently red:
the pinned dialect §11.5 blocks every whole-program R entry on ISA 0.8 because
its mandatory pre-instruction budget faults need ISA-7. ISA 0.9 is published
alongside; it was not silently substituted. The isolated opt mode probe rejects
options before it can report that contract blocker, so no effect-lowering
verdict on the real musl program is inferred.

## Ordered blockers

| Order | Bead / owning repository | Measured frontier / fixed means | Size |
|---|---|---|---|
| 1 | **brt-1ft**, bennett-runtimes | Both ordinary musl compiles: stdio.h not found. Publish real musl headers, libc/stdio/write/flush/exit and optimizer-produced puts closure, startup and native comparison support for both majors. | large, 1440 min |
| 2 | **brt-cgb**, bennett-runtimes | Pinned export audit lacks __bennett_dev_console_write; only memory/allocation exports exist. Implement the musl-to-effect console bridge and lifted console state (§11.2/11.3), preserving R tape/reset semantics. | medium, 480 min |
| 3 | **bp-ivy**, bennett-plugin | Both opt libraries reject --bennett-tier and --bennett-entry. Supply the full whole-program R integration, effects, argv launch and artefacts; respect BD-B-ISA7. Later effect lowering is not yet measured. | large, 960 min |
| 4 | **bennett-driver-cyg**, bennett-driver | Frozen driver exits 0, prints its version and creates no outputs. Implement §19.1/19.2 plus an auditable materialised §17.7 link and argv/native adapters. | large, 960 min |
| 5 | **b-isa-spec-221**, bennett-isa-spec (existing bead augmented, labels consumer/isa) | Required R fault C / mandatory fault 1 cannot be expressed on pinned ISA 0.8. Adopt/implement the published 0.9 semantics with explicit pin migration and producer/executor verification; no generic-trap equivalence claim. | medium, 480 min |
| 6 | **b-isa-spec-04d**, bennett-isa-spec (reference consumer) | Accepted release CLI rejects --replay-tape. Expose genuine tape-consuming replay and reject a tampered tape; equivalent adapters are welcome. | medium, 240 min |
| 7 | **bennett-vm-u8m**, bennett-vm | Pinned detached VM rejects --replay-tape. Expose the same replay capability and corruption check. | medium, 240 min |

[blockers.json](blockers.json) is in this order and includes the determining spec
sentences, exact commands, evidence paths and completion criteria copied into the
owning beads. These are feature/integration prerequisites, not newly invented
semantic defects. The proposed replay flag and comparison-manifest filenames are
consumer glue, not normative ISA option names. brt-1ft/brt-cgb retain the wider
runtime scope already tracked by brt-q1k/brt-zct. Conformance bead:
**bennett-conformance-7bx**, kept blocked/open for the complete rung.

## What works today

Both plugin libraries, the driver and the clean detached VM build from pinned
sources. Both executors admit the published ISA-0.8 `classical.bisa @effects`
control, run its five entropy/console/clock/block effects, and reverse fully to
entry state (memory and device equality, empty console and effect tape).
Console bytes are 0b1621; the control's retained forward tape is unchanged in the
reverse report. This establishes that control's device/reset route, not musl
hello-world. The reference interpreter is the accepted **release** binary;
its SHA-256 and the VM binary hash are in the report.

The eight rung-3 glue tests pass, including rejection of missing/unsafe native
links, corrupted reset flags, ignored/tampered replay tapes, nonzero command exits
and timeouts. Rung-2's five checks pass against real executor outputs; a structural
AST comparison confirms **every rung-2 function remains identical**, with only
four helpers moved verbatim into common support. The checker crate's two tests,
113 clause tests, 23 suite tests and all 16 neutral suite cases pass. Tests and
controls never make the red corpus green.

## Reproducibility and observed gotchas

Source/build pins are listed in [README.md](README.md) and [pins.json](pins.json).
Compiler binaries are the recorded LLVM 18.1.3 and LLVM 23.1.5 toolchains. The
runtime and arith archives are pinned to their published bitcode; the live arith
publication was being updated concurrently and differed from the plugin's SHA
pin during the first invocation. The gate was corrected to extract pinned
publications; that initial arith diagnostic is superseded, not filed as an
upstream defect. The corrected discovery run was
`.tmp/rung-3/20261101T125019.671846Z/` (UNFILED before issues were filed); this
recorded rerun reproduces it with the bead mapping and final native-link checks.

The suite test runner initially failed because BENNETT_ISA_BINARY was unset;
rerunning it with the accepted release binary passed all 23 tests. Raw evidence
is retained locally; compact durable evidence is committed here. The test-first
red and final green glue logs are included.

The driver lacked .beads. `bd init --skip-hooks --skip-agents --non-interactive`
configured an origin Dolt remote and auto-committed a root .gitignore addition.
The remote was removed, sync.remote cleared, the root file restored, and bootstrap
commits consolidated so the final driver commit contains **only .beads files**.
No Dolt remote sync or Git push was performed. Exports use the repository working
directory (`env -C`/subprocess cwd): `bd -C` selects the database but does not
relocate relative `export -o` paths. All final exports were regenerated there.
Concurrent VM work had already checkpointed its new bead; a separate beads-only
commit records this task's frozen evidence. That later VM source was not tested.

## Exact determining commands

```sh
/usr/lib/llvm-23/bin/clang --target=x86_64-unknown-linux-musl --sysroot=/home/tobias/Projects/bennett-workspace/bennett-runtimes/dist/llvm23/musl-sysroot -nostdinc -isystem /home/tobias/Projects/bennett-workspace/bennett-runtimes/dist/llvm23/musl-sysroot/include -O2 -flto -emit-llvm -c /home/tobias/Projects/bennett-workspace/bennett-conformance/ladder/rung-3/fixtures/hello.c -o /home/tobias/Projects/bennett-workspace/bennett-conformance/.tmp/rung-3/20261101T125405.461872Z/23/input.bc
```

```sh
/usr/lib/llvm-18/bin/clang --target=x86_64-unknown-linux-musl --sysroot=/home/tobias/Projects/bennett-workspace/bennett-runtimes/dist/llvm18/musl-sysroot -nostdinc -isystem /home/tobias/Projects/bennett-workspace/bennett-runtimes/dist/llvm18/musl-sysroot/include -O2 -flto -emit-llvm -c /home/tobias/Projects/bennett-workspace/bennett-conformance/ladder/rung-3/fixtures/hello.c -o /home/tobias/Projects/bennett-workspace/bennett-conformance/.tmp/rung-3/20261101T125405.461872Z/18/input.bc
```

```sh
/usr/lib/llvm-18/bin/llvm-nm --defined-only /home/tobias/Projects/bennett-workspace/bennett-conformance/.tmp/rung3-bennett-runtimes-6808622d560b2e301c635c205eecde3d932b6df5/dist/llvm18/bennett-runtimes.bc
```

```sh
/usr/lib/llvm-23/bin/opt -load-pass-plugin=/home/tobias/Projects/bennett-workspace/bennett-conformance/.tmp/rung3-bennett-plugin-00a5537f92af6b8b5d13e4422f3316e1e41005da/build/libbennett-plugin-23.so -passes=bennett -bennett-dialect=0.2.10 -bennett-isa=0.8 --bennett-tier=reversible --bennett-entry=main -bennett-budget-default=10000000,1048576,65536,256,65536 -bennett-out=/home/tobias/Projects/bennett-workspace/bennett-conformance/.tmp/rung-3/20261101T125405.461872Z/23/probe-out -disable-output /home/tobias/Projects/bennett-workspace/bennett-conformance/ladder/rung-3/fixtures/effect-console-r.ll
```

```sh
/usr/lib/llvm-18/bin/opt -load-pass-plugin=/home/tobias/Projects/bennett-workspace/bennett-conformance/.tmp/rung3-bennett-plugin-00a5537f92af6b8b5d13e4422f3316e1e41005da/build/libbennett-plugin-18.so -passes=bennett -bennett-dialect=0.2.10 -bennett-isa=0.8 --bennett-tier=reversible --bennett-entry=main -bennett-budget-default=10000000,1048576,65536,256,65536 -bennett-out=/home/tobias/Projects/bennett-workspace/bennett-conformance/.tmp/rung-3/20261101T125405.461872Z/18/probe-out -disable-output /home/tobias/Projects/bennett-workspace/bennett-conformance/ladder/rung-3/fixtures/effect-console-r.ll
```

```sh
/home/tobias/Projects/bennett-workspace/bennett-conformance/.tmp/rung3-bennett-driver-306b6e90df85a6b0f59e1e57b04ff7fbcae37ce5/target/debug/bennett-driver --bennett-tier=reversible --bennett-entry=main -bennett-dialect=0.2.10 -bennett-isa=0.8 -bennett-out=/home/tobias/Projects/bennett-workspace/bennett-conformance/.tmp/rung-3/20261101T125405.461872Z/18/driver-probe /home/tobias/Projects/bennett-workspace/bennett-conformance/ladder/rung-3/fixtures/effect-console-r.ll
```

```sh
/home/tobias/Projects/bennett-workspace/bennett-isa-spec-review/reference-interpreter/target/release/bennett-isa-ref run-cc /home/tobias/Projects/bennett-workspace/bennett-conformance/ladder/rung-3/fixtures/executor-control.bisa effects --entropy 0b1621 --replay-tape /home/tobias/Projects/bennett-workspace/bennett-conformance/.tmp/rung-3/20261101T125405.461872Z/18/reference-control-tape.json
```

```sh
/home/tobias/Projects/bennett-workspace/bennett-conformance/.tmp/vm-9ac6d3e11b5b62f4e5f9a4c7294622a00db89bcd/target/debug/bennett-vm run-cc /home/tobias/Projects/bennett-workspace/bennett-conformance/ladder/rung-3/fixtures/executor-control.bisa effects --entropy 0b1621 --replay-tape /home/tobias/Projects/bennett-workspace/bennett-conformance/.tmp/rung-3/20261101T125405.461872Z/18/VM-control-tape.json
```

The LLVM 23/runtime and executor counterparts, contract `git show`, archive,
configure/build, control forward/reverse commands and their complete output are
all recorded in report.json. Run `ladder/rung-3/gate.sh` to reproduce these frozen
defaults; no revision flag is necessary. Pass --sysroot18/23 only for a real
Bennett musl publication, and explicitly override revisions when testing fixes.

## Sibling bead commits (no source changes by this task)

| Repository | HEAD when recorded |
|---|---|
| bennett-runtimes | `c49dc6ad3f0c7233b37ed433174bc21656dd65e9` |
| bennett-plugin | `600143a9aaf81dfd173aa7a0f981c8c1b7967f20` |
| bennett-driver | `8c6d0a8cc376ba997febcef9434bee08a79d57e8` |
| bennett-isa-spec | `6c3c1a44bf1c63dc84b7df1980b0f9e5d8aec94b` |
| bennett-vm | `1957cb1e71a9e70128fbd7eb40447c83efcb187e` |

This repository's implementation commit is reported at handoff rather than
self-referenced in the committed file. Remaining unrelated worktree changes are
left for their owners.
