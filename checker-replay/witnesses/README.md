# Differential witnesses: checker-replay vs the published checker (black box)

Source: `python3 -m checker_replay.differential` (run from checker-replay/), black box built
from checker commit e5bc3af. Each directory holds `object.bisa`, a JCS-canonical
`certificate.json`, `extra-args.json` (extra CLI arguments, empty here), and both verdicts
(`replay.json`, `checker.json`).

Reproduce: `bennett-check DIR/certificate.json --object DIR/object.bisa` and
`python3 -m checker_replay DIR/certificate.json --object DIR/object.bisa`.

| Directory | Replay / binary | Determination (spec) |
|---|---|---|
| const-negative-spelling | evidence / reject | binary: `lift.const.i4 -1` must denote 15 (§3.3 literal rule); it replays 0 |
| ashr-shift-ge-n | evidence / reject | binary: `ashr` by >= N must give 0 (§4.1 "shift >= N -> 0", §11.1 shifts); it sign-fills (8 ashr 4 = 15) |
| mcx-bundle-control | proof / reject | binary: `mcx (c) p`, c:wire[2], fires at c=1 (c[1]=0); "X iff all controls are 1" (§4.4) |
| unchecked-subject-closure-absent | reject / unchecked | binary weaker: subject closure absent from an object with no imports |
| unchecked-program-absent | reject / unchecked | binary weaker: expected.program absent from an object with no imports |
| unchecked-oracle-on-action | reject / unchecked | binary weaker: expected "oracle" on an action subject (§12.1 requires §5.2's oracle map) |
| schedule-root-mismatch | (was) unchecked / reject | replay fixed in 20e405a: schedule root closure must be the certificate root |
| table-without-target | reject / unchecked | both safe: replay rejects a table that disagrees with the reference before needing the target |
| timeout-addmod-w8 | unchecked / timeout | binary exceeded 120 s (debug build, loaded host) on a published object |
| lproc-recursive-sum, oracle-store-load-heap, action-hzh | evidence or proof / unchecked | binary capability boundary (lproc/memory rows, H/Z/H action replay) per docs/checker.md |
