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

The previous pin's measurement was 83.44% lines (14,131 / 16,935), with four
missing-fixture failures. It is historical evidence only; the new pin requires
a fresh successful measurement. No coverage threshold is imposed.

## Review and remaining work

Fresh review and council review of verdict changes remain the orchestrator's
responsibility; green tests are not a CLEAR verdict. Remaining quality items:
coverage/error-record tests, value mutation/clause tags, named function phases,
fail-fast and CLI checks, artifact hygiene and the baseline ratchet.
