# Rung 4 gate design: Grover over unmodified SQLite

North Star §8 rung 4 is the acceptance text ([README.md](README.md)). This document fixes how each
clause is checked. It decides mechanics only. If a stage cannot pass today, the gate is red and a bead
in the owning repository says what must change ([RESULTS.md](RESULTS.md)). The gate's exit code is 0
only if every row of `STAGES` in [gate.py](gate.py) passes on both LLVM majors, diagnostics included.

## 1. What is fixed before anything runs

| Item | Value and check |
|---|---|
| SQLite | Stock 3.54.1 amalgamation, `pins.json`: URL, size, published SHA3-256, SHA-256 of the zip, of `sqlite3.c` and of `sqlite3.h`. All are verified on every run; a single changed byte fails "SQLite source unmodified". `sqlite3.c` is compiled as shipped; `shell.c` is never built or run. |
| Build flags | The eight documented options of `pins.json` (`SQLITE_THREADSAFE=0`, `SQLITE_OMIT_LOAD_EXTENSION`, `SQLITE_OS_OTHER=1`, `SQLITE_TEMP_STORE=3`, `SQLITE_DEFAULT_MEMSTATUS=0`, `SQLITE_OMIT_SHARED_CACHE`, `SQLITE_OMIT_DEPRECATED`, `SQLITE_DQS=0`), the spike's set (REPORT §2). They are recorded in the report and appear in every recorded argv. |
| Application-side code | `fixtures/os_shim.c`, which `SQLITE_OS_OTHER=1` requires of the application: the default VFS, no real files, fixed randomness and clock. `fixtures/oracle.c`: open, deserialize, prepare, bind, step, finalize, close. Nothing else stands between the query and SQLite. |
| Compilers | The unchanged musl sysroot from pinned `bennett-runtimes`, `clang -O2 -flto`, both majors. The published runtimes and arith bitcode, then `lto<O3>`, then the production pass (dialect §19.1). No host headers or host libc enter the device build. |
| Database | N rows from `splitmix64` inserted with ordinary `INSERT`s by `fixtures/host_build.c`, natively, on the same SQLite. `sqlite3_serialize` gives the file image; page size 4096. Images must be byte-identical across majors. This is the spike's route A with the image as a link-time global (A′): zero-padded to the smallest 2^A ≥ its length and XORed into a clean register by `__bennett_qprep`. Nothing runs on the database inside the oracle except SQLite. |
| Predicate | The real SQL of `fixtures/rung4.h`: `rowid = ?1` plus `LIKE`, `upper(substr())`, `IN`, `BETWEEN`, modular arithmetic. |
| Oracle interface | §16.7's shape on the real function: `rung4_oracle(img, len, x)`, `img` = `MEM_IN` of `region[A,8]` (quantum-addressed memory), `len` = `ARG`, `x` = `IN` zero-extended, rowid = x + 1, target 1 wire, fault 4 wires. Errors call `__builtin_trap()`, so they are faults, never a quiet "no match". |
| VFS | SQLite's own memdb over `img`, `SQLITE_DESERIALIZE_READONLY`. The region's role is `in`: it is restored on every basis configuration or the run fails. |
| Budget | `pins.json`: steps 1,000,000, heap 512 KiB, stack 4,194,304 cells, depth 64. These are margins over the spike's profile (2.9e5 rows, 182 KiB, 1.34M cells, depth 21), and the measured row-mapping may require more. Raising the budget is mechanics: it is recorded and changes the padded cost. A budget fault on any input is a red row, never a truncated result (NS §3.1). |
| Bead-independent oracle | `gate.py` restates the row generator and predicate in Python, sharing no code with SQLite. It must reproduce the spike's counts (6/256, 13/1024, 11,698/2^20). |

## 2. Sizes

| Clause | N | Bits | Why |
|---|---|---|---|
| (a) | 2^4 | 4 | M = 0: an oracle that never matches |
| (a) | 2^8 | 8 | M = 6 |
| (a) | 1000 | 10 | Not a power of two: rowids 1001..1024 must be no-match |
| (a) | 2^10 | 10 | M = 13, the rung's upper bound; A = 16, 40 KiB image |
| (b) | 2^4 | 4 | M = 0 negative control: one iteration leaves the state uniform; success 0 |
| (b) | 2^6 | 6 | M = 2, k = 4 |
| (b) | 2^8 | 8 | M = 6, k = 5, the rung's upper bound |
| (c) | 2^20 | 20 | M = 11,698, k = 7, A = 25, 33.5 MB image; a real compile of the real program, never extrapolated |

## 3. Pipeline stages

Shared (one result for both majors): fixtures; SQLite source unmodified; independent predicate; plugin,
VM, driver, estimator and checker builds from frozen commits (`pins.json`), each archive verified file by
file; images agree across majors. The reference is the accepted release binary, its hash recorded.

Per major, in order, each stage blocked by the stage it consumes:

1. **host image = native oracle = predicate.** Native `rung4_oracle` over every x, at every size, equals
   the Python table. This holds today, at N up to 2^20.
2. **musl C front end, runtime bitcode link, post-link LTO.** The program must still define the stock
   functions `sqlite3VdbeExec`, `memdbRead`, `sqlite3_deserialize`, `sqlite3BtreeTableMoveto` after LTO.
3. **runtime libc closure.** No undefined symbol other than `llvm.*` and markers. Dialect §4.5 makes a
   call without bitcode a `trapped` coverage line; SQLite executes these on every query (REPORT §5.2(iv)),
   so an unresolved libc routine is a fault, not a pass. Production still runs after a failure here, so
   later blockers are measured; everything that executes depends on this row.
4. **coherent production** on the whole program, per size. The plugin's analysis limits are explicit in
   `pins.json` and generous (1500 s); a limit reached at those values remains red.
5. **launch artefacts and oracle shape.** `host.bc`, `device.bc`, `launches.json`, per-entry artefacts,
   empty `blockers.json`, no `trapped` coverage line, verified bitcode, and the reference object
   `oracle @ref (x0:wire[n] in) (tgt:wire[1]) (fault:wire[4]) (m0:region[A,8] in) (a0:i64)` with at
   least 500 lprocs (the spike's closure is 1,218 functions). A reduced oracle cannot have this shape.
6. **materialised comparison = native** (§17.7): the materialised module linked with runtimes and arith,
   no host libc, gives the same bits as native at every (a) size.

Then for each executor (reference, VM): admission, then the (a) and (b) rows below.

## 4. Clause (a): the oracle, §5.1 part 1

**Domain.** NS §5.1 part 1: D = every x, every t, every e, every borrowed d, with clean ancillae, at the
fixed image. |D| = 2^(n + 1 + 4): 32,768 elements at n = 10. The program takes no `__bennett_qborrow`,
so d is empty. Target and fault entry values are enumerated, not reasoned away: "XOR delivery is
independent" (the string `step-check` prints) is the claim being tested, so it may not be its own
evidence, and the spike's 32× shortcut (one run per x) is not used.

**Pass means, for each of reference and VM, at each (a) size:**

1. **Oracle = native, fault rate zero.** `run-oracle` on `ref.bennett` for every x, with the image from a
   file: `code == 0`, `payload ==` native bit, `steps <=` budget. Measured minimum and maximum steps are
   recorded: the real B to compare with the spike's estimate.
2. **Full tuple on D.** Launch `rung4_tuple(x, t, e)` in `fixtures/grover.c`. It prepares the basis element
   and the database through markers, applies the oracle once, reads all interface wires by measuring the
   deterministic state, XORs the image back out and frees every register. Pass means status COMPLETED,
   no fault, no host request, launch closed, and the returned word exactly x | (t ⊕ f(x)) << n | e << (n+1),
   for all of D. A changed input bit, a misdelivered target or fault bit, or a dirty database register
   (`qreg.free` of an unclean register) fails.
3. **Literal step check.** `step-check --exhaustive` at the fixed image: for every x, the reference step
   machine agrees with the sequential semantics (T6) and every step inverts to the entry tuple (T2),
   workspace included. `mode` must be `exhaustive`, `cases` = 2^n, `fixed_regions` = `[m0]`. This row
   carries clean workspace; row 2 carries the entire interface tuple.

**Label, stated honestly.** Rows 1–3 are exhaustive over x, t, e at the fixed image: what North Star §8
rung 4(a) asks. ISA §12.2 (K74) labels a claim that fixes an `in` region as `evidence` with covered
fraction n/|D| where |D| counts all region cells. The dialect (§25.2) records this as an open
question for the ISA. So:

- **(a) certificate verdict** runs the independent checker on the launch certificate and requires
  `evidence`, elements = |D| as above, fraction exactly |D|/(|D|·2^(8·2^A)). A checker or certificate
  saying `proof` for this claim fails the gate.
- Nothing in this repository reports (a) as `proof`, and nothing here treats that label as a shortfall of
  the rung: §8's domain is the fixed database.

## 5. Clause (b): end-to-end Grover at N ≤ 2^8

**Program.** `rung4_grover()` in `fixtures/grover.c`, the quantum programmer's separate translation unit
(§16.7, §19.3): qalloc, qprep the image, H on the rowid register, target |−⟩, k × (oracle, diffusion),
measure, unprepare, free. The multi-controlled Z is a clean-ancilla CCX ladder. k comes from the
independent table, never from a run.

**Parameters** (closed form, M from the independent table, θ = asin √(M/N), k = ⌊π/(4θ)⌋,
P = sin²((2k+1)θ)):

| N | M | k | P(success) |
|---|---|---|---|
| 2^4 | 0 | 1 (control) | 0, uniform |
| 2^6 | 2 | 4 | 0.99918 |
| 2^8 | 6 | 5 | 0.98570 |

**Evidence kinds, each with its own row:**

- *Exhaustive oracle branches.* (a) above at the same N, on the same object family. This is what the
  branch enumeration of NS §4.2 executes.
- *(b) amplitudes: numerical, by state-vector simulation with branch enumeration.* The executor runs
  the program on its exact engine and dumps the state before the first measurement (§7). The gate
  compares the state to an independent array simulation from the truth table, in `gate.py`: one global
  phase, then max |Δamplitude| ≤ 1e-9 on every branch of the row and target registers. The database
  register must be one constant value equal to the image (unentangled); fault and ancilla registers
  must be 0 on every branch; no other basis state may have weight. P(success) is then checked against
  the closed form within 1e-9. The label is `numerical`, with tolerance and engine recorded. The dense
  engine (bennett-vm-bjm), when integrated, is an additional cross-check at these sizes, never a
  substitute for the reference's row.
- *(b) outcomes: deterministic draws.* Under ISA §14.2's draw rule the measured value is a function of
  the draws and the Born weights. For fixed draw vectors (all 0.5, all 0, and one selecting each marked
  x at the middle of its conditional interval), each executor's result must equal the value predicted
  from the independent simulation. M = 0 selects x = 0 and x = 2^n − 1 instead. This shows that the
  measured register, not only the dump, comes from those amplitudes.

No sampled frequency stands in for a probability, and no run with a smaller database stands in for N = 2^8.

## 6. Clause (c): estimate at N = 2^20

Input is the actual `rung4_grover` launch object, compiled with the real 33.5 MB image, A = 25 and
k = 7, not a scaled-up smaller N. Run `bennett-estimate <object> --target <file>` with the pinned named
target (`superconducting-grid-illustrative.json` from `bennett-targets`, whose numbers are illustrative,
said so in its provenance file). The output must name the target, carry a label (exact, bound, etc.),
bind the object's SHA-256, report the budget tuple equal to `pins.json`, and report the padded cost.
RESULTS.md quotes the label as it stands, including whether the figure is the reference construction
(REPORT §7: ~1.3e19 Toffoli per oracle call) or structure-recovered (~4e7, b-isa-spec-r51). The rung
asks for the padded cost under the budget; the gate does not require a small number, and does not
accept a small number with no sound label.

The M = 1 worst case (k = 804) is a closed-form scaling of the per-iteration cost; it is reported as such.

## 7. Consumer adapters (not spec text)

The specs fix semantics, not these spellings; equivalent real adapters are welcome. Each is measured by
a diagnostic row.

- **`--mem-file name=path`** on `run-oracle` and `step-check`: read a region's bytes from a file.
  Required by measurement: `--mem` carries hex on argv and Linux limits one argument to 128 KiB, so
  A = 16 (N = 2^10) already gets `E2BIG` (OSError 7), and A = 25 is 67 MB of hex.
- **`--state-out path`** on `run-program`: at the first executed measurement, write JSON
  `{"registers":[{"reg","width"}...],"constant":{"<reg>":"<hex>"},"amps":[{"basis":{"<reg>":"<uint>"},"re","im"}]}`.
  A register with one value on every branch may be in `constant`; basis values are decimal. Registers
  are numbered in `qreg.new` order. Without it the only readout is the measured value.
- **`bennett-check check <certificate> --object <object>`** prints JSON
  `{"verdict","elements","covered_fraction"}` for the claim on `ref`.
- **`bennett-estimate <object> --target <file>`** prints JSON with `target`, `label`, `object_sha256`,
  `budget`, `padded`.
- **Materialised comparison:** the rung-3 auditable `native-comparison.json` with `link_argv`,
  `--bennett-native-input` for `fixtures/native_compare.c`.

## 8. What each executor must show

| Row | Reference | VM (release, default engine) |
|---|---|---|
| admission | `check` on oracle and launches, at every size | same |
| oracle = native, fault rate zero | all x, all (a) sizes | same |
| full tuple on D | all (x, t, e), all (a) sizes | same |
| literal step check | exhaustive | same |
| amplitudes | N = 2^4, 2^6, 2^8 | same |
| outcomes | each draw vector | same |

The two executors are compared with the native result and the independent table, not only with each
other. Digests are never compared across implementations. Both LLVM majors must produce objects that
pass, since the rung has no single-major ruling (dialect §19.8 concerned front-end languages).

## 9. Controls that can never turn the gate green

- The published reduced `rung4-mini` object (§16.7), extracted from the pinned spec: admission, all
  published runs and its step check on both executors. It shows the executors and glue work. These rows
  are named diagnostic and never substitute for a required row.
- The published `rung4-grover-tu` through production, also diagnostic: the smallest witness for marker
  lowering gaps.
- Glue tests (`check_gate.py`) use synthetic values only and reject: hash mismatch; altered query text;
  the wrong interface shape; a closure below the size floor; a program without stock SQLite functions;
  trapped coverage; wrong target or fault delivery; non-exhaustive step checks; wrong k; wrong marked
  set; garbage in fault, ancilla or database registers; a `proof` label on the fixed-region claim;
  an estimate of a different object.

## 10. Narrowings this design rules out

Each of these would turn the gate green and each is forbidden: a smaller or hand-written "database"
engine; a query precomputed into a table; omitting t or e from D; sampling x at N ≤ 2^10; removing
reachable floating point or varargs from the build by editing the amalgamation; treating a `trapped`
libc call as acceptable because a sample did not reach it; pre-evaluating SQLite at build time in place
of a lifted oracle; lowering the budget's fault rows to warnings; extrapolating (c) from a small N;
the host-libc baseline as the only native reference once the §17.7 comparison exists; relabelling
(a) as `proof`; a dense-engine run without the exact-engine row.

## 11. Evidence

Each run writes `.tmp/rung-4/runs/<UTC>/`: `report.json` (argv, exits, durations, hashes, pins, stage
errors, per-clause numbers and labels), `table.md`, per-command stdout and stderr, JSONL per sweep. `--record` copies
the report, table and a SHA-256 inventory of every raw file to `evidence/<UTC>/`. Running the whole
acceptance needs hours of CPU once the pipeline is open (REPORT §7: order 2e10 rows at N = 2^10 per
executor); `--jobs` bounds concurrency.
