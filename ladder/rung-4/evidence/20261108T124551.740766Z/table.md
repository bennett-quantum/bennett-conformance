| Stage | LLVM 18 | LLVM 23 |
|---|---|---|
| sibling revisions | PASS | PASS |
| reference build | PASS | PASS |
| fixtures | PASS | PASS |
| SQLite source unmodified | PASS | PASS |
| independent predicate | PASS | PASS |
| plugin build | PASS | PASS |
| VM build | PASS | PASS |
| driver build | PASS | PASS |
| estimator build | PASS | PASS |
| checker build | PASS | PASS |
| host image = native oracle = predicate | PASS | PASS |
| images agree across majors | PASS | PASS |
| musl C front end | PASS | PASS |
| runtime bitcode link | PASS | PASS |
| runtime libc closure | FAIL · brt-dj7 | FAIL · brt-dj7 |
| post-link LTO | PASS | PASS |
| coherent production | FAIL · bp-1bb | FAIL · bp-1bb |
| launch artefacts and oracle shape | BLOCKED · bp-1bb | BLOCKED · bp-1bb |
| materialised comparison = native | BLOCKED · bp-1bb, brt-dj7 | BLOCKED · bp-1bb, brt-dj7 |
| reference admission | BLOCKED · bp-1bb, brt-dj7 | BLOCKED · bp-1bb, brt-dj7 |
| reference (a) oracle = native, fault rate zero | BLOCKED · bp-1bb, brt-dj7 | BLOCKED · bp-1bb, brt-dj7 |
| reference (a) full tuple on D | BLOCKED · bp-1bb, brt-dj7 | BLOCKED · bp-1bb, brt-dj7 |
| reference (a) literal step check | BLOCKED · bp-1bb, brt-dj7 | BLOCKED · bp-1bb, brt-dj7 |
| reference (b) amplitudes | BLOCKED · bp-1bb, brt-dj7 | BLOCKED · bp-1bb, brt-dj7 |
| reference (b) outcomes | BLOCKED · bp-1bb, brt-dj7 | BLOCKED · bp-1bb, brt-dj7 |
| VM admission | BLOCKED · bp-1bb, brt-dj7 | BLOCKED · bp-1bb, brt-dj7 |
| VM (a) oracle = native, fault rate zero | BLOCKED · bp-1bb, brt-dj7 | BLOCKED · bp-1bb, brt-dj7 |
| VM (a) full tuple on D | BLOCKED · bp-1bb, brt-dj7 | BLOCKED · bp-1bb, brt-dj7 |
| VM (a) literal step check | BLOCKED · bp-1bb, brt-dj7 | BLOCKED · bp-1bb, brt-dj7 |
| VM (b) amplitudes | BLOCKED · bp-1bb, brt-dj7 | BLOCKED · bp-1bb, brt-dj7 |
| VM (b) outcomes | BLOCKED · bp-1bb, brt-dj7 | BLOCKED · bp-1bb, brt-dj7 |
| (a) certificate verdict | BLOCKED · bp-1bb, brt-dj7 | BLOCKED · bp-1bb, brt-dj7 |
| (c) resource estimate at N=2^20 | BLOCKED · bp-1bb | BLOCKED · bp-1bb |
| reference reduced control (diagnostic) | PASS | PASS |
| VM reduced control (diagnostic) | PASS | PASS |
| published Grover TU production (diagnostic) | FAIL · bp-vsa | FAIL · bp-vsa |
| reference --mem-file (diagnostic) | FAIL · b-isa-spec-mtc | FAIL · b-isa-spec-mtc |
| VM --mem-file (diagnostic) | FAIL · bennett-vm-0mp | FAIL · bennett-vm-0mp |
| reference measured control (diagnostic) | PASS | PASS |
| VM measured control (diagnostic) | PASS | PASS |
| estimator CLI (diagnostic) | FAIL · bennett-estimate-b8o | FAIL · bennett-estimate-b8o |
| checker CLI (diagnostic) | FAIL · bennett-conformance-51d | FAIL · bennett-conformance-51d |
