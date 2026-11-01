# Rung 1: ordinary scalar C, Rust and Julia

North Star §8: **exhaustive at Int8 over the full domain D of §5.1 part 1,
both executors equal to native; fault rate zero**. Byte-exact reversal is an
additional required row. A known blocker is always red; exit 0 requires every
applicable stage, language and major to pass. N/A is a separate status, counted
neither as a pass nor as a failure and ignored by the exit code. No language
or input is silently skipped.

Orchestrator ruling 2026-11-01 (closed `bennett-conformance-oc2`): North Star
§8 requires both executors equal to native, not both LLVM majors per language.
Dialect 0.2.10 §19.8, “An input newer than M is BD-E-VERSION”, accepts older
IR and forbids newer IR. The installed stock frontend matrix is:

| Language | LLVM 18 | LLVM 23 |
|---|---|---|
| C (same-major clang) | applicable | applicable |
| Rust 1.99.1 (LLVM 23.1.1 IR) | N/A · §19.8 | applicable |
| Julia 1.13.3 (LLVM 20.1.8 IR) | N/A · §19.8 | applicable |

The gate detects applicability from each emitted bitcode's LLVM producer
identification record using `llvm-bcanalyzer`, rather than a language/version
lookup. Julia's unchanged `code_llvm` text is additionally serialized using
that Julia process's own loaded LLVM library to retain its producer version;
the pipeline still consumes the original text. Unknown producers fail loudly.
Stages through frontend emission retain their real results; downstream cells
with newer IR read `N/A · §19.8 (frontend IR <version> newer than LLVM <M>)`.
There is no IR downgrade and no language is dropped. Applicable Rust/23 and
Julia/23 cells retain plugin blockers `bp-xb8` and `bp-zeg`; `bp-75y` remains
the materialised-comparison/full-D frontier.

Run headlessly from any directory:

```sh
/home/tobias/Projects/bennett-workspace/bennett-conformance/ladder/rung-1/gate.sh
```

`pins.json` freezes the plugin's pushed main `157d4bd`, VM's pushed main
`13f5a9e`, dialect 0.2.10 `023f5a7`, ISA 0.8 reference release `ba7d13f`,
runtimes `6808622`, and arith publication `249e619`. Plugin sources are archived
and byte-verified before building; VM builds in a clean detached sibling
worktree with `cargo build --release`, never the possibly dirty live source. Published runtime/arith archives
are extracted from their pinned commits; the arith digest must also match the
plugin's SPEC_VERSIONS.toml. Use `--arith18 /path/to/pinned.bc --arith23 ...`
for explicit extracted publications; the SHA checks still apply. `--plugin-rev`,
`--vm-rev`, `--reference`, `--runtimes18/23`, `--output`, and `--jobs` support
explicit reruns. Corpus commands have a 300-second host limit; `--timeout` may
only lower it. Build limits are recorded separately.

The five source functions in each language are saturating add, Euclidean gcd,
popcount, a branchy sign/absolute-value mix, and an eight-entry lookup. Each
has Int8 arguments and an Int8 result. Gcd of magnitudes 128 is represented by
the Int8 bit pattern 0x80; no signed overflow or zero divisor is evaluated.
The sources contain no Bennett declarations, annotations or marker calls.
Native harnesses and the Julia `code_llvm` emitter are separate fixture files.
Every function carries valid dialect/ISA clause IDs in `fixtures/expect.json`;
all fixtures, including harnesses and expectations, have pinned SHA-256 hashes.
Budgets are 256 steps / 0 heap / 32 stack / depth 32, except popcount, whose
linked bit-loop helper receives 1024 steps and 512 stack cells. These cover the
complete source domains; budget exhaustion is never an accepted outcome.

The input order is unsigned Int8 encoding 0..255 (signed values 0..127,
-128..-1), last argument fastest. Binary functions have 65,536 cases and unary
functions 256: **131,840 per language**. Each frontend's O0 and O2 native
executables must agree byte for byte, with no excluded inputs. The linked
materialised comparison is a separate check: same-major clang, the actual
`materialised.bc`, pinned runtimes and arith, no host libc/libm, no unresolved
symbols, static non-PIE text at 0x400000 and scalar constant data at 0x1000.
The lookup's published global base is checked against the native symbol map.
A host unable to run that fixed-address ELF is a blocker, never a reason to
move the comparison data or replace the materialised program with the source.

The pipeline follows dialect §§19.1/17.7: clang `-O2 -flto`, Rust
`--emit=llvm-bc`, Julia `code_llvm(raw=true, dump_module=true, optimize=true)`;
link major-matched runtimes and SHA-pinned arith, `lto<O3>`, then the plugin.
The source functions are selected via the scalar oracle route, not by inserting
markers into them. The Julia specialised function's ordinary implicit gcstack
parameter remains visible as an `ARG(64)` binding before the source Int8 inputs; the gate never strips its GC/TLS prelude. This
currently exposes an entry-state/ABI adapter frontier in the plugin. Newer
Rust/Julia IR is never rewritten to appease LLVM18 (§19.8).

**Full D is larger than the source inputs.** The scalar oracle has an 8-bit
target, a 4-bit fault register, no borrowed dirty ancillae, and initially clean
internal workspace. Thus each binary D contains 268,435,456 tuples and each
unary D 1,048,576: **540,016,640 tuples per language/major/executor**. Constant
globals are fixed by the declared source instance. D covers all input, target
and fault values, preserves inputs, checks target XOR native result and fault
XOR zero, checks phase and no residual workspace, then reverses byte-exactly.
No sampling or target/fault structural shortcut counts as exhaustion.

`full_domain_worker.py` uses a public ISA action adapter around the unchanged
emitted oracle and its unchanged closure hashes. The adapter's content hash is
computed using ISA §9.2; both executors independently admit it. It exposes every
interface wire using `reverse`'s forward and restored sparse basis states.
Input-only diagnostics use the oracle retained-history reversal CLI and compare
its full machine snapshot (including memory), clean inverse frames and native
payload on every source input. **Those diagnostics cannot pass full-D rows.**
The full-D corpus starts only after all functions have native comparisons,
admission, successful input diagnostics and admitted adapters. Each language
row is the conjunction of its functions; individual results remain in report.json.

Every invocation owns `.tmp/rung-1/<UTC timestamp>/`. `report.json` records pins,
versions, exact argv, status, command limits/timings, hashes, per-function rows
and propagated bead IDs. Per-command stdout/stderr and per-input JSONL retain
results and exact executor commands; partial summaries survive a timeout.
`table.md` is the stage table, and `.tmp/rung-1/latest` points to the latest
completed report. [RESULTS.md](RESULTS.md) records the real run;
[blockers.json](blockers.json) lists measured blockers in pipeline order.

Gate glue checks (never corpus acceptance):

```sh
TMPDIR=/home/tobias/Projects/bennett-workspace/bennett-conformance/.tmp python3 ladder/rung-1/check_gate.py
```

Rung 1 reuses `ladder/common/gate_support.py` for command evidence, failures and
red dependency propagation. Rung 2 and rung 3 source and behaviour are unchanged.

If an observed failure needs a new bead after a run, `classify_report.py
/path/to/report.json` attaches the current blocker mappings without changing any
measured status or command. It preserves `report.unclassified.json` and records
the raw report hash, mapping hash, UTC time and exact classification command.
Unfiled failures make classification fail loudly. This is annotation of that
same real run, not a rerun or a conversion of a failure to a pass.
