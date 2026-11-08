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
| `proof_relative_to_image` | Exhaustive agreement on valid fixed image bindings and every value of every remaining component; preparation and entry factorisation remain separate |
| `evidence` | Agreement only on the stated slice or sampled draws, with exact replay count and distinct covered fraction |
| `accept` | The named numerical/channel bound or syntactic structural rule at the resolved invocation scope |
| `reject` | A named obligation failed; a diagnostic gives its witness or malformed field |
| `unchecked` | No conclusion; a reason identifies the missing mechanism or host limit |

A table has one row for every declared-domain element in enumeration order,
including sampled claims. The checker validates all row shapes and input
correspondence, then replays only the drawn elements for a sample. A legal but
false output at an undrawn point remains untested; the sample verdict reports
evidence only for its distinct drawn subset.

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
shift, including `ashr`, produces zero. Under K93, MCX list entries expand
multi-wire bundles in increasing bit index and written entry order. Repeated or
overlapping entries reject before execution, including under disabled predicates.
The target must be one live wire; flattened explicit controls determine k, and
effective predicate controls determine the additional part of c. An absent subject or expected-program closure in
an object with no imports rejects, as does an oracle relation on an action.
Table schemas, full-tuple encodings, widths, cardinality and input enumeration
are checked before target-dependent replay; output values still require the
realised operation for a scheduled claim.

Basis replay gives each `qloop while` a distinct private flag lifetime, preserving
outer predicates across nested loops and subsequent instructions. Source writes
or consumption of an active while selector reject, including mutable call
actuals and disabled gate paths. Qif selectors retain a precise read-only bit guard across explicit predicate
pops, linked calls and adjoints. Structural qif flips restore their selector;
the guard ends at loop exit. Full static selector-provenance analysis remains
unfinished. Replay executes both qif arms through §4.6's literal predicated expansion: allocations, images
and releases happen unconditionally while gates obey the active controls.
Adjoints recover unique explicit allocation sites and reverse the complete qif
expansion across all blocks. Reused names, implicit allocations and borrowed
release recovery still yield unchecked. Local allocation results must be fresh;
release checks ownership and cleanliness. Indexed wire actuals preserve the
other bundle bits, and action calls check exact widths, region shapes and wire
disjointness before effects, including disabled calls. Bulk operations accept
disjoint indexed views and reject overlap or unequal widths. Quantum memory
requires exactly A address wires (only `()` at A=0), W value wires and matching
owned cleanup. Local regions exist once per invocation from function entry,
including declarations in zero-trip loops. Images have exact sizes and
little-endian packed cells.

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

For supported native action realisations, including repeated invocation
contexts, the checker recomputes scheduled leaf timing, all declared resource values,
gate counts, capacity and predicate limits, invocation multiplicities, and
local/inclusive error recurrence. Target metadata is validated through its
nested closed schemas. Gate costs distinguish own controls `k` from total
controls `c`; adjoint calls match and count the inverse primitive (`tdg`, not
`t`, for an adjointed T). Symbolic source-slot widths are evaluated at each
resolved inherited-control count. K87 `resource_contexts` must cover every
reached invocation in canonical full-key order. Unkeyed resources, gate counts
and optional estimates equal the first canonical context. Child reports use
invocation-local elapsed time; root reports count their own complete event list
once. Every resource display is a canonical upward 34-digit decimal string;
occupancy and peak rows carry their encoded class qualification. Exact
accounting precedes display rounding, with or without `estimate`.

Disjoint gates in a root action may overlap. Timing uses the maximum event end,
including when the last source event ends earlier, and default starts follow
that complete makespan. A sweep of half-open event intervals checks
`max_parallel`; gates sharing a wire retain their source-order dependencies.
Overlapping nested-call resource reports still require call-boundary lifetime
modelling and remain unchecked.

Mutation tests visit every field and array position of the published positive
certificate (250), target (43), schedule (54), coverage (30), and the
scheme-bearing target (67), frame-context records (39), and resource-context
records (26), with exact expected rejection
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

Repeated contexts may have different controls, timing and argument-dependent
costs for one closure. K87 reports them individually under complete context
keys; no aggregation of child peak/time maps replaces the root's own event
accounting. Overlapping nested-call lifetimes and complete multi-class
accounting remain unchecked where their expansion is unfinished.

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

The K90 `frame_contexts` alternative validates full concrete invocation keys,
canonical order and complete coverage of default-covered and predicated-off
calls in the supported native action tree. Each selected descriptor is checked;
a uniform `frame` evaluates its slot widths at every reached control count.
Allocation lifetimes and multi-class placement remain separate unchecked
obligations where their expansion is unfinished.

The checker-owned `controlled-action-k87` fixture repairs the published
`controlled-action-09` resource encodings in detached and embedded certificates.
Its source, interface, closure, stripped-object, target and schedule hashes are
unchanged. A separate regression checks that the upstream numeric pre-K87
report rejects; the required upstream fixture migration is recorded for its owner.

K85 fixed-image domains resolve only in-role interface regions, validate their
exact shape, lowercase bytes, zero padding and raw-byte SHA-256, and fill those
cells on every replay. Fixed cells add no enumeration or sampler component.
Exhaustive success is `proof_relative_to_image` only when every other component
has its full range; otherwise it is evidence. Sampled success remains evidence.
Coverage uses unrestricted D. The report retains ordered mem-slot image
bindings, invocation and context. Executable preparation and entry factorisation
remain separate unchecked obligations, so a conditional theorem cannot discharge
an unrestricted inverse-pair or borrowed-frame proof.

K92 source inventory requires every supplied region's qinplace site to have
an entry for its exact resolved oracle pair in that owner record. Missing pairs
reject; referenced exhaustive claims must specify `inverse_pair`. This inventory
check does not discharge invocation budget, argument or token identity coverage.

Quantum interface schemas are closed and checked against source declarations
before host limits: widths, names, roles, regions, classical argument types,
ABI/kind/tier and declared bounds must agree. Malformed or missing arrays cannot
become empty lists. The signature sweep covers all 33 field/type positions and
22 required field removals.

K88 frame slots accept the homogeneous `class` form or a `fragments` array.
Fragments must cover the complete slot in order, without gaps or overlap, use
positive integer widths and declared classes, and merge adjacent equal classes.
The slot keeps its original source identity. Validation of inherited class
vectors, placement and live intervals remains an explicit unchecked obligation.

K91 token equality partitions are distinct from token tuples and parameter
hashes. Concrete invocations with token arguments require explicit canonical
labels; equal labels require equal tuples. Oracle order prepends its primary
token; actions have only token-argument positions. Root schedules must match the
certificate partition. Native action contexts forward source identities and
canonicalize their own local labels. Uniform structural token proofs quantify
all consistent partitions; inverse-pair entries bind their partition too.
Token-free omission derives the unique empty/oracle-singleton partition for
published older positives; a specification clarification is recorded. Scheduled
carrier/lifecycle integration remains unfinished, and the value-only coherent
entry API defers token-bearing replay until it carries identities.

Object parsing checks §16.1 lexical form before hashing function bodies:
canonical numeric spellings, spacing, LF line endings and comment-free text.
The empty `regs {  }` and empty-HEX data exceptions retain their specified
spelling. Metadata JSON and quoted strings keep their own lexical rules.

Schedules containing a recompute cleanup decision remain `unchecked` because
the checker has not derived that cleanup expansion or its resources. A source
adjoint's reverse trace cannot certify a different scheduled implementation.

Checker tests require `BENNETT_ISA_SPEC`, which the gate sets to
`{sibling:bennett-isa-spec}`. The selected checkout must have a detached HEAD
equal to the `bennett-isa-spec` SHA in `gate.toml`. An unset variable, missing
checkout, symbolic HEAD or different revision fails with a diagnostic naming
`BENNETT_ISA_SPEC`; no local-cache path or live sibling is substituted.

After materializing the pin, use the headless environment in
[quality.md](quality.md) and run this one-line command from the repository root:

```sh
BENNETT_ISA_SPEC="$PWD/.tmp/gate/cache/bennett-isa-spec@3b1090790d82/bennett-isa-spec" systemd-run --user --scope --quiet timeout 240 cargo test --offline --manifest-path checker/Cargo.toml -j3 -- --test-threads=4
```

The variable points outside any fresh exported checkout, so the same pin can be
used without a cache inside that export. See [quality.md](quality.md) for
measured runtime and coverage limits.

Selected §12.1 borrow-frame relations admit the published §12.2 workspace and
entry-products vocabulary. They remain **unchecked**: this checker does not
derive maximal source occurrences, normative entry domains or tensor-factor
replay, and assigns no element count, coverage or proof. Required frame fields,
subject/schedule binding and standalone invocation binding are checked before
this deferral. Domain, occurrence and sampler validity remain named unresolved
obligations. Ordinary interface relations retain their closed domain schema.

## Canonical source admission differential

Before hashing, the reader checks §16.1 atoms in their §16.2 contexts: integer
type/opcode widths, version components, unsigned bounds/counts/indices/region
dimensions, signed integer literals, and canonical finite gate-angle floats.
Punctuation uses the printed grammar spacing, including the empty-regs and
empty-HEX exceptions. No host-width cap is added to lexical integer validation.

The default integration test generates mutations at every numeric-atom and
punctuation position of a canonical multi-function object: 52 numeric sites,
183 punctuation sites, and 1,933 nonidentity mutation cases. Numeric changes
include leading zeros, signs, hex, underscores, decimal-looking integer
spellings and inserted whitespace; punctuation changes remove/add spaces,
tabs and newlines on both sides. All eight numeric roles must occur in the seed.

Each candidate goes through the gate-pinned release interpreter's black-box
`hash` and `check` commands and this reader's object parser. The hash command
repairs stated metadata independently, so a stale §9.2 binding cannot conceal
an admission mismatch. Every interpreter-rejected candidate is also sent to
the certificate entry point with bindings rebound if this reader admits it;
neither overall verification nor a claim proof may result. JSON metadata and
data HEX use their separate grammars and are excluded from INT mutations.

The test runs the release interpreter under the validated ISA pin and checks
that any `BENNETT_ISA_BINARY` identifies that same binary. Missing binaries fail
with a build instruction. The canonical identity control still verifies;
noncanonical integer declarations and bounds reject with `E_CERT_OBJECT`
at §16.1. Internal evaluator defense tests inject malformed tokens after a
valid parse; they do not treat those spellings as legal source objects.


Source admission now runs before document binding and verdict evaluation. It
checks complete function/header productions and all instruction bodies,
including unclaimed functions, both conditional arms and zero-trip loops.
Malformed source receives E_CERT_OBJECT with its specification section. Indexed
control prefixes retain their complete operand, and opcode/operand typing is
separate from basis-state replay.

Executed loop bodies must preserve the predicate stack at body boundaries
(§3.5/§4.6); a violation receives E_CERT_OBJECT. The equivalent control can be
written with balanced qpred/qunpred inside each body.

Three complete-source checks remain explicit unchecked obligations: repeated
same-type definitions whose pinned SSA admission disagrees with §3.4, indirect
label widths that require full binding-instance index spaces (§3.6), and linked
imported bodies (§3.3/§9.2). Any such obligation defers the whole certificate
before a region or claim proof is issued.


The default generated source differential currently checks 3,331 candidates
against the detached gate-pinned release interpreter in both admission
directions, with zero disagreements. Every interpreter-rejected candidate also
passes through the public certificate entry point to ensure it receives no
verification or claim proof. The original canonical and indexed-control
positives retain independently specified full-domain truth tables.
