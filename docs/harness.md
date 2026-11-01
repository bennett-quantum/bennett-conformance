# Differential harness

`harness/bdiff.py` drives any two or more executors of ISA objects as black-box binaries over corpora and
generated programs, compares every observable the command-line contract gives, and reports divergences with
minimised witnesses. It shares no code with any executor: objects, vectors and fixtures are read as data, and
`harness/lib/identity.py` implements §9.2 from the spec text. Its third-party check is that it reproduces spec example 1,
the stored hashes of every valid object in the reference fixtures, conformance objects, neutral suite and ladder evidence, and the neutral suite's closure hashes (`harness/tests/test_identity.py`).

## Running it

```sh
J=.tmp/opus-harness; export TMPDIR=$PWD/$J/tmp      # TMPDIR must be inside the workspace, or it refuses to run
python3 -I -m unittest discover -s harness/tests     # self-tests; BENNETT_HARNESS_REF names the real reference binary
python3 -I harness/bdiff.py \
  --exec ref=$J/bin/bennett-isa-ref \
  --exec vm-int=$J/bin/bennett-vm,BENNETT_VM_ENGINE=interpreter \
  --exec vm-jit=$J/bin/bennett-vm,BENNETT_VM_ENGINE=jit,BENNETT_JIT_AUDIT=1 \
  --corpus all --gen all --seed 1 --wall-seconds 60 --summary out.json
```

An executor is `name=path[,ENV=VALUE...]`, so a later dense engine is one more `--exec`. `--pair A:B` narrows the pairs
(default all). Exit status: 0 clean; 1 a divergence, an oracle failure or nothing executed; 2 nothing failed but
something was unsupported, timed out or skipped. Binaries are best built from committed snapshots: sibling worktrees
are often dirty and rebuilt during a run.

## What is compared, and how

Each process outcome is one of: json, diagnostic, unsupported (explicit `unsupported`, `unimplemented`, `staged deliverable`
on stderr), timeout, crash (a signal, a panic or an exit other than 0/1) or badoutput. Each case lands in exactly one class
per pair, and `summary()` asserts the counts total: `MATCH_EXACT`, `MATCH_TOLERANCE`, `AGREE_REJECT`, `UNSUPPORTED`, `TIMEOUT`, `DIVERGE`.

| field | mode | meaning |
|---|---|---|
| everything by default, including unknown fields, results, faults, steps, tape, console, clock, measurements under fixed `--draws`, trace, reversal end state | **exact** | structural; true is not 1; binary64 by bits |
| `state`, `forward_state` amplitudes | **tolerance** | abs difference at most 1e-12; a separate class, never counted as exact |
| `memory_digest`, `entry_memory_digest` | **relational** | the ISA gives no digest serialisation, so only digest == entry digest is compared across executors; memory bytes are observed through `trace` and loads |
| `first_difference` | **nullness** | descriptive text |
| saved tape file (`--tape-out`) | **exact bytes**, plus **cross replay** | each executor replays its neighbour's tape and must reproduce its own stdout byte for byte |
| diagnostics | exit status only | differing cited rules are an advisory count, since rule text is not contract |

Spec oracle: ISA vectors (`expect`) and neutral-suite expectations judge each executor separately, on top of the pairwise
comparison. The harness never edits them. Exact neutral-suite expectations use the same field policy, so opaque digests
recorded from the reference do not fail other executors.

## Corpora and generators

`isa-vectors` (ISA §15 cases), `neutral-suite`, `vm-differential` (argv lists extracted as data from bennett-vm's differential test
by `harness/tools/extract_vm_differential.py`), `fixture-objects` and `ladder-evidence` (invocations derived from symbol signatures by
`harness/lib/drivers.py`). Whatever is not driven is a counted skip with a reason.

Generators (`harness/lib/generate.py`; SplitMix64, fixed in `prng.py`, so a seed means the same bytes on any Python):
`r` arithmetic, memory, effects, segments, branches and loops; `lift` oracle rows, lprocs, recursion, nested oracles, regions and globals; `cc`
registers, measurement, region calls; `q` gates, `qif`, `qloop` (and `while`), `qpred`, `ctl`/`adj` calls, windows, local regions; `mutant` one random
edit of a well-formed program with identity recomputed, to find validator gaps. A wall bound truncates a seed sequence, never
changes it; the summary names the seed range. Not generated yet (bead bennett-conformance-g45): ISA 0.9 objects, `qborrow`, `hostcall`,
`qinplace`, `lift.perm_call`, action memory regions. They are reached through corpora only.

Witnesses (`harness/lib/shrink.py`): drop option groups, zero values, drop symbols, ddmin lines, drop `regs` entries, reduce literals, recomputing
§9.2 at each step, keeping a candidate only under the same divergence signature (command, outcome kinds, diverging top-level fields, cited rule).
Budgeted and deterministic for a given run; an exhausted budget or an object with inconsistent stored hashes is stated in `shrink_status`.
A signature fixes the outcome kinds, not the root cause, so a shrink may slip to a neighbouring bug on the same field: read the witness.

## Recorded run, 2026-11-01

Pins: bennett-isa-spec 4900508 (reference sha256 15ed552e...), bennett-vm 2456722 (sha256 76354dc4...), built by `cargo build --release` from
`git archive HEAD`. Command as above with `--witnesses-per-signature 1 --shrink-seconds 6 --timeout 20`. Exit status **1**, because divergences exist. Full data:
`harness/runs/2026-11-01-seed1.json`, `.log`. Each generator class ran under the 60 s bound including shrinking: r 168 programs (seeds 1..168), lift 152, cc 72, q 96, mutant 104; no
well-formed program was rejected by every executor. Totals below are per pair, 3 pairs: 11640 cases, 29251 MATCH_EXACT, 338 MATCH_TOLERANCE, 4733 AGREE_REJECT, 8 UNSUPPORTED, 0 TIMEOUT,
590 DIVERGE, 193 counted skips, 57 divergence signatures. 12 oracle failures (6 cases, 2 VM engines). Of 29653 json pairs compared, 25797 were byte-identical; the rest differ at least in opaque digests or tolerated amplitudes.
356 AGREE_REJECT pairs cited different rule tokens (advisory). The JIT audit reported native runs on 5936 executor outcomes, so the JIT pair is not vacuous.

| corpus | cases | pair | MATCH_EXACT | MATCH_TOLERANCE | AGREE_REJECT | UNSUPPORTED | TIMEOUT | DIVERGE |
|---|---:|---|---:|---:|---:|---:|---:|---:|
| fixture-objects | 3854 | ref:vm-int | 3133 | 28 | 531 | 0 | 0 | 162 |
| fixture-objects | 3854 | ref:vm-jit | 3133 | 28 | 531 | 0 | 0 | 162 |
| fixture-objects | 3854 | vm-int:vm-jit | 3165 | 0 | 683 | 0 | 0 | 6 |
| gen-cc | 792 | ref:vm-int | 461 | 0 | 294 | 0 | 0 | 37 |
| gen-cc | 792 | ref:vm-jit | 461 | 0 | 294 | 0 | 0 | 37 |
| gen-cc | 792 | vm-int:vm-jit | 493 | 0 | 299 | 0 | 0 | 0 |
| gen-lift | 1672 | ref:vm-int | 1672 | 0 | 0 | 0 | 0 | 0 |
| gen-lift | 1672 | ref:vm-jit | 1672 | 0 | 0 | 0 | 0 | 0 |
| gen-lift | 1672 | vm-int:vm-jit | 1672 | 0 | 0 | 0 | 0 | 0 |
| gen-mutant | 1098 | ref:vm-int | 350 | 13 | 642 | 0 | 0 | 93 |
| gen-mutant | 1098 | ref:vm-jit | 350 | 13 | 642 | 0 | 0 | 93 |
| gen-mutant | 1098 | vm-int:vm-jit | 456 | 0 | 642 | 0 | 0 | 0 |
| gen-q | 1344 | ref:vm-int | 1218 | 126 | 0 | 0 | 0 | 0 |
| gen-q | 1344 | ref:vm-jit | 1218 | 126 | 0 | 0 | 0 | 0 |
| gen-q | 1344 | vm-int:vm-jit | 1344 | 0 | 0 | 0 | 0 | 0 |
| gen-r | 1176 | ref:vm-int | 1176 | 0 | 0 | 0 | 0 | 0 |
| gen-r | 1176 | ref:vm-jit | 1176 | 0 | 0 | 0 | 0 | 0 |
| gen-r | 1176 | vm-int:vm-jit | 1176 | 0 | 0 | 0 | 0 | 0 |
| isa-vectors | 447 | ref:vm-int | 413 | 1 | 31 | 2 | 0 | 0 |
| isa-vectors | 447 | ref:vm-jit | 413 | 1 | 31 | 2 | 0 | 0 |
| isa-vectors | 447 | vm-int:vm-jit | 414 | 0 | 32 | 1 | 0 | 0 |
| ladder-evidence | 866 | ref:vm-int | 861 | 0 | 5 | 0 | 0 | 0 |
| ladder-evidence | 866 | ref:vm-jit | 861 | 0 | 5 | 0 | 0 | 0 |
| ladder-evidence | 866 | vm-int:vm-jit | 861 | 0 | 5 | 0 | 0 | 0 |
| neutral-suite | 346 | ref:vm-int | 322 | 1 | 22 | 1 | 0 | 0 |
| neutral-suite | 346 | ref:vm-jit | 322 | 1 | 22 | 1 | 0 | 0 |
| neutral-suite | 346 | vm-int:vm-jit | 323 | 0 | 22 | 1 | 0 | 0 |
| vm-differential | 45 | ref:vm-int | 45 | 0 | 0 | 0 | 0 | 0 |
| vm-differential | 45 | ref:vm-jit | 45 | 0 | 0 | 0 | 0 | 0 |
| vm-differential | 45 | vm-int:vm-jit | 45 | 0 | 0 | 0 | 0 | 0 |

| corpus | executor | pass | fail | unavailable |
|---|---|---:|---:|---:|
| isa-vectors | ref | 445 | 0 | 2 |
| isa-vectors | vm-int | 445 | 0 | 2 |
| isa-vectors | vm-jit | 445 | 0 | 2 |
| neutral-suite | ref | 346 | 0 | 0 |
| neutral-suite | vm-int | 340 | 6 | 0 |
| neutral-suite | vm-jit | 340 | 6 | 0 |

| corpus | reason | count |
|---|---|---:|
| fixture-objects | action interface wider than 256 wires: no basis vectors | 3 |
| fixture-objects | data symbol: no execution command (identity only) | 8 |
| fixture-objects | harness parser refused object: unknown section 'bogus' | 1 |
| fixture-objects | lproc: reached only through its callers (§3.7) | 10 |
| isa-vectors | no CLI probe: staged deliverable 10 | 31 |
| isa-vectors | no CLI probe: staged deliverable 11 | 9 |
| isa-vectors | no CLI probe: staged deliverable 8 | 16 |
| isa-vectors | no CLI probe: staged deliverable 9 | 16 |
| isa-vectors | non-CLI fixture data (crate-level contract test, no executor command) | 14 |
| ladder-evidence | data symbol: no execution command (identity only) | 4 |
| ladder-evidence | lproc: reached only through its callers (§3.7) | 32 |
| vm-differential | no fixture argument (scratch or helper list) | 1 |
| vm-differential | non-literal argv element (computed in the test): &format! | 1 |
| vm-differential | non-literal argv element (computed in the test): &p | 13 |
| vm-differential | non-literal argv element (computed in the test): &pair | 2 |
| vm-differential | non-literal argv element (computed in the test): &path | 6 |
| vm-differential | non-literal argv element (computed in the test): &step.to_string | 2 |
| vm-differential | non-literal argv element (computed in the test): &tuple | 1 |
| vm-differential | non-literal argv element (computed in the test): cmd | 2 |
| vm-differential | non-literal argv element (computed in the test): command | 6 |
| vm-differential | non-literal argv element (computed in the test): if variant == 1 { "qreg:11" } else { "qr | 1 |
| vm-differential | non-literal argv element (computed in the test): match a.ty { ... | 2 |
| vm-differential | non-literal argv element (computed in the test): match p.ty { ... | 4 |
| vm-differential | non-literal argv element (computed in the test): match p.ty { ... | 1 |
| vm-differential | non-literal argv element (computed in the test): p.to_str | 1 |
| vm-differential | non-literal argv element (computed in the test): path | 2 |
| vm-differential | non-literal argv element (computed in the test): path.to_str | 4 |


### Findings

Interpreter and JIT never diverged from each other in behaviour: the only 6 `vm-int:vm-jit` DIVERGE cases are shared panics. The reference and the VM diverged as below. Lift rows and R runs
agreed on all seeds in the recorded run. These are claims about the sampled seeds and pinned commits, not proofs of agreement. Beads carry the witnesses in `harness/witnesses/`; the reference was judged right against spec text except where noted.

| bead | finding |
|---|---|
| bennett-vm-erb (P1) | oracle_call tgt/fr writes are lost after a later action_call on another register |
| bennett-vm-dwj (P1) | measurement at an exact CDF tie uses f64 weights (normative in 0.9 §4.1; on 0.8 objects a CLI convention) |
| bennett-vm-ffj (P1) | validator accepts reused SSA names, undeclared locations, bad regs roles, unmatched uncalls |
| bennett-vm-h6j (P1) | panic on r21 length fixtures |
| bennett-vm-jub (P2) | reverse reports equals_entry=false after segment uncall plus later control flow (seed 1165 of `--seed 1000`, outside the recorded range) |
| bennett-vm-af4 (P2) | refusal to run actions whose certificate binds a target: **owner open**, no spec sentence found either way |
| bennett-vm-syt (P2) | 6 neutral-suite diagnostics lack the pinned rule text |
| bennett-vm-5ye (P3) | budget-dependent execution of unlinked objects |

MATCH_TOLERANCE pairs came from f64 rotation amplitudes, for example `rx(pi)` giving -6.1232435e-17 against -6.1232340e-17.

## Limits

No claim of completeness; the sampled programs are bounded small. `harness/README.md`'s native-versus-compiled differential with sanitizer classifiers is not built (bennett-conformance-06p); nothing here covers it.
Self-tests are not in `scripts/pre-push` (bennett-conformance-68g).
