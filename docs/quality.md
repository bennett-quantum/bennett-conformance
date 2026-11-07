# Checker quality measurement

ISA inputs and the neutral-suite binary share gate pin
`3b1090790d82a00ac95d30b8da6b42d1d7a9f5d1`. Tests validate the detached
gate-cache HEAD and never read a live sibling. The pin was materialized and
its release interpreter built with the pinned gate tool's `verify` command
against the main checkout and job commit dc5e2e2.

## Reproduction environment

```sh
unset DISPLAY WAYLAND_DISPLAY DBUS_SESSION_BUS_ADDRESS BROWSER
export TMPDIR="$PWD/.tmp/quality-checker/tmp"
export CARGO_TARGET_DIR="$PWD/.tmp/quality-checker/target"
mkdir -p "$TMPDIR"
free -g
uptime
systemd-run --user --scope --quiet timeout 240 /usr/bin/time -p cargo test --offline --manifest-path checker/Cargo.toml -j3 --no-fail-fast -- --test-threads=4
cargo fmt --manifest-path checker/Cargo.toml -- --check
```

Builds require at least 6 GB available RAM and one-minute load below 12.

## Default checker tests

| Configuration | Result | Wall time |
| --- | --- | ---: |
| Before pin repair | 270 passed, 4 missing-fixture failures | 37.45 s |
| Updated pin, original binaries | 274 passed, 0 failed, 0 ignored | 79.86 s |
| Consolidated binary, line-table test profile | 276 passed, 0 failed, 0 ignored | 46.30 s |

The last run includes the profile rebuild. Every integration case is included in
`tests/conformance.rs`; no slow tests or exclusions were needed. The two added
cases cover selected borrow-frame admission as explicit unchecked, malformed
frame fields and invocation bindings. Complete borrow-frame semantics remain an
unresolved obligation, with no proof/count/coverage assigned.

The inherited nested-loop, qif-lifetime, sampled-table, signed-literal,
large-shift, whole-bundle MCX, lexical and recompute-cleanup regressions all pass.
A recompute schedule retains explicit unchecked obligations.

## Coverage

**Lines: 83.67% (14,248 / 17,029); functions: 82.04%; regions: 85.48%.**
Measured on the unified pin after adding rejection-record tests: 278 passed,
zero failures, without ignoring run failures.

```sh
systemd-run --user --scope --quiet timeout 300 cargo llvm-cov --offline --manifest-path checker/Cargo.toml -j3 --json --summary-only --output-path .tmp/quality-checker/coverage.json -- --test-threads=4
```

This measures Rust checker execution, separately from clause coverage. No
threshold is imposed. `fault.rs` is now 100% lines: path prefixing, JSON fields,
Display and both ResolveError conversions are asserted exactly.

The five weakest source files have explicit retained work in
bennett-conformance-ac5:

| File | Lines | Reason / next witness |
| --- | ---: | --- |
| `eval/borrow_frames.rs` | 56.41% | Scheduled-context branches and independently derived frame-entry mechanics remain under the full adoption issue; no invented positive proof. |
| `domain.rs` | 69.02% | Large host-limit/component-shape branches need bounded witnesses that avoid materializing huge domains. |
| `structural.rs` | 75.08% | Adjoint/library and frame-provenance branches need contract-derived positive compositions. |
| `main.rs` | 75.19% | Usage/IO/UTF-8/help and rejected exits are the next fail-fast item. |
| `eval.rs` | 76.14% | Remaining procedure/memory unsupported branches need bounded independent witnesses. |

## Review and remaining work

Fresh review and council review of verdict changes remain the orchestrator's
responsibility; green tests are not a CLEAR verdict. Remaining quality items:
named function phases,
fail-fast and CLI checks, artifact hygiene and the baseline ratchet.

## Test oracles and clause mapping

Value mutations cover six same-type wrong binding hashes, all eight output bits
and four shifted input rows of the controlled-action table. Type sweeps still
visit every field; they no longer hard-code a snapshot's field count. The heap
store/load witness additionally checks the independently derived payload x and
zero fault for all sixteen x values, rather than only its verdict/count.
The updated integration run passed 269 cases.

```sh
python3 clauses/clausegraph.py coverage clauses/ISA-0.9.json --tags checker/tests/clause_tags.json
```

Fifteen mapped tests were confirmed executed in the successful integration run.
This partial checker-only map reports 671 clauses: 6 strong, 23 weak section-only,
642 uncovered. It is separate from proof tags and line coverage. The clause graph
predates the current pinned contract (source hashes differ); the retained tags
refer to unchanged paragraph text. Updating the graph, K89 tags, Python tests and
neutral-case aggregation is retained as bennett-conformance-fv9.
