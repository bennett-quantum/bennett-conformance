# Rung 4 measured gate — 2026-11-01T22:16:46Z

**Rung 4 is not accepted: the gate is red, as expected.** No lifted SQLite oracle exists yet, so clauses
(a), (b) and (c) have no result. The run's report has non-PASS rows on both majors, which `finish()` turns
into exit 1 (covered by `check_gate.py`). The shell exit status of this recording was not captured.

- [Report](evidence/20261101T221646.155856Z/report.json): exact argv, exits, durations, hashes, pins, errors
- [Table](evidence/20261101T221646.155856Z/table.md), [gate log](evidence/20261101T221646.155856Z/gate.log), [SHA-256 inventory of every raw file](evidence/20261101T221646.155856Z/raw-evidence-sha256.json)
- Raw run: `.tmp/rung-4/runs/20261101T221646.155856Z/` (local). Discovery run: `.tmp/rung-4/runs/20261101T213213.441081Z/`, which the beads cite and which measured the same failures before the mapping existed.

[DESIGN.md](DESIGN.md) says what each row means. Neither a diagnostic row nor a control can satisfy a required row.

## Result, both majors

| Stage | LLVM 18 and LLVM 23 |
|---|---|
| fixtures; SQLite source unmodified; independent predicate | PASS |
| plugin, VM, driver, estimator, checker builds from pins | PASS |
| host image = native oracle = predicate | PASS |
| images agree across majors | PASS |
| musl C front end; runtime bitcode link; post-link LTO | PASS |
| **runtime libc closure** | **FAIL · brt-dj7** |
| **coherent production** | **FAIL · bp-1bb** |
| launch artefacts and oracle shape | BLOCKED · bp-1bb |
| materialised comparison = native | BLOCKED · bp-1bb, brt-dj7 |
| reference and VM: admission, (a) three rows, (b) two rows | BLOCKED · bp-1bb, brt-dj7 |
| (a) certificate verdict | BLOCKED · bp-1bb, brt-dj7 |
| (c) resource estimate at N=2^20 | BLOCKED · bp-1bb |
| reference and VM reduced control (diagnostic) | PASS |
| published Grover TU production (diagnostic) | FAIL · bp-vsa |
| reference `--mem-file`; `--state-out` (diagnostic) | FAIL · b-isa-spec-mtc; b-isa-spec-u6o |
| VM `--mem-file`; `--state-out` (diagnostic) | FAIL · bennett-vm-0mp; bennett-vm-1wf |
| estimator CLI (diagnostic) | FAIL · bennett-estimate-b8o |
| checker CLI (diagnostic) | FAIL · bennett-conformance-51d |

## Ordered blockers

[blockers.json](blockers.json) holds the determining sentences, exact commands and completion criteria
copied into the owning beads. Sizes are estimates.

| Order | Bead, repository | Frontier measured | Fixed means | Estimate |
|---|---|---|---|---|
| 1 | **brt-dj7**, bennett-runtimes | After link and `lto<O3>`, undefined: `bcmp`, `localtime`, `realloc`, `strcspn`, `strncmp`, `strrchr`, `strspn`, both majors. Dialect §4.5 makes these trapped calls, and SQLite executes them. | Definitions published for both majors; closure row green. | 480 min |
| 2 | **bp-1bb**, bennett-plugin | N = 16, both majors: `§5 PT10 composite operation` at `getelementptr …, <2 x i64>`, 113–189 s into analysis. This is SLP output from the default pipeline. | Vector GEP handled lane-wise (PT2, N21). Disabling the vectoriser is not a fix. | 480 min |
| 3 | **bp-fj1**, bennett-plugin | Exploratory, LLVM 18 only, see below: without vector GEPs, analysis still reaches no fixpoint in 1400 s (`§5.2 PT6 budget exhausted at AV(%407) in @sqlite3_step`). | Fixpoint on the whole closure, both majors, at a recorded practical budget. | 1440 min |
| 4 | **bp-vsa**, bennett-plugin | Published `rung4-grover-tu` (verdict=accept) rejected at `__bennett_qprep`; by source reading, `gate2` and `gate3` hit the same default. | CC lowering, §17.4; the published TU and `fixtures/grover.c` launch. | 480 min |
| 5 | **b-isa-spec-mtc**, bennett-isa-spec (reference) | `--mem-file` unknown; `--mem` hex cannot reach A = 16: execve OSError 7 at 131,072 characters. | Region bytes from a file. | 120 min |
| 6 | **bennett-vm-0mp**, bennett-vm | Same. | Same. | 120 min |
| 7 | **b-isa-spec-u6o**, bennett-isa-spec (reference) | `--state-out` unknown; a launch reports only measured values. | Pre-measurement state readout. | 240 min |
| 8 | **bennett-vm-1wf**, bennett-vm | Same. | Same. | 240 min |
| 9 | **bennett-estimate-b8o**, bennett-estimate | At pin, prints its version and ignores arguments; stream muse-estimate is on this repo. | Labelled estimate of the actual 2^20 object: budget, padded cost. | 480 min |
| 10 | **bennett-conformance-51d**, bennett-conformance (checker) | `bennett-check` CLI stub at pin; stream muse-checker is on it. | Verdict JSON, `evidence` with exact fraction. | 240 min |

Tracking: **bennett-conformance-tmb**, open until the gate exits 0. Note appended to **bp-xv0**: the default
30 s analysis guard fails SQLite earlier (`§5.2 PT11 seeds budget exhausted`), so the gate passes an explicit
limit (`pins.json`), not relying on any default.

Stages behind these rows have never executed. More blockers there are likely, for example in the
materialised comparison, budget sizes, or executor speed on 2.9e5-row oracle calls (REPORT §7). Each will
get a bead when measured; none is presumed here.

**Contract decision, not blocking: b-isa-spec-xv4.** North Star §5.1 calls an exhaustive claim over a declared
finite domain `proof`, and its D has no region cells. ISA 0.9 §12.2 labels any claim fixing a region
`evidence`, which at A = 16 is a covered fraction of 2^-524288 for an exhaustive statement about this
database. Dialect §25.2 lists the question as open. The gate requires today's label and never reports
`proof`. The bead proposes text for a fixed-image binding.

## What works today

- **Native SQLite equals the independent predicate on every x at every size,** N up to 2^20, including all
  2^20 rows (the spike sampled V1 at 2^20). Both majors; images are byte-identical across majors. M = 0/16,
  2/64, 6/256, 13/1000, 13/1024, 11,698/2^20.
- **Unmodified SQLite compiles against the published musl sysroot** on both majors and survives link and
  `lto<O3>`, still defining `sqlite3VdbeExec`, `memdbRead`, `sqlite3_deserialize` and `sqlite3BtreeTableMoveto`.
- **The reduced control passes on both executors:** published `rung4-mini` admission, all 18 published run
  lines including budget faults, and its step check. This shows that the executors and glue work on that
  object. It says nothing about SQLite.
- **Glue tests:** 32 pass (`python3 check_gate.py`, output in the evidence directory), including rejection of
  edited SQLite, altered shape, wrong k, garbage in fault, ancilla or database registers, and `proof` on the
  fixed image. Synthetic values never count as acceptance.

Grover plans from the independent table (closed form, not measured): N = 2^6: M = 2, k = 4, P = 0.99918.
N = 2^8: M = 6, k = 5, P = 0.98570. N = 2^20: M = 11,698, k = 7, P = 0.99973.

## Exploratory probe (no acceptance value)

To see past bp-1bb, not to pass anything: LLVM 18, N = 256, plugin 3a856a7, the same program built with
`-fno-vectorize -fno-slp-vectorize` and `-vectorize-loops=false -vectorize-slp=false`, so the module has
no vector GEP. Limits 1400 s, 14 GB. Result: exit 1 after real 23m29s, user 16m13s, shared machine at load
about 27, at the bp-fj1 message. Not repeated on LLVM 23. An intermediate run with only `opt` vectorisation
off still had the pre-link vector GEPs and stopped at bp-1bb. Artefacts: `.tmp/opus-rung4/proto/nv/` (local,
not recorded in `evidence/`). This build is forbidden as acceptance (DESIGN §10).

## Reproduce

```sh
ladder/rung-4/fixtures/fetch-sqlite.sh   # only if ~/.local/share/bennett-sqlite has no copy
ladder/rung-4/gate.sh                    # add --record to copy evidence
```

Pins: [pins.json](pins.json). The recorded run took about 11 minutes with builds cached, under load; acceptance
once open is hours (DESIGN §11). Pins will need moving as beads land: that is the rung's ordinary maintenance.
