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
six-wire host cap. Event/resource tracing uses a separate bound of at most
1,048,576 wire identities and can verify wide interfaces without matrix work.
Channel LDL work has an eight-dimensional interface cap,
and interval precision has a 4096-bit host cap. These are implementation
limits, never ISA legality limits. Structured source nesting above 64
levels is detected by a flat scan before recursive AST analysis and returns
an unchecked host limit. Retained replay/table/coverage storage is
also bounded by a conservative 256 MiB estimate before allocation. `--library` accepts additional JCS documents;
independent library-proof provenance is still unchecked. `--oracle-bin` is
reserved for differential replay and explicitly reports that the optional
comparison has not been performed.

Native source expansion currently permits at most 100,000 expanded rows and
64 loop/invocation levels. Exact decimal materialisation permits at most
100,000 significand digits and an absolute decimal scale of 100,000. Oversized
decimal fields produce an explicit host-limit verdict before constructing
powers of ten; exponents never truncate into machine-sized powers. This
preflight applies to decimal fields, including embedded schedule fields, and
leaves names and other opaque strings untouched.

Every object metadata section must itself contain JCS bytes. The §9.1 rule
rejecting JSON numbers at or above 2^53 applies recursively to metadata in
objects and supplied certificate, target and schedule documents. Signature
widths, region A/W, bounds and literal frame widths accept canonical integer
strings and retain their exact value; host integer conversions are checked.
Quantum interface sizes beyond u64 are compared with their source declarations
before returning a host limit. A source/signature mismatch still rejects.
Exact decimal
strings retain their field-specific interpretation and are not rounded into
JSON numbers.

Reference replay has a separate default limit of 1,048,576 work steps per
evaluation, shared through nested oracle and action calls. Oracle rows, action
instructions and loop visits consume work; an embedding can set
`Limits.max_reference_steps`. Each oracle still uses its own ISA budget. A
host work limit gives no proof label, evidence count or coverage; ISA budget
exhaustion at that boundary remains the specified budget fault.

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
and permutation-action semantics. Lifted row widths are not restricted to classical 128-bit widths. Per-vector
widths and aggregate live reference storage have a host cap of 999,999 bits;
excess produces an unchecked result before materialisation.
Fault causes retain every declared bit, including causes above u64; malformed
or out-of-range literals reject even when a faultif predicate is false.
Parallel moves snapshot all sources before writes. Lifted destinations must
be declared locations, operand/result widths must match, and missing labels
reject instead of defaulting to row zero.
Signed `lift.const.iN` literals use the
§3.3 modulo bit encoding after strict spelling/range checks; every oversized
shift, including `ashr`, produces zero. MCX entries naming multi-wire bundles
remain unchecked pending `b-isa-spec-sqr`; explicitly indexed controls are
replayed without truncation. An absent subject or expected-program closure in
an object with no imports rejects, as does an oracle relation on an action.
Table schemas, full-tuple encodings, widths, cardinality and input enumeration
are checked before target-dependent replay; output values still require the
realised operation for a scheduled claim.

For supported action bodies, reference replay retains exact amplitudes in
Q(√2,i) through H, S/S†, T/T†, CZ and permutation gates, predicates, counted
loops and nested/adjoint action calls. It accepts a table tuple only when the
final state is exactly one basis vector with phase +1. Intermediate
superpositions are retained; final superpositions or other phases reject a
permutation assertion. This path has a ten-wire host cap and a separate
2^25 amplitude-visit cap, and shares the reference instruction work limit.
Rotations and allocation/borrow/region lifetimes on this path stay unchecked.

Lifted reference memory implements `addr`, `boundscheck`, `load`, `store`,
`alloc` and `free` from §§4.2/7.2 using sparse little-endian bytes. It checks
whole-access allocation bounds without wrapping, pinned globals and mapped
W=8 interface regions, and monotonic heap/allocation-table history. Empty
allocations advance by one byte; free never reclaims either counter. Invalid
stores leave no partial delta. This model supports H ≤ 63, with host caps of
one million sparse bytes/table entries. It establishes reference F_B results;
it does not supply the realised quantum oracle expansion.

Lifted procedure replay supports multiple procedures and fixed token, classical
and region parameters, including recursive forwarding. Canonical row placement
uses closure SCC keys and member indices, followed by lexicographic entry-origin
binding keys. Equal argument values never merge instances; calls in unexecuted
branches still contribute instances. Labels and indirect-jump ranges belong to
the selected instance. Argument forwarding preserves raw iN, pointer, f64 and
token bits; region parameters refer to the original mapped cells. Immutable
parameters add no stack cells and cannot alias writable locations.

Calls capture arguments before saving and clearing callee locations; returns
restore them before delivering results. Stack use remains monotonic after
returns; stack/depth checks precede effects, and rows share the oracle's ISA and
host budgets. Only active records are materialised; unread historical cells
are compressed extensionally while their stack extent remains charged.
Procedure plans have host caps of 4,096 binding instances, 100,000 rows and
256 MiB of retained instance state/plan data and 1,048,576 static binding-work
units; active saved frames have a separate
256 MiB cap. Unsupported rows and unresolved invocation contexts remain
explicitly unchecked. These checks establish reference F_B results; realised
quantum oracle expansion is still incomplete. The native
matrix path currently handles single-class action calls, sequential action
blocks, counted `qloop` bodies, coherent `qif` arms and
X/CX/CCX/MCX/SWAP/H/S/S†/T/T†/CZ/RX/RY/RZ
gates. It includes invocations covered entirely
by defaults, forwards classical arguments, and checks maximal gate selector
specificity and ties. Static source ordinals are retained across loop visits;
iteration selectors use the complete dynamic nesting vector. Zero-trip loops
emit no events and cannot witness a selector tie. The two selector flips of
`qif` retain their own generated ordinals; full complex-column tests check
both arms and global phase. Schemes beyond the exact one-leaf case described below, `qloop while`, topology, multi-class
allocation and complete oracle expansion remain unchecked.

Rotation literals and bound `f64` arguments are decoded to exact binary64
rationals, including signed zero. Sin/cos use outward rational Taylor
enclosures with a Lagrange remainder below the §17 threshold. Modulo reduction
uses a proved Machin-formula enclosure of π. Excessive cancellation or work
produces a host-limit outcome; it never supplies an approximate constant or
an acceptance tolerance. These claims establish the requested numerical bound,
including global phase, rather than exact equality for arbitrary angles.

The checker does not infer exactness from the mere presence of an epsilon-zero
target entry. It checks the selected native entry with the actual total control
count. A scheme built from exact leaves is not assumed to realise its advertised
operation. K86 native-model metadata is hash-bound and independently checked.
The first implemented representation is a complete uncontrolled one-wire
rational unitary matrix. Exact rational Gram products establish unitarity;
exact Hermitian Schur elimination checks the advertised operator-norm accuracy,
including zero pivots and global phase. Numerical claims then use the required
outward interval procedure, so a tight certificate bound may remain inconclusive
or reject even when the separate exact model-accuracy check succeeds.

Models contradicting zero-epsilon opcodes reject. Positive epsilon without a
model and unknown model versions remain unchecked. Realised truth-table replay
uses the ideal opcode only after each supplied model is proved equal to it.
Controlled/multi-wire embeddings, transcendental model expressions and
Kraus/instrument/enclosure equivalence still need implementation; none receives
an ideal replacement. Native expression checking has a 64-level host nesting
cap, 4,096-digit rational constants and 65,536-bit expression intermediates.

Translation-lemma checks include counted loop bodies whose instructions retain
their wire bindings and borrowed lifetimes. Maximality is checked within the
containing static sequence; outer borrow touches do not enlarge an inner frame.
The published `borrow_demo` identity is pinned, and both printed orientations
are replayed over all 8 uncontrolled and 16 controlled basis inputs each.
Loops changing lifetimes or predicates still need visit-specific analysis.

For supported reference action bodies, indexed universal translation proofs
can discharge logical borrow preservation when they cover every borrowed-wire
touch and all borrowed lifetimes are returned. Region verdicts report this
conclusion explicitly. Missing indices, uncovered outer touches or proofs
limited to one invocation do not establish universal coverage. Frame storage
descriptors remain unchecked even when the logical preservation proof succeeds.
Oracles require their own T3 frame proof and cannot substitute this lemma.

`adjoint_table` and independent library correspondence still need complete proof
linkage. Verification-index shapes do not discharge borrow-frame entry-state or
qinplace-entry-applicability obligations. Source coverage and expanded non-native
accounting remain explicitly unchecked where they have not been independently
recomputed. Malformed shapes
are still rejected before claiming support.

For exact native realisations with one context per region, the
checker now recomputes scheduled leaf timing, all declared resource values,
gate counts, capacity and predicate limits, invocation multiplicities, and
local/inclusive error recurrence. Target metadata is validated through its
nested closed schemas. Gate costs distinguish own controls `k` from total
controls `c`; adjoint calls match and count the inverse primitive (`tdg`, not
`t`, for an adjointed T). Symbolic source-slot widths are evaluated at each
resolved inherited-control count.

Disjoint gates in a root action may overlap. Timing uses the maximum event end,
including when the last source event ends earlier, and default starts follow
that complete makespan. A sweep of half-open event intervals checks
`max_parallel`; gates sharing a wire retain their source-order dependencies.
Overlapping nested-call resource reports still require call-boundary lifetime
modelling and remain unchecked.

Mutation tests visit every field and array position of the published positive
certificate (250), target (43), schedule (54), coverage (30), and the
scheme-bearing target (67), with exact expected rejection
codes. The target and schedule sweeps rebind their hashes so a stale digest
cannot mask a malformed nested value. Schedule bindings validate scheduler and
expansion records, nonnegative error shares and exact budget encodings.

Native costs and achieved epsilon use exact expression evaluation, including
table lookups, `ceil`, and the integer `log2inv_eps`/`eps_decade` definitions.
Native and scheme matching checks parameter declarations against the source
angle. Parametric scheme ancilla/repeat expressions bind exact binary64 theta,
and template i is bound only within a repeat. An entry exceeding the requested
share is skipped in array order. Invalid
domains, out-of-range table indices, division by zero and exact entries that
depend on `eps` reject. Nonzero native realisation models and positive default
shares for expressions depending on `eps` remain unchecked. A zero region
accuracy gives a known zero default share. Missing selected native entries,
unknown named schemes and schemes whose opcode/control range does not match
reject with `E_SCHEDULE_INFEASIBLE`. Zero-ancilla, nonparametric single-leaf schemes are independently expanded.
The source and actual leaf operators must agree exactly, including controls
and phase. Native matching never recursively selects a scheme for a template
leaf. The leaf supplies its own controls, cost and opcode to accounting,
while predicate-depth checks retain the source effective context. Multiple
leaves, repeats, parameters and positive ancilla lifetimes remain unchecked.
Control-range
integers are compared exactly even above the host integer range.

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
`STRUCTURAL`, `COVERAGE`, `RESOURCE`, `GATE_COUNT`, `ESTIMATE`, `MULTIPLICITY` and
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
rebinding its hashes before checking and requiring `E_CERT_TARGET`. Schedule
hash rebinding covers all **54** published schedule fields and positions;
the three-classification coverage fixture covers all **30** with
`E_CERT_COVERAGE`. These sweeps do not sample fields. These checks are one part of mutation
coverage; they do not claim that every legal metadata edit must be rejected.

Coverage metadata gets closed-schema checks, conditional `fault_code` and
`shim_hash` fields, exact nonnegative counts, JCS bytes and program-point
bindings through symbol interface hashes. Correct source classification and
shim correspondence remain unchecked without dialect source evidence; valid
coverage metadata alone does not discharge those obligations.

JCS tests include all finite number samples from
[RFC 8785 Appendix B](https://www.rfc-editor.org/rfc/rfc8785.html#appendix-B),
UTF-16 property sorting and invalid surrogate rejection. The binary64
`43143ff3c1cb0959` tie is a regression for the standard Rust formatter's different
choice, which is why number rendering uses `ryu-js`.
