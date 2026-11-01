# Independent certificate checker

`checker/` is the Rust crate `bennett-cert-checker`. It implements certificate
checking from the published Bennett ISA 0.9 contract, without importing or
copying the reference interpreter, VM or plugin implementation. General-purpose
integer/rational arithmetic and ECMAScript number rendering are dependencies;
JSON parsing, domain resolution, hashing, replay and certificate reasoning live
in this crate.

Legacy certificates, including the published 0.8 compatibility object, are
reported unchecked until their version-specific contract is implemented.

The implementation is in progress. The published `controlled-action-09`
fixture now receives a complete **verified** verdict: its full-tuple exhaustive
claim, both numerical claims, native target feasibility, resources, gate counts,
invocation multiplicities and error composition are checked independently.
This is a verified fixture path, not a claim that every §12 capability has been
implemented. Unsupported obligations still produce explicit unchecked results.

## Running

```sh
cargo run --offline --manifest-path checker/Cargo.toml --bin bennett-check -- \
  certificate.json --object program.bisa --target target.json --schedule schedule.json

cargo run --offline --manifest-path checker/Cargo.toml --bin bennett-check -- \
  --from-object program.bisa --target target.json
```

An omitted `--schedule` uses the object's embedded schedule, if any. A standalone
reference table/program relation does not need a schedule. Explicit documents
are checked against the certificate bindings. The object hash excludes both
schedule and certificate sections as §9.2 requires; changing the schedule still
changes its separate binding.

`--max-elements N` bounds replay and native expansion work. `--max-wires N`
bounds matrix work; the current dense rational implementation also has a
six-wire host cap. Channel LDL work has an eight-dimensional interface cap,
and interval precision has a 4096-bit host cap. These are implementation
limits, never ISA legality limits. Retained replay/table/coverage storage is
also bounded by a conservative 256 MiB estimate before allocation. `--library` accepts additional JCS documents;
independent library-proof provenance is still unchecked. `--oracle-bin` is
reserved for differential replay and explicitly reports that the optional
comparison has not been performed.

The executable prints exactly one canonical JSON verdict line on stdout.
Usage and file/UTF-8 input errors go to stderr. Exit status is:

| Status | Meaning |
|---|---|
| 0 | Every represented obligation was checked successfully |
| 1 | At least one certificate obligation was rejected |
| 2 | At least one obligation remains unchecked, without a rejection |
| 3 | Usage or file input error |

Output has `overall`, recomputed `hashes`, global `errors`, `establishes`,
`unchecked_items`, and `regions` keyed by closure hash. Each region has errors,
unchecked obligations and claims in their original zero-based order. Diagnostics
contain `code`, `section`, `path` and a concrete message. Output has no timestamps,
random IDs or execution-time fields. Byte-identical inputs yield identical JSON.

## What a claim establishes

| Claim outcome | Established scope |
|---|---|
| `proof` | Exhaustive agreement on every component of D, or both inverse-pair compositions over the entry's full input/memory domain and declared classical-argument ranges |
| `evidence` | Agreement only on the stated slice or sampled draws, with exact replay count and distinct covered fraction |
| `accept` | The named numerical/channel bound or syntactic structural rule at the resolved invocation scope |
| `reject` | A named obligation failed; a diagnostic gives its witness or malformed field |
| `unchecked` | No conclusion; a reason identifies the missing mechanism or host limit |

For exhaustive and sampled relations, complete tuples include inherited controls,
all interface bundles (including arbitrary initial oracle target and fault
contents), all borrowed positions and every interface region cell. Omissions
are zero-filled slices, never full coverage. Components resolve by role/position,
not a union of interface and borrowed names. Expected programs correspond by
signature position, allowing different local names. Inputs and borrowed systems
must be preserved, workspace must finish clean, and permutation replay admits
no phase approximation. A producer's requested proof label is rejected if the
completed check yields only evidence. Sampled success stays evidence even at
full distinct coverage. Host-limited claims have no evidence count or fraction.

Numerical checking constructs the realised and reference matrices at every
resolved native action context, controls first in allocation order. It computes
interval enclosures of `V†U-I` and the Frobenius upper bound, with outward binary
rounding after arithmetic operations. Channel checking constructs the
unnormalised, input-first Choi difference and verifies all three dual inequalities
by interval Hermitian LDL factorisation. Every pivot must have a strictly
positive lower endpoint; zero, negative and inconclusive pivots reject. The
reported bound is the diamond norm itself, not half that norm.

The translation lemma checks the actual X-family circuit, including effective
controls, the exact `V;W;reverse(V);W` shape, live borrowed wires, all-W-control
exclusion and maximality. Its conclusion concerns that reference borrow frame
on arbitrary borrowed states, including entanglement with an external reference.
It does not establish another frame's entry domain or a target realisation.

## Current implementation boundary

Reference replay covers an independently implemented subset of lifted arithmetic
and permutation-action semantics. Unsupported rows, lproc/stack machinery and
unresolved invocation contexts are explicit unchecked outcomes. The native
matrix path currently handles straight-line, single-class action calls and
X/CX/CCX/MCX/SWAP/H/S/S†/T/T†/CZ gates. It includes invocations covered entirely
by defaults, forwards classical arguments, and checks maximal gate selector
specificity and ties. Scheme expansion, rotations, topology, multi-class
allocation and complete oracle expansion remain unchecked.

The checker does not infer exactness from the mere presence of an epsilon-zero
target entry. It checks the selected native entry with the actual total control
count. A scheme built from exact leaves is not assumed to realise its advertised
operation. Nonzero native epsilon alone does not specify the actual operator;
the missing realisation-model binding is recorded as a spec question.

`adjoint_table` and independent library correspondence still need complete proof
linkage. Verification-index shapes do not discharge borrow-frame entry-state or
qinplace-entry-applicability obligations. Source coverage and expanded non-native
accounting remain explicitly unchecked where they have not been independently
recomputed. Malformed shapes
are still rejected before claiming support.

For exact native straight-line realisations with one context per region, the
checker now recomputes scheduled leaf timing, all declared resource values,
gate counts, capacity and predicate limits, invocation multiplicities, and
local/inclusive error recurrence. Target metadata is validated through its
nested closed schemas. Gate costs distinguish own controls `k` from total
controls `c`; adjoint calls match and count the inverse primitive (`tdg`, not
`t`, for an adjointed T). Symbolic source-slot widths are evaluated at each
resolved inherited-control count.

Launch estimates on this path use exact reduced numerator/denominator rows,
percent-encoded resource/class names and canonical upward 34-digit decimal
strings. A `1/3`-second regression checks exact rational accounting separately
from display rounding and rejects wrong/unreduced fractions or rounded-down
values.

Repeated contexts can have different widths, timings and argument-dependent
costs for one closure. The current §12.3 region resource maps are unkeyed and
do not specify how those contexts aggregate. Such reports remain unchecked
pending the staged spec proposal for explicit context resource records; no
sum, maximum or arbitrary invocation is silently selected. Numerical checks
still enumerate every context. Overlapping event lifetime accounting also
remains unchecked.

## Diagnostics and verification

`E_CERT_*` codes are this checker's stable diagnostics; §12 does not publish a
checker-code registry. Important families include `JSON`, `JCS`, `SCHEMA`,
`VERSION`, `OBJECT`, `HASH`, `BINDING`, `INVOCATION`, `DOMAIN`, `TABLE`, `REPLAY`,
`SAMPLER`, `COVERAGE_LABEL`, `FRAME`, `VERIFICATION_INDEX`, `NUMERICAL`, `CHANNEL`,
`STRUCTURAL`, `RESOURCE`, `GATE_COUNT`, `ESTIMATE`, `MULTIPLICITY` and
`COMPOSITION`. Schedule decision ties use the contract's
`E_SCHEDULE_INFEASIBLE` code.

```sh
env -u DISPLAY -u WAYLAND_DISPLAY -u DBUS_SESSION_BUS_ADDRESS -u BROWSER \
  TMPDIR="$PWD/checker/.tmp/tmp" CARGO_TARGET_DIR="$PWD/checker/.tmp/target" \
  timeout 180 cargo test --offline --manifest-path checker/Cargo.toml -j4 -- --test-threads=4
```

Tests include the published hash witnesses, full component-coverage replay,
quantified inverse pairs, numerical/channel checks, structural negatives and
CLI determinism. The certificate field mutation sweep visits all **250** object
fields and array positions of the published positive fixture, replacing each
with an invalid type and requiring its individually specified diagnostic code.
A second sweep mutates all **43** fields and positions of the published target,
rebinding its hashes before checking and requiring `E_CERT_TARGET`. Neither
sweep samples fields. These invalid-type checks are one part of mutation
coverage; they do not claim that every legal metadata edit must be rejected.

JCS tests include all finite number samples from
[RFC 8785 Appendix B](https://www.rfc-editor.org/rfc/rfc8785.html#appendix-B),
UTF-16 property sorting and invalid surrogate rejection. The binary64
`43143ff3c1cb0959` tie is a regression for the standard Rust formatter's different
choice, which is why number rendering uses `ryu-js`.
