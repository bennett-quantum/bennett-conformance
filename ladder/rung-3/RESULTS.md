# Rung 3 recorded run — 2026-11-03T09:33:32.238236+00:00

**The recorded gate is red on both LLVM majors.** Ordinary musl `hello.c`
compiles, but the first required failure is **`arith publication differs from
plugin pin`**, before `llvm-link` runs. No hello-world native comparison,
production object, executor equality, reset or required replay was reached.
The missing-runtime error described as the first failure in the job brief is
actually a separate, later driver diagnostic. The raw report and full log agree.

The run uses the origin-main implementation pins in [pins.json](pins.json):
plugin `0ac6180`, VM `c26a4fd`, driver `54d32fa`, runtimes `248e468`, arith
`5e9b450`. Dialect remains 0.2.10 at `023f5a7`; ISA remains 0.8. No pin was changed
by this analysis. Original sysroot paths and header hashes are in the manifest;
both recorded front-end commands passed using the selected musl headers.

Raw evidence: `.tmp/rung-3/20261103T093332.236979Z/`. Durable byte-identical
[report](evidence/20261103T093332.236979Z/report.json),
[table](evidence/20261103T093332.236979Z/table.md) and
[orchestrator log](evidence/20261103T093332.236979Z/gate.log) retain their original
UNFILED entries. The [SHA-256 inventory](evidence/20261103T093332.236979Z/raw-evidence-sha256.json)
covers all 112 raw evidence files. Full log source:
`bennett/.tmp/orchestrator/s4/heavy/rung3.log.full` in the workspace.

## Original measured table

| Stage | LLVM 18 | LLVM 23 |
|---|---|---|
| fixtures | PASS | PASS |
| plugin build | PASS | PASS |
| VM build | PASS | PASS |
| driver build | PASS | PASS |
| musl C front end | PASS | PASS |
| runtime bitcode link | FAIL · UNFILED | FAIL · UNFILED |
| post-link LTO | BLOCKED · UNFILED | BLOCKED · UNFILED |
| reversible production | BLOCKED · UNFILED | BLOCKED · UNFILED |
| launch artefacts | BLOCKED · UNFILED | BLOCKED · UNFILED |
| native comparison | BLOCKED · UNFILED | BLOCKED · UNFILED |
| reference admission | BLOCKED · UNFILED | BLOCKED · UNFILED |
| reference forward = native | BLOCKED · UNFILED | BLOCKED · UNFILED |
| reference reset | BLOCKED · UNFILED | BLOCKED · UNFILED |
| reference tape replay | BLOCKED · UNFILED | BLOCKED · UNFILED |
| VM admission | BLOCKED · UNFILED | BLOCKED · UNFILED |
| VM forward = native | BLOCKED · UNFILED | BLOCKED · UNFILED |
| VM reset | BLOCKED · UNFILED | BLOCKED · UNFILED |
| VM tape replay | BLOCKED · UNFILED | BLOCKED · UNFILED |
| runtime libc/console audit (diagnostic) | FAIL · UNFILED | FAIL · UNFILED |
| R-mode probe (diagnostic) | FAIL · UNFILED | FAIL · UNFILED |
| driver probe (diagnostic) | FAIL · UNFILED | FAIL · UNFILED |
| ISA-0.8 R contract (prerequisite) | FAIL · b-isa-spec-221 | FAIL · b-isa-spec-221 |
| reference device/reset control (diagnostic) | PASS | PASS |
| reference replay API (diagnostic) | PASS | PASS |
| VM device/reset control (diagnostic) | PASS | PASS |
| VM replay API (diagnostic) | PASS | PASS |

Each major has 9 PASS, 5 FAIL and 12 BLOCKED rows. Diagnostics are separate
controls; they cannot satisfy required hello-world rows. In particular:

- The runtime libc/console audit fails the same arithmetic hash check before
  `llvm-nm`; it supplies no current missing-symbol verdict.
- Both R-mode probes accept the options and exit 3 with the real
  `blocked:BD-B-ISA7` diagnostic. The old unknown-option frontier has advanced.
- Both reference and VM device/reset controls and saved-tape API probes PASS.
  The API probe only requires that the supplied-tape command succeeds. Required
  hello-world replay checks, including rejection of a corrupted tape, remain
  BLOCKED. This is not full replay acceptance.

## First blockers in pipeline order

| Order | Bead / owner | Determination and completion criterion |
|---|---|---|
| 1 | **bp-mq6**, bennett-plugin, open | Plugin arith pins disagree with archived origin-main arith on both majors. Verify and explicitly re-pin the required publication, compiled plugin pins and materialised L/CC closure; then refresh conformance pins coherently. Keep the hash checks. |
| 2 | **bp-is0**, bennett-plugin, open | The isolated R-mode probe reaches the pinned ISA0.8 typed-fault prerequisite. Implement and verify the published typed-fault producer contract and coordinate explicit migration. No silent ISA upgrade or generic-trap substitution. |
| 3 | **bennett-conformance-igc**, conformance, fixed/closed | Archived driver probe omitted major and publication paths; both probes defaulted to LLVM23 and a nonexistent sibling runtime. The gate now supplies all frozen paths and the selected major. Verification covers glue; a full rerun is still owed. |
| 4 | **b-isa-spec-fuk**, bennett-isa-spec, open | Coordinate producer/reference/executor and contract-pin migration for typed R faults. Historical b-isa-spec-221 is closed because ISA0.9 resolves its wording; it is not a remaining ISA0.9 spec defect. The pinned ISA0.8 consumer prerequisite remains red. |

The [blocker registry](blockers.json) lists these current frontiers first,
including exact diagnostics, determining clauses and completion criteria.
Historical missing-header/export, unknown-option, version-only-driver and
replay-option patterns remain for regression recognition, after current entries;
this run does not re-establish those failures. Evidence updates for the three
existing upstream beads are requested in `.tmp/sol-rung3/cross-repo-beads.md`;
no sibling database was changed. Sibling statuses are from their passive exports:
`bd show` cannot open their embedded-Dolt LOCK on this read-only filesystem, so
the orchestrator must confirm live status when applying the updates.
**bennett-conformance-7bx remains blocked.**

After confirming that the beads exist, recorded errors were classified with
`Gate.stage` and the table/report emitted with `Gate.finish`, using dependencies
from the original BLOCKED errors. This executed no corpus command. The separately
labelled [classified report](evidence/20261103T093332.236979Z/classified/report.json)
and [classified table](evidence/20261103T093332.236979Z/classified/table.md) change
only bead associations and add classification metadata; every status, error,
pin, argv and exit is retained, and `finish` still returns 1.
[Recording output](evidence/20261103T093332.236979Z/recording.log) is separate from
the original gate log. The unchanged raw UNFILED report remains the run evidence.

## Arithmetic publication determination

The archived plugin SPEC_VERSIONS.toml pins arith commit `0dea42d`; the tested
origin-main arithmetic commit is `5e9b450`. Both actual bitcode hashes agree with
the latter publication’s SHA256SUMS and differ from the plugin’s expected hashes:

| Major | Plugin expected SHA-256 | Selected arith SHA-256 |
|---|---|---|
| 18 | `8d99956e8741803f2115e48fa28c6c71795fb0041e992f8835851b71dd4b6fc1` | `a027f16991c06f39eb1305f28da0b1504651791bdd3533888171aa54e704f7ff` |
| 23 | `59563a3a10949b1ea48ca8b79027310ac9f9f7e28c85a30e70a013ced04a6793` | `8216b27231c36d750c326bbaef384011a1bdf7f7dbb717b8b8f5167551163f79` |

[Publication inspection](evidence/20261103T093332.236979Z/publication-pins.json)
records paths, full commits and hashes. This is a reproducible consumer/producer
pin disagreement, not evidence of corrupt arith or runtime bitcode. Selecting
old arith or dropping verification would hide the origin-main integration
failure; neither was done. There is no failed `runtime-link` command in the
report because `runtime_files` rejects the mismatch first.

## Gate-owned driver fix

Read-only inspection of `bennett-driver` at `54d32fa:src/main.rs` confirms its
published options `--bennett-llvm-major`, `--bennett-plugin`, `--bennett-runtimes`
and `--bennett-arith`. Its default major is 23 and publication paths are resolved
relative to the build checkout’s parent. An archived driver under conformance’s
`.tmp/rung3-bennett-driver-<sha>` consequently searches `.tmp/bennett-runtimes`.
Dialect §19.1/§19.8 requires same-major inputs; it prescribes no sibling directory
layout. Explicit publication selection is the gate’s responsibility.

The determining driver error, on both recorded probes, begins:

```text
BD-E-INPUT: hash /home/tobias/Projects/bennett-workspace/bennett-conformance/.tmp/bennett-runtimes/dist/llvm23/bennett-runtimes.bc: sha256sum: /home/tobias/Projects/bennett-workspace/bennett-conformance/.tmp/bennett-runtimes/dist/llvm23/bennett-runtimes.bc: No such file or directory
```

`gate.py` now shares explicit major/plugin/runtime/arith arguments between the
probe and materialised comparator. The probe uses `runtime_files`, retaining
archived defaults, explicit `--runtimes18/23` overrides and publication hashing,
and depends on plugin build. The comparator already selected major/runtime/arith
but also needed the explicit archived plugin. There is no driver-default defect
filed for this layout. On unchanged incompatible pins, the corrected probe will
fail the arithmetic check before calling the driver; that failure stays red.

## Verification and rerun

Red-first driver tests failed with 10 assertions on the old gate; after the fix
all 11 checks passed. A later red-first blocker-mapping check failed four cases;
after mapping existing beads the complete focused suite passes **12 tests**:

```sh
env -u DISPLAY -u WAYLAND_DISPLAY -u DBUS_SESSION_BUS_ADDRESS -u BROWSER \
  TMPDIR=/home/tobias/Projects/bennett-workspace/bennett-conformance/.tmp/sol-rung3/tmp \
  CARGO_TARGET_DIR=/home/tobias/Projects/bennett-workspace/bennett-conformance/.tmp/sol-rung3/target \
  timeout 60s python3 -B /home/tobias/Projects/bennett-workspace/bennett-conformance/ladder/rung-3/check_gate.py
```

Evidence: [driver tests red](evidence/20261103T093332.236979Z/tests-red.txt),
[mapping test red](evidence/20261103T093332.236979Z/mapping-red.txt),
[final tests green](evidence/20261103T093332.236979Z/tests-green.txt).
Checks cover both majors, explicit publication defaults/overrides, hash mismatch
rejection, unchanged driver failures, precise bead matching, UNFILED propagation,
and the existing fixture/native-link/reset/replay checks. Synthetic results are
not corpus acceptance.

No full rung or repository gate was rerun: R3 retains the heavy slot, and this
sandbox cannot create the sibling VM worktree. Exact orchestrator rerun:

```sh
env -u DISPLAY -u WAYLAND_DISPLAY -u DBUS_SESSION_BUS_ADDRESS -u BROWSER \
  TMPDIR=/home/tobias/Projects/bennett-workspace/bennett-conformance/.tmp/sol-rung3/tmp \
  CARGO_TARGET_DIR=/home/tobias/Projects/bennett-workspace/bennett-conformance/.tmp/sol-rung3/target \
  CARGO_BUILD_JOBS=4 \
  timeout 7200s /home/tobias/Projects/bennett-workspace/bennett-conformance/ladder/rung-3/gate.sh --jobs 4
```

The existing wrapper resets TMPDIR to conformance’s `.tmp`; the original report
records that effective environment. The rerun uses unchanged pins and should
still expose the arithmetic blocker until owners supply a verified compatible
publication. No rerun result or later-stage success is claimed here.

This is the whole-device entry profile: reversible main/argv/status and console
tape expectations remain exact. The comparator restrictions cover the
materialised device closure. Mixed fat objects keep an ordinary native host
binary and marked device closures; native main, host-only libc and host I/O
need no ISA region, certificate record or device tape. No new launch ABI is
invented, and no whole-binary lowering requirement is introduced (bennett-4ht).

Previous measured results remain in
[evidence/20261101T125405.461872Z](evidence/20261101T125405.461872Z/table.md).

## Gate preparation — 2026-11-07T10:26:41Z


**This is unit-tested gate preparation, not a new end-to-end rung pass.**
The full rung/repository gate was deliberately not run: the orchestrator owns
that heavy run. Implementation commit pins are unchanged, so the recorded
arithmetic incompatibility and selected ISA0.8 R prerequisite remain red until
coherent upstream adoption. Archived evidence above is unchanged.
**bennett-conformance-7bx remains open.**

The corpus retains hello and adds ordinary `printf_fmt.c`: `%s %d %u %x %c`,
multiple arguments, width/precision, negative and extreme 32-bit values, zero
precision, and a final buffered line without LF or explicit flush. Its 213 ASCII
bytes and status 0 are a literal hand-derived table in
`fixtures/printf_fmt.expect.json`. Source/oracle hashes are pinned. Every required
pipeline/native/executor row covers both workloads and LLVM majors. Hello row
names remain; formatted rows use a `printf_fmt:` prefix. Each executor has separate
reset, saved-tape and corrupt-tape rows. A failed formatted row cannot be hidden
by a passing hello path.

`pins.json` adds version/path fields only. Selected versions drive options and
clause graphs. The R prerequisite compares the pinned producer declarations with
the pinned dialect publication's full compatibility unit and ISA target. The
known ISA0.8 publication still gives BD-B-ISA7. Matching ISA0.9 producer/publication
pairs pass the adapter; unknown/mismatched pairs fail loudly. Synthetic future
version cases do not establish approval of any draft or producer. The existing
igc explicit major/plugin/runtime/arith fix is retained and tested on both majors;
historical missing-directory diagnostics also classify from isolated checkouts.

The native audit accepts direct exact-input links or the documented multi-stage
bitcode/linker-script adapter. It checks device/runtime/arith hashes and replays
successful ordered LLVM transformations from those exact leaves. Additional
inputs are restricted to the published generated layout constant and same-major
pinned native-support bytes. Scripts cannot introduce file/library inputs;
quoted archive selectors, library-search, comma/response injection and opaque
passes fail loudly. Actual ELF shared-library/interpreter/undefined-import checks
remain. The explicit driver work manifest is `out/native-work/manifest.json`.
Unsupported adapter transforms fail loudly; opaque merged filenames are not proof.

Red-first output: `.tmp/r3-gate/red.txt` (initial 8 tests: 7 errors, 1 failure).
Additional negative-receipt/blocker-path regressions are captured in that job
directory. Focused verification from the assigned checkout:

```sh
env -u DISPLAY -u WAYLAND_DISPLAY -u DBUS_SESSION_BUS_ADDRESS -u BROWSER \
  TMPDIR=/home/tobias/Projects/bennett-workspace/bennett-conformance-job-r3/.tmp/r3-gate/tmp \
  timeout 60s python3 -B -m unittest discover -s ladder/rung-3/tests -v
env -u DISPLAY -u WAYLAND_DISPLAY -u DBUS_SESSION_BUS_ADDRESS -u BROWSER \
  TMPDIR=/home/tobias/Projects/bennett-workspace/bennett-conformance-job-r3/.tmp/r3-gate/tmp \
  timeout 60s python3 -B ladder/rung-3/check_gate.py
```

Results: **15 new tests PASS**, including actual tiny LLVM receipt rebuild/static
link/exit-value checks on both majors (no skips), and **12 existing tests PASS**.
The independent tiny closure returns 7+11=18 after a spoofed intermediate is rebuilt;
it is not either musl workload. Negatives cover host libc in final/intermediate
inputs, dropped runtime, substituted hashes/bytes/paths, wrong-major/untracked
native support, script inputs and opaque options. Executor tests cover missing
unterminated-tail output and supplied/corrupted tapes on every workload/major/
executor. Logs: `.tmp/r3-gate/tests-green.txt` and `legacy-green.txt`.

Independent fixture smoke used each major's clang with
`-O2 -Wall -Wextra -Werror <source> -o <job-output>` and compared stdout/status
against the literal tables: hello 13 bytes, formatted 213 bytes, status 0 on both.
It deliberately used **host libc** solely to validate fixture/oracle consistency;
it did not exercise the materialised native comparison, musl runtime, producer,
reference or VM. Exact commands/results: `.tmp/r3-gate/fixture-smoke.txt`.

Full formatted-musl R production, native agreement, reset and strict replay on
both executors remain unmeasured. The orchestrator must adopt reviewed coherent
publications and run the heavy rung gate before claiming acceptance.


## Pushed-main promotion run — 2026-11-09T10:30:35Z

**Rung 3 remains BLOCKED.** The full, unchanged rung gate executed on both LLVM
majors. Each major has **12 PASS / 4 FAIL / 28 BLOCKED** rows. Both ordinary musl
front ends pass for hello (13-byte oracle) and formatted printf (213-byte oracle).
The first required failure is still **`arith publication differs from plugin pin`**.
No materialised native comparison, musl ISA object admission, executor output,
reset, saved-tape replay or corrupt-tape rejection ran. Passing isolated device
controls and replay API probes does not satisfy either workload's required rows.
The runtimes variadic checker re-land is pinned; this earlier publication boundary
prevents measuring it in the full rung. No input, oracle or acceptance condition
was reduced. **bennett-conformance-7bx stays open; the orchestrator owns acceptance.**

The implementation checkpoint is `1c0b514`, backed up to `wip/r3-promote`.
The selected contract is **dialect 0.3-r1 / ISA 0.9**, with mapping 0.3,
normaliser 0.2.8 and saved-tape-v1. This is the accepted classical R profile,
not whole-0.3 grading. Current pushed mains were queried using
`git -C <sibling> ls-remote origin main` immediately before execution and again
after it; **no pin moved**. Both reference-source keys identify the same ISA main.
The pending plugin/arith pushes had not published newer mains at the final
observation. Observations: [before](evidence/20261109T102407.573477Z/origin-before-run.json),
[after](evidence/20261109T102407.573477Z/origin-after-run.json).

| Input | Pushed main |
|---|---|
| bennett-plugin | `abe5557977e9e9a38d8922aff69cbfc638995798` |
| bennett-vm | `83b5f7f0248fe1c36ebfe3acd7fb187ed301affa` |
| bennett-runtimes | `40f243e2a2128eb61d0d2ab4786aa082a8e2c90f` |
| bennett-driver | `8b1b4a1814ce3c99f4357d2e11f048b1ff2a090b` |
| bennett-arith | `cd245bcb07ad38da629d9b47f642e46e3c25cae9` |
| bennett-isa-spec | `695cb4b7c330742820c6b315337a260e56de8bce` |
| bennett-isa-spec-review | `695cb4b7c330742820c6b315337a260e56de8bce` |
| bennett-dialect-spec | `830bd7f2a328556af983f2c3aa4c75ef9eb29b65` |

The run builds the reference from ISA `695cb4b`, rather than trusting an existing
binary. Its SHA-256 is `5893350323915fea5b7b36c97565e8dc2f2e3b08b63bc549c718fa626487b232`.
Both native headers/sysroots are from the archived runtime pin. Plugin CMake's
published pin resolver receives a link used to read to the main checkout's verified
gate cache; selected runtime/arith publications remain separately archived and
hash checked. VM is the gate's clean detached dependency worktree under this job.

### Execution receipt

Exact command (working directory is the assigned worktree):

```sh
/usr/bin/python3 -B /home/tobias/Projects/bennett-workspace/bennett-conformance-job-r3-promote/.tmp/r3-promote/run.py
```

Detached unit: `bennett-conf-r3-promote.service`, invocation
`587b7e07ed014808aa210b393b1a9c77`, launched with `systemd-run --user`,
`--setenv=PATH="$PATH" --setenv=HOME="$HOME"`, desktop variables unset and
TMPDIR/CARGO_TARGET_DIR inside `.tmp/r3-promote/`. The harness calls the actual
unabridged `Gate.main`; acceptance methods and dependencies are unchanged.
It records each command through wait4 and confines temporary sources/builds.
The job-local [guard table](evidence/20261109T102407.573477Z/hang-guards.json) cites named Spiral receipts
at a margin of at least 10x. Conformance has no durable class table: follow-up
**bennett-conformance-ji0** owns that infrastructure gap. Guard trips are UNKNOWN
exit 124; **none tripped**. Eight build workers were derived from nproc/MemAvailable
with headroom; launch admission had about 52 GiB available and load 17 < 64/2.

Result: **exit 1, 160.084968 s wall,
590.707025 s unit CPU, 5403381760 bytes peak memory**.
All **60 executed gate commands** have argv, wall, CPU, exit and peak-RSS receipts;
all subprocesses exit 0. The gate exits 1 because its publication assertions fail,
not because a subprocess failed. The unexecuted rows have no fabricated command
receipt. No full repository push gate or sibling gate was run.
[Unit receipt](evidence/20261109T102407.573477Z/receipt.json), [command receipts](evidence/20261109T102407.573477Z/command-receipts.json),
[unaltered raw report](evidence/20261109T102407.573477Z/report.json),
[complete per-row table](evidence/20261109T102407.573477Z/table.md),
[raw evidence hash inventory](evidence/20261109T102407.573477Z/raw-evidence-sha256.json).
Raw sources, command outputs and gate log remain locally under
`.tmp/r3-promote/runs/20261109T102407.573477Z/` and `.tmp/r3-promote/gate.log`.
Only structured evidence is published; no log/transcript/progress/scratch file
is included in the backup branch.

### Complete executed table

The report's historical recognizer labels the mismatch **bp-mq6**. Its current
passive export marks that bead **closed** for the earlier publication. A **new
plugin follow-up or reopen** is requested in
`.tmp/r3-promote/cross-repo-beads.md#plugin-arith-main`; every FAIL and dependent
BLOCKED row below belongs to that request, not an assertion that bp-mq6 is open.
[Ownership disposition](evidence/20261109T102407.573477Z/ownership.md).

| Stage | LLVM 18 | LLVM 23 |
|---|---|---|
| fixtures | PASS | PASS |
| plugin build | PASS | PASS |
| VM build | PASS | PASS |
| driver build | PASS | PASS |
| R contract (prerequisite) | PASS | PASS |
| musl C front end | PASS | PASS |
| runtime bitcode link | FAIL · bp-mq6 | FAIL · bp-mq6 |
| post-link LTO | BLOCKED · bp-mq6 | BLOCKED · bp-mq6 |
| reversible production | BLOCKED · bp-mq6 | BLOCKED · bp-mq6 |
| launch artefacts | BLOCKED · bp-mq6 | BLOCKED · bp-mq6 |
| native comparison | BLOCKED · bp-mq6 | BLOCKED · bp-mq6 |
| reference admission | BLOCKED · bp-mq6 | BLOCKED · bp-mq6 |
| reference forward = native | BLOCKED · bp-mq6 | BLOCKED · bp-mq6 |
| reference reset | BLOCKED · bp-mq6 | BLOCKED · bp-mq6 |
| reference tape replay | BLOCKED · bp-mq6 | BLOCKED · bp-mq6 |
| reference corrupt-tape rejection | BLOCKED · bp-mq6 | BLOCKED · bp-mq6 |
| VM admission | BLOCKED · bp-mq6 | BLOCKED · bp-mq6 |
| VM forward = native | BLOCKED · bp-mq6 | BLOCKED · bp-mq6 |
| VM reset | BLOCKED · bp-mq6 | BLOCKED · bp-mq6 |
| VM tape replay | BLOCKED · bp-mq6 | BLOCKED · bp-mq6 |
| VM corrupt-tape rejection | BLOCKED · bp-mq6 | BLOCKED · bp-mq6 |
| printf_fmt: musl C front end | PASS | PASS |
| printf_fmt: runtime bitcode link | FAIL · bp-mq6 | FAIL · bp-mq6 |
| printf_fmt: post-link LTO | BLOCKED · bp-mq6 | BLOCKED · bp-mq6 |
| printf_fmt: reversible production | BLOCKED · bp-mq6 | BLOCKED · bp-mq6 |
| printf_fmt: launch artefacts | BLOCKED · bp-mq6 | BLOCKED · bp-mq6 |
| printf_fmt: native comparison | BLOCKED · bp-mq6 | BLOCKED · bp-mq6 |
| printf_fmt: reference admission | BLOCKED · bp-mq6 | BLOCKED · bp-mq6 |
| printf_fmt: reference forward = native | BLOCKED · bp-mq6 | BLOCKED · bp-mq6 |
| printf_fmt: reference reset | BLOCKED · bp-mq6 | BLOCKED · bp-mq6 |
| printf_fmt: reference tape replay | BLOCKED · bp-mq6 | BLOCKED · bp-mq6 |
| printf_fmt: reference corrupt-tape rejection | BLOCKED · bp-mq6 | BLOCKED · bp-mq6 |
| printf_fmt: VM admission | BLOCKED · bp-mq6 | BLOCKED · bp-mq6 |
| printf_fmt: VM forward = native | BLOCKED · bp-mq6 | BLOCKED · bp-mq6 |
| printf_fmt: VM reset | BLOCKED · bp-mq6 | BLOCKED · bp-mq6 |
| printf_fmt: VM tape replay | BLOCKED · bp-mq6 | BLOCKED · bp-mq6 |
| printf_fmt: VM corrupt-tape rejection | BLOCKED · bp-mq6 | BLOCKED · bp-mq6 |
| runtime libc/console audit (diagnostic) | FAIL · bp-mq6 | FAIL · bp-mq6 |
| R-mode probe (diagnostic) | PASS | PASS |
| driver probe (diagnostic) | FAIL · bp-mq6 | FAIL · bp-mq6 |
| reference device/reset control (diagnostic) | PASS | PASS |
| reference replay API (diagnostic) | PASS | PASS |
| VM device/reset control (diagnostic) | PASS | PASS |
| VM replay API (diagnostic) | PASS | PASS |

### Determining failure and owner

For **each** LLVM major, the failing rows are `runtime bitcode link`,
`printf_fmt: runtime bitcode link`, `runtime libc/console audit (diagnostic)`
and `driver probe (diagnostic)`. Each exact output is:

```text
arith publication differs from plugin pin
```

Determining sentence: pushed plugin `abe5557` expects arithmetic `5e9b450`,
while pushed arithmetic `cd245bc` publishes different bytes on both majors.
This is the same-major publication-identity boundary of accepted dialect
0.3-r1 §§19.1/19.9. The determining execution command is the full gate command
above; `runtime_files` asserts before llvm-link/nm/driver, so there is **no failed
runtime-link command**, no current missing-symbol verdict and no current driver
failure to assign to another owner. [Each failing row, exact command/output and
completion condition](evidence/20261109T102407.573477Z/failures.json).

| Major | Producer expected SHA-256 | Selected publication SHA-256 |
|---|---|---|
| 18 | `a027f16991c06f39eb1305f28da0b1504651791bdd3533888171aa54e704f7ff` | `053d820087cdb91f851d2505c6eb37d117ec3041a614f1c53d23d47a93423d92` |
| 23 | `8216b27231c36d750c326bbaef384011a1bdf7f7dbb717b8b8f5167551163f79` | `b08ca7c535aa4475452b7a82e4d82847c667fef8dc019affb914a327ade590c5` |

The selected hashes match their pinned arithmetic SHA256SUMS. Exact independent
inspection argv/output: [publication determination](evidence/20261109T102407.573477Z/publication-pins.json)
and command receipts `publication-hashes` / `producer-pins`. Owning repository:
**bennett-plugin**; follow-up/reopen request linked above. Fixed means verify and
explicitly re-pin the required pushed publication and compiled digests, verify
the materialised L/CC closure, then re-pin current pushed mains and execute this
entire unchanged matrix. Selecting old arithmetic or removing the check would
hide the integration failure. No sibling repository or bead database was edited.

The selected R prerequisite and isolated R-mode probes now PASS. Historical
ISA0.8 typed-fault recognizers remain documented in blockers.json, without being
reported as current failures or implying general migration closure.

### Gate interface changes and focused checks

[Exact patch listing every changed glue line](evidence/20261109T102407.573477Z/gate-glue.patch):
`contract.py` adds the §1.7 profile branch and validates the producer's separate
`profiles.r_contract` identity, published acceptance paragraph, ISA, map,
normaliser and replay identity. `gate.py` changes only the two fixture-graph
selection lines to select the rung-owned graph for the new profile. The graph
was generated by `clauses/clausegraph.py build --spec DLT --version 0.3-r1
--file .tmp/r3-promote/dialect.md --xspec clauses/ISA-0.9.json
--out-dir ladder/rung-3/clauses`; its source hash matches the pinned publication.
This graph lists clauses of the full document; it grades no additional scope.

Red-first profile regression is `.tmp/r3-promote/profile-red.txt`. Exact focused
commands executed through the local guard/receipt harness:

```sh
python3 -B -m unittest discover -s ladder/rung-3/tests -v
python3 -B ladder/rung-3/check_gate.py
python3 -B scripts/quality_ratchet.py
```

Results: **24 focused tests PASS** (including real tiny LLVM18/23 receipt/native
adapter checks), **12 legacy checks PASS**, **quality ratchet PASS**. Those
small fixtures are glue tests, not musl acceptance. The only baseline rows changed
are `ladder/rung-3/contract.py` and `ladder/rung-3/gate.py`; all other baseline
values and litter allowances are unchanged. [Scoped diff receipt](evidence/20261109T102407.573477Z/quality-diff.patch).
Earlier RESULTS sections and evidence directories remain unchanged.


## Final archived-interface verification — 2026-11-09T10:37:31Z

The final code checkpoint **`4c9d9ce`** (backed up to `wip/r3-promote`) was
executed in a second detached unit after moving the archived plugin's published
pin-cache setup from the job harness into `Gate.plugin`. This is a new code tree;
the earlier tree had no PASS receipt. The exact same **44 rows × both majors**
produce **12 PASS / 4 FAIL / 28 BLOCKED per major**. **Rung acceptance remains
BLOCKED**, with no materialised-native, musl executor/reset/strict-replay result.
Both source/oracle workloads and every required dependency are unchanged.

All eight pushed mains were rechecked **immediately before this final run** and
after it; none changed from the full pin table above. This includes the still
pending plugin/arith publications. [Final pins](evidence/20261109T103340.064773Z/pins.json),
[before](evidence/20261109T103340.064773Z/origin-before-run.json), [after](evidence/20261109T103340.064773Z/origin-after-run.json).
The R-contract prerequisite, both front ends and isolated executor controls pass.
All eight failing row instances still output exactly:
`arith publication differs from plugin pin`.
They are owned by the **new plugin follow-up/reopen request for closed bp-mq6**
in `.tmp/r3-promote/cross-repo-beads.md#plugin-arith-main`, not an open historical
bead. The full determining sentence, original hash values, exact top-level
command/output and completion criterion above remain applicable to every failure.
[Per-row failures and owners](evidence/20261109T103340.064773Z/failures.json),
[publication inspection](evidence/20261109T103340.064773Z/publication-pins.json).

Exact command remains:

```sh
/usr/bin/python3 -B /home/tobias/Projects/bennett-workspace/bennett-conformance-job-r3-promote/.tmp/r3-promote/run.py
```

Unit `bennett-conf-r3-promote-final.service`, invocation
`68dde8cbba62453b8364fb86a3d8d040`, with the same explicit PATH/HOME/headless/local
environment and job class table, returns **exit 1, 4.146480 s wall,
4.506025 s CPU**. All **59 gate subprocesses exit 0**
and have exact argv/wall/CPU/exit/RSS receipts. Immutable sources and build outputs
were reused and verified. No guard tripped. Systemd reports no unit MemoryPeak
for this short cached run; individual command RSS is recorded and the earlier
cold-build peak remains available. Admission used that measured peak plus 6 GiB
headroom (about 54 GiB available, load 11.85 < 32).
[Final receipt](evidence/20261109T103340.064773Z/receipt.json), [raw report](evidence/20261109T103340.064773Z/report.json),
[per-command receipts](evidence/20261109T103340.064773Z/command-receipts.json),
[complete per-row table](evidence/20261109T103340.064773Z/table.md),
[evidence inventory](evidence/20261109T103340.064773Z/raw-evidence-sha256.json).

The cache-adapter regression first failed because the archived source had no
`.tmp/gate/cache` symlink (`.tmp/r3-promote/cache-red.txt`). It now passes after
`Gate.plugin` checks the published cache directory, creates the archive's cache
parent/link and rejects a different link target. CMake's existing pin resolver
still validates exact pinned HEADs and publication bytes. [Cumulative patch
listing every gate-glue line](evidence/20261109T103340.064773Z/gate-glue.patch) records the profile branch,
profile validator, graph selection and these **seven added cache-adapter lines**.
No acceptance method, fixture, oracle, arithmetic check, budget or row was relaxed.

Final focused verification:
`python3 -B -m unittest discover -s ladder/rung-3/tests -v` — **25 PASS**, no skips
(1.806 s); the 12 legacy checks remain PASS; the structural ratchet remains PASS.
Only the two owned rung-3 baseline rows changed; every other value and litter
allowance remains unchanged. [Cumulative quality diff](evidence/20261109T103340.064773Z/quality-diff.patch).
**7bx remains open**. No sibling repo fix, sibling/full push gate, main push,
acceptance closure or public log/scratch/progress publication occurred.


## arith-rev 5e9b450 (the plugin's pinned arith), not arith main

Recorded 2026-11-09T17:26:28Z. **The full rung gate is red: 21 PASS / 3 FAIL / 20 BLOCKED
per major (44 rows each).** The orchestrator explicitly authorized this consistent
publication variant. It does **not** establish acceptance at arithmetic main
`19fb8b5`; that variant still waits on **bennett-plugin-eph**. Earlier all-main
mismatch observations and all historical evidence above are retained unchanged.
**bennett-conformance-7bx stays open; the orchestrator decides acceptance.**

Hello now passes frontend, runtime/arith link, LTO, reversible production,
launch artefacts and actual object admission/hash in **both executors on both
majors**. Its materialised native comparison fails at the driver's contract-version
check. Consequently no required forward equality, reset, saved-tape replay or
corrupt-tape rejection is measured. Formatted printf passes frontend/link/LTO,
then the real producer exhausts its analysis allowance on each major. Its full
213-byte oracle, signed/unsigned extremes, width/precision and final buffered
unterminated line remain required. No workload, oracle, budget or acceptance
method was changed. Passing isolated device/reset/replay diagnostics cannot
satisfy either corpus workload.

### Selected inputs and publication proof

Pin checkpoint: `f0aec3b0a5a28e507d09c26f84daa02edffa7b55`, backed up to
`wip/r3-promote`. These are pushed mains observed by `git -C <sibling> ls-remote
origin main`, except the explicit arithmetic exception. Both remote observations
agree; no sibling main moved across this execution.
[Before](evidence/20261109T171853.561083Z/origin-before-run.json),
[after](evidence/20261109T171853.561083Z/origin-after-run.json).

| Input | Selected commit |
|---|---|
| bennett-plugin | `1bfe6613624bfd004b5dff649f9e4db8e1b07a44` |
| bennett-vm | `83b5f7f0248fe1c36ebfe3acd7fb187ed301affa` |
| bennett-runtimes | `20ee8b10f9dfa58493b379c556b0a4a3aa5263c4` |
| bennett-driver | `bcd4e038d8663e319abfd7003216a4585cfcbc2b` |
| bennett-arith | `5e9b4509f3ee3f95ac9f7d22b43359e72cf9fdab` |
| bennett-isa-spec | `64434934f7f9eaadbffebe34ce9734e33a7ced53` |
| bennett-isa-spec-review | `64434934f7f9eaadbffebe34ce9734e33a7ced53` |
| bennett-dialect-spec | `687284fa28a79c536b3885cabec3adf5de26e031` |

Contract remains **dialect0.3-r1 / ISA0.9 / mapping0.3 / normaliser0.2.8 /
saved-tape-v1**. Both selected arithmetic bitcode digests were checked against
the selected plugin's `SPEC_VERSIONS.toml` and arithmetic `SHA256SUMS` **before
execution**: LLVM18 `a027f16991c06f39eb1305f28da0b1504651791bdd3533888171aa54e704f7ff`;
LLVM23 `8216b27231c36d750c326bbaef384011a1bdf7f7dbb717b8b8f5167551163f79`.
[Publication proof](evidence/20261109T171853.561083Z/publication-pins.json).
Runtime headers/publications are archived from their pin; the reference is built
from pinned ISA6443493. Plugin CMake independently consumes its published verified
pin cache. No sibling source was fixed.

### Executed command and receipts

Working directory: `/home/tobias/Projects/bennett-workspace/bennett-conformance-job-r3-promote`.
The unchanged full gate was invoked as:

```sh
/usr/bin/python3 -B /home/tobias/Projects/bennett-workspace/bennett-conformance-job-r3-promote/.tmp/r3-promote/run.py
```

Detached unit `bennett-conf-r3-arith5e9.service`, invocation
`1342fe07e5d84f8b8d5661af5f08c7dd`, was launched with `systemd-run --user
--setenv=PATH="$PATH" --setenv=HOME="$HOME"`; exact launch argv are in
[launch.json](evidence/20261109T171853.561083Z/launch.json).
Desktop variables were unset; TMPDIR/CARGO_TARGET_DIR are job-local. Eight build
workers were derived from64threads/MemAvailable with6GiB headroom for concurrent
jobs. Admission required the recorded cold-run peak plus6GiB and load<32; it
passed. Existing cited Spiral short/moderate/long/eternal guards bound every
command and the whole run; **none tripped**. The durable guard-table gap remains
bennett-conformance-ji0.

Result: **exit1, 220.167322s wall, 700.166137s
unit CPU, 5418496000 bytes peak memory**. All **91 executed
commands** have argv/wall/CPU/exit/RSS receipts. Six subprocesses exit1 (the six
actual FAIL row instances below); all others exit0. Blocked rows were not run.
[Receipt](evidence/20261109T171853.561083Z/receipt.json),
[commands](evidence/20261109T171853.561083Z/command-receipts.json),
[raw report](evidence/20261109T171853.561083Z/report.json),
[raw table](evidence/20261109T171853.561083Z/table.md),
[raw file hashes](evidence/20261109T171853.561083Z/raw-evidence-sha256.json).
Raw local run: `.tmp/r3-promote/runs/20261109T171853.561083Z/`; full local unit output:
`.tmp/r3-promote/arith5e9/gate.log`. No logs, transcripts, scratch/progress files
or binaries are included in this public evidence package. No full repository
push gate or sibling gate was run.

### Every measured row and owner

The original raw report/table preserve UNFILED values. The separate
[classified report](evidence/20261109T171853.561083Z/classified/report.json)
and [table](evidence/20261109T171853.561083Z/classified/table.md)
attach ownership using the updated registry; they re-execute no commands and
preserve every status, error, pin, argv and exit. `request-driver-r-contract`
labels a **new cross-repo bead request**, not an existing bead. Its request is
`.tmp/r3-promote/cross-repo-beads.md#driver-r-contract`; **bp-owg** is an existing
open plugin task (passive export), with evidence update requested at `#bp-owg-rung3`.
The driver failure inherits to hello execution/reset/replay rows; the formatted
production failure inherits to all its downstream rows.

| Stage | LLVM 18 | LLVM 23 |
|---|---|---|
| fixtures | PASS | PASS |
| plugin build | PASS | PASS |
| VM build | PASS | PASS |
| driver build | PASS | PASS |
| R contract (prerequisite) | PASS | PASS |
| musl C front end | PASS | PASS |
| runtime bitcode link | PASS | PASS |
| post-link LTO | PASS | PASS |
| reversible production | PASS | PASS |
| launch artefacts | PASS | PASS |
| native comparison | FAIL · request-driver-r-contract | FAIL · request-driver-r-contract |
| reference admission | PASS | PASS |
| reference forward = native | BLOCKED · request-driver-r-contract | BLOCKED · request-driver-r-contract |
| reference reset | BLOCKED · request-driver-r-contract | BLOCKED · request-driver-r-contract |
| reference tape replay | BLOCKED · request-driver-r-contract | BLOCKED · request-driver-r-contract |
| reference corrupt-tape rejection | BLOCKED · request-driver-r-contract | BLOCKED · request-driver-r-contract |
| VM admission | PASS | PASS |
| VM forward = native | BLOCKED · request-driver-r-contract | BLOCKED · request-driver-r-contract |
| VM reset | BLOCKED · request-driver-r-contract | BLOCKED · request-driver-r-contract |
| VM tape replay | BLOCKED · request-driver-r-contract | BLOCKED · request-driver-r-contract |
| VM corrupt-tape rejection | BLOCKED · request-driver-r-contract | BLOCKED · request-driver-r-contract |
| printf_fmt: musl C front end | PASS | PASS |
| printf_fmt: runtime bitcode link | PASS | PASS |
| printf_fmt: post-link LTO | PASS | PASS |
| printf_fmt: reversible production | FAIL · bp-owg | FAIL · bp-owg |
| printf_fmt: launch artefacts | BLOCKED · bp-owg | BLOCKED · bp-owg |
| printf_fmt: native comparison | BLOCKED · bp-owg | BLOCKED · bp-owg |
| printf_fmt: reference admission | BLOCKED · bp-owg | BLOCKED · bp-owg |
| printf_fmt: reference forward = native | BLOCKED · bp-owg | BLOCKED · bp-owg |
| printf_fmt: reference reset | BLOCKED · bp-owg | BLOCKED · bp-owg |
| printf_fmt: reference tape replay | BLOCKED · bp-owg | BLOCKED · bp-owg |
| printf_fmt: reference corrupt-tape rejection | BLOCKED · bp-owg | BLOCKED · bp-owg |
| printf_fmt: VM admission | BLOCKED · bp-owg | BLOCKED · bp-owg |
| printf_fmt: VM forward = native | BLOCKED · bp-owg | BLOCKED · bp-owg |
| printf_fmt: VM reset | BLOCKED · bp-owg | BLOCKED · bp-owg |
| printf_fmt: VM tape replay | BLOCKED · bp-owg | BLOCKED · bp-owg |
| printf_fmt: VM corrupt-tape rejection | BLOCKED · bp-owg | BLOCKED · bp-owg |
| runtime libc/console audit (diagnostic) | PASS | PASS |
| R-mode probe (diagnostic) | PASS | PASS |
| driver probe (diagnostic) | FAIL · request-driver-r-contract | FAIL · request-driver-r-contract |
| reference device/reset control (diagnostic) | PASS | PASS |
| reference replay API (diagnostic) | PASS | PASS |
| VM device/reset control (diagnostic) | PASS | PASS |
| VM replay API (diagnostic) | PASS | PASS |

### Determining failures

Every FAIL row below has its **complete exact argv and output**, determining
sentence, command exit/wall/CPU and owning bead/request in
[failures.json](evidence/20261109T171853.561083Z/failures.json).
The command tag identifies the exact argv entry in command-receipts.json.

| LLVM | Failing row | Command tag | Owner | Exact determining diagnostic |
|---|---|---|---|---|
| 18 | native comparison | `18/hello/driver-comparison` | driver-r-contract request (driver) | `bennett-driver: BD-E-VERSION: required dialect 0.2.13 (or explicit historical 0.2.10) and ISA 0.8` |
| 18 | printf_fmt: reversible production | `18/printf_fmt/production` | bp-owg (plugin) | `bennett: analysis-resource: §5.2 PT2 budget exhausted at AV(%153) in @__bennett_rt_vfs_io; compiler resource allowance: thread CPU milliseconds at   %153 = getelementptr inbounds i8, ptr %144, i64 %152` |
| 18 | driver probe (diagnostic) | `18/driver-probe` | driver-r-contract request (driver) | `bennett-driver: BD-E-VERSION: required dialect 0.2.13 (or explicit historical 0.2.10) and ISA 0.8` |
| 23 | native comparison | `23/hello/driver-comparison` | driver-r-contract request (driver) | `bennett-driver: BD-E-VERSION: required dialect 0.2.13 (or explicit historical 0.2.10) and ISA 0.8` |
| 23 | printf_fmt: reversible production | `23/printf_fmt/production` | bp-owg (plugin) | `bennett: analysis-resource: §5.2 PT6 budget exhausted at AV(%189) in @tatan; compiler resource allowance: thread CPU milliseconds at   %189 = load i512, ptr %11, align 16, !noalias !78` |
| 23 | driver probe (diagnostic) | `23/driver-probe` | driver-r-contract request (driver) | `bennett-driver: BD-E-VERSION: required dialect 0.2.13 (or explicit historical 0.2.10) and ISA 0.8` |

Driver determination: pushed driverbcd4e03 rejects the explicitly accepted
0.3-r1/ISA0.9 contract at its **arguments stage**. Its embedded legacy BD-B-ISA7
message is not evidence of an ISA0.9 spec defect. Completion requires explicit
driver/native-comparison adoption of the selected contract and retained exact
publication/materialised/static-link audit; downgrading conformance flags would
not satisfy acceptance. Formatted-production determination: the full ordinary
source exits1 with §5.2 analysis-resource on each major (LLVM18 PT2/VFS GEP;
LLVM23 PT6/tatan i512 load). This is an observed compiler resource frontier,
not a host guard trip or an established internal root cause. bp-owg completion
still requires full unchanged printf materialisation, exact native/executor
agreement and reset/replay on both majors. No sibling fix or scope reduction was
attempted.

### Scoped verification

`python3 -B -m unittest discover -s ladder/rung-3/tests -v`: **25 PASS**,
no skips (including real tiny LLVM18/23 native-receipt adapter checks), before
execution at the selected pins. These are glue checks, not corpus acceptance.
No new gate glue or acceptance-check line changed relative to db38cf2;
[glue disposition](evidence/20261109T171853.561083Z/gate-glue-changes.json).
`docs/quality.md` is unchanged by this continuation; the required cumulative
`git diff f1b75cc..HEAD -- docs/quality.md` receipt changes only the predecessor's
two owned rung-3 rows:
[quality diff](evidence/20261109T171853.561083Z/quality-diff.patch).
The full gate is not green and no acceptance closure is claimed.

Final evidence validation (2026-11-09T17:28:07Z): `python3 -B ladder/rung-3/check_gate.py` —
**12 PASS** after the ownership update; `python3 -B scripts/quality_ratchet.py` —
**PASS**. All three focused invocations have guarded wall/CPU/exit receipts in
[focused-check-receipts.json](evidence/20261109T171853.561083Z/focused-check-receipts.json).
[Integrity validation](evidence/20261109T171853.561083Z/integrity.json) verifies all **246 raw file hashes**,
byte-identical raw report/table, unchanged historical directories and RESULTS
prefix, all44rows per major, exact failure/status preservation in the ownership
classification, complete command receipts and headless/job-local environment.
Only pins, this append, blocker ownership and the new evidence directory changed
in this continuation. **Rung acceptance remains BLOCKED.**

## 2026-11-12: isolated hosted native provider consumer

Implementation `8cb2d15` selects the frozen real C launch-provider DSO and
canonical target from `pins.json`. Producer and comparison use data base 65536
and text base 4194304. Native O0/O2 comparison now passes on LLVM 18 and 23:
exact `hello, world\n`, status 0, no signal/provider fault, original input hashes
unchanged, static component closure and fixed ranges audited independently of
hosted imports, and fresh namespace/load/binding/entry/unload receipts validated.
Bitcode replay preserves the original materialisation and the existing linked
components; no relinked component supplies the baseline.

The complete unchanged hello + formatted-printf gate ran with:

```sh
unset DISPLAY WAYLAND_DISPLAY DBUS_SESSION_BUS_ADDRESS BROWSER
export TMPDIR="$PWD/.tmp" CARGO_BUILD_JOBS=5 RUST_TEST_THREADS=5
ladder/rung-3/gate.sh --jobs 5 --output .tmp/vip/final-runs
```

Run `20261112T062703.088778Z`, exit **1**, 174 recorded commands, no recorded
command timeout. The local `report.json` SHA-256 is
`cd79558a880a7cd084d11630ccc459f9258953fe80198fc6e41441db9e695491`.
All four gate/audit source fingerprints match the implementation commit.
Raw command receipts, component/config/runtime artifacts and observations stay
in the local evidence directory; no host paths or raw logs are copied here.

| LLVM major | Prior recorded run (PASS / FAIL / BLOCKED) | This run (PASS / FAIL / BLOCKED) | Native comparison |
|---|---|---|---|
| 18 | 22 / 3 / 20 | 23 / 4 / 18 | PASS (O0 and O2) |
| 23 | 22 / 3 / 20 | 23 / 4 / 18 | PASS (O0 and O2) |

The prior counts are the `20261111T191456.782662Z` run recorded in the VIP bead;
they are not a new measurement here. An exploratory run of the new selection
before the linker-parser fix also completed at 22 / 3 / 20 per major, with native
audit failure on quoted linker identifiers. That consumer defect is fixed and
covered by regression tests. The focused existing-artifact audit subsequently
passed all four major/optimisation combinations before the full run above.

Remaining failures are distinct from provider selection:

* Both executor forward rows now reach the pre-existing consumer requirement for
  `out/main/argv-replies.json`, which is absent. They fail before any executor
  forward invocation. The gate produces the ISA artifact directly through the
  plugin, whereas pinned driver `docs/PROCESS_ARGUMENTS.md` publishes reply
  generation during its process build. This is remaining conformance/process
  adapter integration under the rung tracker, not evidence of a missing driver
  serializer. No replies are fabricated and no host-request check is relaxed.
* `18/printf_fmt/production`: plugin dialect §5.2 `analysis-resource`, PT4/PT5 at
  `AV(%25)` in `@tpow`, compiler thread CPU allowance exhausted.
* `23/printf_fmt/production`: plugin dialect §5.2 `analysis-resource`, PT8/PT9/PT12
  at `AV(%66)` in `@trans_eval`, compiler thread CPU allowance exhausted.
  These are the existing `bp-owg` frontier, not host-command timeouts.
* Both `18/driver-probe` and `23/driver-probe`: the driver reports
  `DRIVER-PLUGIN-REFUSAL`, §19 inputs/options and §25 fail-loud obligations;
  its pinned producer exits signal 11 (`SIGSEGV`) in `lowerQuantumMarker`,
  `src/ProductionMultiGates.inc:73:57`. The implementation owner is the plugin;
  the driver is reporting the actual producer failure.

The gate command above reproduces these frontiers. Exact per-major argv and
stderr are in the report's commands with the named tags. Inputs remain pinned
as listed in `pins.json`; no sibling patch or live-checkout substitution was
made. Reset, replay and corrupted-tape corpus rows remain blocked by these
prerequisites. **Rung acceptance is not established; bennett-conformance-7bx
remains BLOCKED until the complete gate exits 0.**

Validation also ran `python3 -B -m unittest discover -s ladder/rung-3/tests -v`
(46 PASS), `python3 -B ladder/rung-3/check_gate.py` (12 PASS), and the quality
ratchet self-test and check (PASS). The audit regressions cover provider/target
selection, shared layout, closure replay and hostile leaves, original-input
hashes, component ELF/range/heap checks, separate hosted dependency evidence,
stale receipts, lifecycle outcomes, and actual stdout comparison.

## 2026-11-12: published argv launch replies in the consumer

Implementation `56efc02` fixes the missing-replies boundary without weakening any
acceptance criterion or changing an implementation pin, fixture or budget. Contract ownership
was established from the frozen publications before implementation:

* Dialect `9bbf6dee` §19.2 requires the CC argv/envp launch service.
* Plugin `512236ee`, `docs/R_PROCESS_ARGUMENTS.md`, publishes the launch-adapter
  obligation and `bennett.argv.bytes` v1. The build marker
  `entry_initialisation.arguments = "bennett-argv-bytes-1"` selects two requests
  carrying the same complete payload; pointer-unused entries retain the legacy
  `bennett.argv` header. Its `tests/r_process_arguments.py` and
  `tests/reversible.py` publish executable examples of these reply formats.
* Driver `aeb01e18`, `docs/PROCESS_ARGUMENTS.md`, publishes reply-file generation
  during its process build and empty argv/envp by default. Its
  `tests/process_arguments_codec.py` confirms both empty-context encodings.
  Direct `opt` production does not invoke that serializer. The consumer is the
  launch adapter on this route; no missing driver/plugin publication is inferred.

The gate now selects the protocol from the actual build record, verifies the
object's service/version and pointer width, and generates consumer
`argv-replies.json`. Forward, both reversal paths and saved replay must report
exactly the selected request capacities, payloads and replies. No console/libc
host reply is permitted. Build/object/reply hashes and the empty process context
are recorded in the manifest. Unknown protocols or versions fail at this row.

Both actual hello objects select legacy `bennett.argv` v0.2.10 with a zeroed
20-byte header (argc 0 and two null pointers). The reply file SHA-256 is
`e10c9501b9b550b5d4ad16d676dd16b536ef8333ea2828b2c13e8d44ba95ae2d`
on both majors. Empty byte-import contexts at 32/64 bits are covered by glue
regressions; this run does not claim a byte-import corpus execution.

The full unchanged hello + formatted-printf gate built its own pinned components
and ran with:

```sh
env -u DISPLAY -u WAYLAND_DISPLAY -u DBUS_SESSION_BUS_ADDRESS -u BROWSER \
  TMPDIR="$PWD/.tmp" CARGO_BUILD_JOBS=4 RUST_TEST_THREADS=4 \
  ladder/rung-3/gate.sh --jobs 4 --output .tmp/argv/runs
```

Run `20261112T063949.118296Z`, exit **1**, **195 recorded commands**, no recorded
command timeout. The local `report.json` SHA-256 is
`1fbbe52bc90e7518e91e585fc418da6cb78ef436ec6ceb363465a6b7bd685979`.
All four gate/audit source fingerprints match the implementation. Raw receipts
and exact per-command argv remain in the local run directory; no raw logs or
host paths are copied into this append.

| LLVM major | Prior recorded run (PASS / FAIL / BLOCKED) | This run (PASS / FAIL / BLOCKED) |
|---|---|---|
| 18 | 23 / 4 / 18 | 31 / 2 / 12 |
| 23 | 23 / 4 / 18 | 31 / 2 / 12 |

The prior counts were independently read from the previous section's
`20261112T062703.088778Z` report, whose SHA-256 matches its recorded digest;
they are not a second baseline execution here. Both runs have 45 rows per major.

Every hello row now passes on both majors. The isolated hosted native comparison
passes O0/O2. Reference and VM forward runs complete with i32 status 0, exact
`hello, world\n`, no faults and exactly the supplied argv exchange. Their ordered
console tapes contain two writes (12 bytes and LF). Both `run-program --reverse`
and retained-history `reverse` report REVERSED, full-state and memory equality,
equal entry/final memory digests, no first difference, empty console and zero
live effect-tape length. Saved-tape replay with empty entropy reproduces the
status, result, console, tape and length. All four corrupted-tape commands exit
1 with a §7.3/§10 replay-mismatch diagnostic at the first console effect.

The remaining failures were left unchanged in ownership and scope:

| LLVM | Row / command tag | Owner | Determining observation |
|---|---|---|---|
| 18 | printf_fmt: reversible production / `18/printf_fmt/production` | plugin, `bp-owg` | §5.2 PT8/PT9/PT12 budget exhausted at `AV(%58)` in `@trans_eval`; thread CPU milliseconds, `llvm.lifetime.end.p0(i64 80, ptr nonnull %58)` |
| 23 | printf_fmt: reversible production / `23/printf_fmt/production` | plugin, `bp-owg` | §5.2 PT8/PT9/PT12 budget exhausted at `AV(%3)` in `@texp`; thread CPU milliseconds, `llvm.lifetime.start.p0(ptr nonnull %3)` |
| 18, 23 | driver probe (diagnostic) / `18/driver-probe`, `23/driver-probe` | plugin | Driver exits 1 with `DRIVER-PLUGIN-REFUSAL`, §19 inputs/options and §25 fail-loud obligations; producer signal 11 (SIGSEGV), `lowerQuantumMarker`, `src/ProductionMultiGates.inc:73:57`. |

The probe remains `UNFILED` in the unchanged historical classifier; its measured
implementation owner is the plugin, as shown by both backtraces. It is independent
of the hello corpus chain. Every BLOCKED row inherits `bp-owg`: formatted launch
artefacts and native comparison, plus each executor's admission, forward, reset,
saved replay and corrupt-tape rejection (12 per major). No new cross-repo argv
contract gap was found, no upstream implementation was substituted, and neither
other failure was worked around. **Rung acceptance remains BLOCKED;
bennett-conformance-7bx stays BLOCKED until the complete gate exits 0.**

Verification in the same headless, repository-local temporary environment:

* `python3 -B -m unittest discover -s ladder/rung-3/tests`: **50 PASS**, no skips,
  including real LLVM18/23 native receipt integration. An initial run found one
  older stdout fixture still using a nonexistent reply path; supplying its real
  synthetic argv exchange preserved the buffered-tail assertion and made it pass.
* `python3 -B ladder/rung-3/check_gate.py`: **12 PASS**.
* `python3 -B scripts/quality_ratchet.py --self-test` and
  `python3 -B scripts/quality_ratchet.py`: **PASS**. Only the owned gate baseline
  row changed, with its measured explanation in `docs/quality.md`.

The new protocol tests failed against the original missing-file implementation.
A no-op mutation of the exchange validator is rejected by the tests (six
assertion failures), including real call wiring for forward, both reversal paths
and replay. These glue checks do not replace corpus evidence. Only test-fixture
and documentation changes followed the completed full-run source measurement.
