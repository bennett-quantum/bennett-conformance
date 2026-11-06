# Checker quality measurement

Measured 2026-11-06T06:39:21Z. This round is **blocked**, with no CLEAR or successful
repository-gate claim. The runner manages commits; the orchestrator owns review
and merge.

## Reproduction environment

Run from the repository root:

```sh
unset DISPLAY WAYLAND_DISPLAY DBUS_SESSION_BUS_ADDRESS BROWSER
export TMPDIR="$PWD/.tmp/quality-checker/tmp"
export CARGO_TARGET_DIR="$PWD/.tmp/quality-checker/target"
mkdir -p "$TMPDIR"
```

ISA inputs resolve exclusively through `gate.toml`, currently
`ecaaac060c44e317d1181345fa6b8e38c096c099`, and the detached cached HEAD is
checked before reading inputs. There is no live-sibling or environment override.

## Default checker tests

```sh
timeout 240 /usr/bin/time -p cargo test --offline --manifest-path checker/Cargo.toml -j3 --no-fail-fast -- --test-threads=4
cargo fmt --manifest-path checker/Cargo.toml -- --check
```

The test run took **46.93 seconds**, with **270 passed, 4 failed, 0 ignored**.
Formatting passed. `--no-fail-fast` runs the complete default set and preserves
all failures. The four failures are two tests in `published_certificates` and
two in `token_aliases`: the pinned cache lacks `accepted-k84-09` and
`accepted-k91-09`. The newer fixtures were previously read from a live checkout.
No tests were skipped to work around this mismatch. The successful full-suite
under-180-second criterion remains unverified until the pin is repaired.

The inherited nested-loop, qif-lifetime, sampled-table, signed-constant,
large-shift and bundle-control regressions pass. New lexical rejection tests
and the existing recompute-cleanup over-certification regression pass. A
schedule containing recompute cleanup now retains an explicit unchecked
obligation rather than using the reverse trace to certify it.

## Coverage

**Line coverage: 83.44% (14,131 / 16,935 lines).** Functions: 81.63%; regions:
85.16%. This is Rust checker coverage, not clause coverage or ecosystem coverage.

```sh
timeout 300 cargo llvm-cov --offline --manifest-path checker/Cargo.toml -j3 --ignore-run-fail --json --summary-only --output-path .tmp/quality-checker/coverage.json -- --test-threads=4
```

This run also had 270 passing and the same four failing tests. The explicit
`--ignore-run-fail` flag collects the report after the failed tests; its zero
exit status means report generation succeeded. It does not establish a green
test run. Re-measure without that flag after the pinned fixtures are available.
The report is in `.tmp/quality-checker/coverage.json`; full diagnostic output is
in `.tmp/quality-checker/coverage.log`. No coverage threshold is imposed.

The five least-covered source files still require tests. COMMON rule 8 stops
implementation while the pinned repository run has an unavailable prerequisite;
these follow-ups are recorded for the orchestrator in the job's Beads handoff.

| File | Lines | Retained test work |
| --- | ---: | --- |
| `fault.rs` | 50.00% | Exercise path prefixing, JSON records, Display and both ResolveError conversions. |
| `domain.rs` | 69.02% | Cover unresolved identities, omitted components, invalid dimensions and host limits; K89 requires the newer contract pin. |
| `structural.rs` | 75.08% | Cover structural frame endpoint/lifetime failures and supported positive compositions. |
| `main.rs` | 75.19% | Cover all exit codes, file/UTF-8/embedded-certificate failures and successful help. |
| `eval.rs` | 75.75% | Extend malformed call/operand and resource-bound execution cases, with semantic oracles. |

## Remaining round work

The local stop is the missing pinned fixture snapshot. The orchestrator must
select and hydrate a reviewed ISA pin containing K84/K89/K91/K93. K89 schema
handling remains open because the current published pin does not define
`borrow_frame`, `workspace` or `entry_products`; the checker must not invent
those schemas from review shorthand.

The job handoff retains binary consolidation and test-profile work, value
mutation tests, clause tags and `clausegraph.py coverage`, the named checker
function splits, fail-fast review, CLI/fault tests, and artifact hygiene.
The over-300-line inventory has an explicit quality-checker owner in that
handoff. No runtime temporary-file writers were found in the Rust tests;
compiled test artifacts reached 1.9 GB before coverage, so profile/binary
consolidation remains relevant. Review documents and worklogs are owned by
separate jobs. Verdict changes require fresh review and council review before
any CLEAR claim.
