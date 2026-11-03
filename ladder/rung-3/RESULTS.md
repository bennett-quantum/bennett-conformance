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
