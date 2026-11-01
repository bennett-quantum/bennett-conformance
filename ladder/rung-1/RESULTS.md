# Rung 1 real run — 20261101T135131.872696Z

**Gate exit 1: full North Star acceptance is blocked.** No missing language, input, target or fault value is counted as a pass.

Raw evidence: `.tmp/rung-1/20261101T135131.872696Z/`. Compact durable evidence: [evidence/20261101T135131.872696Z](evidence/20261101T135131.872696Z/). `report.json` contains every exact command, version, SHA, timing, per-function status and dependency. Per-input JSONL stays under the raw path; its sizes and SHA-256 hashes are committed in `raw-input-evidence.json`.

Toolchains found: clang 18.1.3 and 23.1.5; rustc 1.99.1 (LLVM 23.1.1); Julia 1.13.3 (`code_llvm`, LLVM 20.1.8). No executable was silently skipped. Rust and Julia frontends execute on both requested paths, but their newer stock IR cannot be read by LLVM18 (§19.8); those link stages FAIL with a conformance toolchain bead.

Pins: plugin `157d4bd1fab98afed8579b2367f26d1d742b0173`, detached release VM `13f5a9ed7482b8645c2e37e2ed95e0a7df9dfa88`, reference release `ba7d13f76e73c2af4e035d3f51145d2fad76bd2f`, runtimes dist `6808622d560b2e301c635c205eecde3d932b6df5`, arith dist `249e6192b95c6dbbb875510c32a677ba113e9af5`, dialect 0.2.10 `023f5a771a814270634db3be9cfde88ded69f975`, ISA 0.8. The gate/worker content hashes match implementation commit `3f5c0e8`. The gate extracts pinned publications, verifies arith against the plugin pins, and sets `BENNETT_ARITH_BITCODE` for each plugin command. Concurrent sibling source/dist edits are not used.

Each language has two binary and three unary functions: 131,840 source inputs. All six native O0/O2 pairs agree, with identical combined output SHA-256 `48a33b8e8a5683095ec6c5684c33bfccb76b90cff2c0d68661ede60d55e8e0c8`; no inputs are excluded. Full D additionally ranges over all 256 target and 16 fault values: 540,016,640 tuples per language/major/executor, with no borrowed dirty wires and initially clean workspace. It is **not** replaced by the input-only diagnostics below.

## Stage table

| Language / stage | LLVM 18 | LLVM 23 |
|---|---|---|
| C / toolchain | PASS | PASS |
| C / fixtures | PASS | PASS |
| C / plugin build | PASS | PASS |
| C / VM build | PASS | PASS |
| C / native O0 = O2 | PASS | PASS |
| C / frontend | PASS | PASS |
| C / runtime bitcode link | PASS | PASS |
| C / post-link LTO | PASS | PASS |
| C / pipeline | PASS | PASS |
| C / materialised native comparison | FAIL · bp-75y | FAIL · bp-75y |
| C / reference admission | PASS | PASS |
| C / VM admission | PASS | PASS |
| C / reference input exhaustive = native (diagnostic) | PASS | PASS |
| C / VM input exhaustive = native (diagnostic) | PASS | PASS |
| C / reference input zero faults (diagnostic) | PASS | PASS |
| C / VM input zero faults (diagnostic) | PASS | PASS |
| C / reference input byte-exact reversal (diagnostic) | PASS | PASS |
| C / VM input byte-exact reversal (diagnostic) | PASS | PASS |
| C / reference full D interface | PASS | PASS |
| C / VM full D interface | PASS | PASS |
| C / reference exhaustive full D = native | BLOCKED · bp-75y | BLOCKED · bp-75y |
| C / VM exhaustive full D = native | BLOCKED · bp-75y | BLOCKED · bp-75y |
| C / reference zero faults on D | BLOCKED · bp-75y | BLOCKED · bp-75y |
| C / VM zero faults on D | BLOCKED · bp-75y | BLOCKED · bp-75y |
| C / reference byte-exact reversal on D | BLOCKED · bp-75y | BLOCKED · bp-75y |
| C / VM byte-exact reversal on D | BLOCKED · bp-75y | BLOCKED · bp-75y |
| Rust / toolchain | PASS | PASS |
| Rust / fixtures | PASS | PASS |
| Rust / plugin build | PASS | PASS |
| Rust / VM build | PASS | PASS |
| Rust / native O0 = O2 | PASS | PASS |
| Rust / frontend | PASS | PASS |
| Rust / runtime bitcode link | FAIL · bennett-conformance-oc2 | PASS |
| Rust / post-link LTO | BLOCKED · bennett-conformance-oc2 | PASS |
| Rust / pipeline | BLOCKED · bennett-conformance-oc2 | FAIL · bp-xb8 |
| Rust / materialised native comparison | BLOCKED · bennett-conformance-oc2 | BLOCKED · bp-xb8 |
| Rust / reference admission | BLOCKED · bennett-conformance-oc2 | BLOCKED · bp-xb8 |
| Rust / VM admission | BLOCKED · bennett-conformance-oc2 | BLOCKED · bp-xb8 |
| Rust / reference input exhaustive = native (diagnostic) | BLOCKED · bennett-conformance-oc2 | BLOCKED · bp-xb8 |
| Rust / VM input exhaustive = native (diagnostic) | BLOCKED · bennett-conformance-oc2 | BLOCKED · bp-xb8 |
| Rust / reference input zero faults (diagnostic) | BLOCKED · bennett-conformance-oc2 | BLOCKED · bp-xb8 |
| Rust / VM input zero faults (diagnostic) | BLOCKED · bennett-conformance-oc2 | BLOCKED · bp-xb8 |
| Rust / reference input byte-exact reversal (diagnostic) | BLOCKED · bennett-conformance-oc2 | BLOCKED · bp-xb8 |
| Rust / VM input byte-exact reversal (diagnostic) | BLOCKED · bennett-conformance-oc2 | BLOCKED · bp-xb8 |
| Rust / reference full D interface | BLOCKED · bennett-conformance-oc2 | BLOCKED · bp-xb8 |
| Rust / VM full D interface | BLOCKED · bennett-conformance-oc2 | BLOCKED · bp-xb8 |
| Rust / reference exhaustive full D = native | BLOCKED · bennett-conformance-oc2 | BLOCKED · bp-xb8 |
| Rust / VM exhaustive full D = native | BLOCKED · bennett-conformance-oc2 | BLOCKED · bp-xb8 |
| Rust / reference zero faults on D | BLOCKED · bennett-conformance-oc2 | BLOCKED · bp-xb8 |
| Rust / VM zero faults on D | BLOCKED · bennett-conformance-oc2 | BLOCKED · bp-xb8 |
| Rust / reference byte-exact reversal on D | BLOCKED · bennett-conformance-oc2 | BLOCKED · bp-xb8 |
| Rust / VM byte-exact reversal on D | BLOCKED · bennett-conformance-oc2 | BLOCKED · bp-xb8 |
| Julia / toolchain | PASS | PASS |
| Julia / fixtures | PASS | PASS |
| Julia / plugin build | PASS | PASS |
| Julia / VM build | PASS | PASS |
| Julia / native O0 = O2 | PASS | PASS |
| Julia / frontend | PASS | PASS |
| Julia / runtime bitcode link | FAIL · bennett-conformance-oc2 | PASS |
| Julia / post-link LTO | BLOCKED · bennett-conformance-oc2 | PASS |
| Julia / pipeline | BLOCKED · bennett-conformance-oc2 | FAIL · bp-zeg |
| Julia / materialised native comparison | BLOCKED · bennett-conformance-oc2 | BLOCKED · bp-zeg |
| Julia / reference admission | BLOCKED · bennett-conformance-oc2 | BLOCKED · bp-zeg |
| Julia / VM admission | BLOCKED · bennett-conformance-oc2 | BLOCKED · bp-zeg |
| Julia / reference input exhaustive = native (diagnostic) | BLOCKED · bennett-conformance-oc2 | BLOCKED · bp-zeg |
| Julia / VM input exhaustive = native (diagnostic) | BLOCKED · bennett-conformance-oc2 | BLOCKED · bp-zeg |
| Julia / reference input zero faults (diagnostic) | BLOCKED · bennett-conformance-oc2 | BLOCKED · bp-zeg |
| Julia / VM input zero faults (diagnostic) | BLOCKED · bennett-conformance-oc2 | BLOCKED · bp-zeg |
| Julia / reference input byte-exact reversal (diagnostic) | BLOCKED · bennett-conformance-oc2 | BLOCKED · bp-zeg |
| Julia / VM input byte-exact reversal (diagnostic) | BLOCKED · bennett-conformance-oc2 | BLOCKED · bp-zeg |
| Julia / reference full D interface | BLOCKED · bennett-conformance-oc2 | BLOCKED · bp-zeg |
| Julia / VM full D interface | BLOCKED · bennett-conformance-oc2 | BLOCKED · bp-zeg |
| Julia / reference exhaustive full D = native | BLOCKED · bennett-conformance-oc2 | BLOCKED · bp-zeg |
| Julia / VM exhaustive full D = native | BLOCKED · bennett-conformance-oc2 | BLOCKED · bp-zeg |
| Julia / reference zero faults on D | BLOCKED · bennett-conformance-oc2 | BLOCKED · bp-zeg |
| Julia / VM zero faults on D | BLOCKED · bennett-conformance-oc2 | BLOCKED · bp-zeg |
| Julia / reference byte-exact reversal on D | BLOCKED · bennett-conformance-oc2 | BLOCKED · bp-zeg |
| Julia / VM byte-exact reversal on D | BLOCKED · bennett-conformance-oc2 | BLOCKED · bp-zeg |

## Individual functions

P = PASS; F = FAIL; B = BLOCKED. R/V cells show reference / VM. “Inputs” includes native payload equality, zero faults and byte-exact retained-history reversal over the complete source-input product. Full-D acceptance remains blocked on corpus prerequisites.

| Language / major / function | Pipeline | Materialised native | Admission R/V | Inputs R/V | Full-D interface R/V |
|---|---|---|---|---|---|
| C/23 / r1_sat_add | P | P | P/P | P/P | P/P |
| C/23 / r1_gcd | P | P | P/P | P/P | P/P |
| C/23 / r1_popcount | P | P | P/P | P/P | P/P |
| C/23 / r1_sign_abs | P | P | P/P | P/P | P/P |
| C/23 / r1_lookup | P | F | P/P | P/P | P/P |
| Rust/18 / r1_sat_add | B | B | B/B | B/B | B/B |
| Rust/18 / r1_gcd | B | B | B/B | B/B | B/B |
| Rust/18 / r1_popcount | B | B | B/B | B/B | B/B |
| Rust/18 / r1_sign_abs | B | B | B/B | B/B | B/B |
| Rust/18 / r1_lookup | B | B | B/B | B/B | B/B |
| C/18 / r1_sat_add | P | P | P/P | P/P | P/P |
| C/18 / r1_gcd | P | P | P/P | P/P | P/P |
| C/18 / r1_popcount | P | P | P/P | P/P | P/P |
| C/18 / r1_sign_abs | P | P | P/P | P/P | P/P |
| C/18 / r1_lookup | P | F | P/P | P/P | P/P |
| Julia/18 / r1_sat_add | B | B | B/B | B/B | B/B |
| Julia/18 / r1_gcd | B | B | B/B | B/B | B/B |
| Julia/18 / r1_popcount | B | B | B/B | B/B | B/B |
| Julia/18 / r1_sign_abs | B | B | B/B | B/B | B/B |
| Julia/18 / r1_lookup | B | B | B/B | B/B | B/B |
| Julia/23 / r1_sat_add | F | B | B/B | B/B | B/B |
| Julia/23 / r1_gcd | F | B | B/B | B/B | B/B |
| Julia/23 / r1_popcount | F | B | B/B | B/B | B/B |
| Julia/23 / r1_sign_abs | F | B | B/B | B/B | B/B |
| Julia/23 / r1_lookup | F | B | B/B | B/B | B/B |
| Rust/23 / r1_sat_add | P | P | P/P | P/P | P/P |
| Rust/23 / r1_gcd | F | B | B/B | B/B | B/B |
| Rust/23 / r1_popcount | P | P | P/P | P/P | P/P |
| Rust/23 / r1_sign_abs | P | P | P/P | P/P | P/P |
| Rust/23 / r1_lookup | F | B | B/B | B/B | B/B |

## What the run establishes

All C functions produce admitted ISA objects on LLVM18 and LLVM23. The input diagnostics test every Int8 argument tuple on both executors against the independently compiled native outputs; inspect the per-function table for any failed checks. A successful input diagnostic also verifies zero fault code, full machine snapshot equality after reversal (including memory), no first difference and clean inverse frames. The action adapter exposes and checks an arbitrary boundary input/target/fault tuple, including exact phase and restoration; an interface probe does not establish exhaustive D.

Four C functions have runnable, successful §17.7 materialised comparisons; lookup’s fixed data address 4096 is below this host’s `vm.mmap_min_addr=65536`, so its native comparison fails. All five C functions pass 131,840 source inputs per executor/major, zero faults and byte-exact reversal. The actual lookup ISA/input diagnostics still run independently. Rust/23 retains its individual successes; gcd and lookup fail at V3 after a residual `personality ptr @rust_eh_personality`. Julia/23 reaches the post-link plugin but all five specialised entries fail at `ptr %pgcstack_arg`, before later runtime capabilities can be established. No runtime, reference or VM defect is invented for an unexecuted downstream frontier.

## Ordered blockers

1. **bennett-conformance-oc2** — `bennett-conformance`; runtime bitcode link (major compatibility). Exact determining contract sentences, instruction, commands, evidence and completion criteria are in [blockers.json](blockers.json).
2. **bp-xb8** — `bennett-plugin`; pipeline (Rust EH materialisation). Exact determining contract sentences, instruction, commands, evidence and completion criteria are in [blockers.json](blockers.json).
3. **bp-zeg** — `bennett-plugin`; pipeline (Julia entry state). Exact determining contract sentences, instruction, commands, evidence and completion criteria are in [blockers.json](blockers.json).
4. **bp-75y** — `bennett-plugin`; materialised native comparison / layout option. Exact determining contract sentences, instruction, commands, evidence and completion criteria are in [blockers.json](blockers.json).

Rung acceptance bead: **bennett-conformance-px1**, blocked. Upstream source is untouched by this task; sibling changes are Beads export commits only.

## Consumer verification and superseded discovery

An independent read of all 26 per-input logs verifies complete, duplicate-free argument products, native payload equality, zero fault codes and byte-exact reversal for 659,456 executions; see `coverage-audit.log`. Full D remains blocked.

Rung-1 glue checks: eight pass. Rung-2: all five driver checks pass against the real executors. Rung-3: all eight glue checks pass. `ladder/rung-2`, `ladder/rung-3` and `ladder/common` have no changes from this task. The required `scripts/pre-push` is run once on the final recorded commit before handback.

Earlier discovery records are not acceptance evidence: an initial consumer budget exceeded the scalar layout; a later 32-cell stack was too small for the linked popcount helper. Popcount now has 1024 steps and 512 stack cells, while the other fixtures use 256 steps and 32 stack cells; all reserve zero heap and depth 32. The native harness now fixes only published globals at the device address, leaving unobservable compiler-generated constants at ordinary linker addresses. These were gate mistakes, fixed locally without upstream defect beads or dropping popcount. A focused preflight verified every popcount input on both majors and both executors before this final full invocation. The debug-VM run was superseded by a detached release build.

Native comparison code is freestanding harness glue around the actual `materialised.bc` plus pinned runtimes and arith, never a replacement for the source functions. It uses no host libc/libm, checks unresolved symbols, and validates the lookup global’s native address against the emitted layout. Every corpus command has the recorded 300-second outer host limit; no LLVM view pass was used.

## Sibling bead commits

`bennett-plugin`: `d264830` (Rust/Julia refusals), `993560b` (comparison-layout option), `b85546a` (Julia ABI binding clarification). The current sibling HEAD can include concurrent source work; the gate continues to use the frozen pushed-main pin above. This repository’s implementation/evidence commits and final HEAD are reported at handback. No Git push or Dolt remote sync was performed.
