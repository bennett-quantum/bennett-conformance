| Stage | LLVM 18 | LLVM 23 |
|---|---|---|
| fixtures | PASS | PASS |
| plugin build | PASS | PASS |
| VM build | PASS | PASS |
| driver build | PASS | PASS |
| musl C front end | PASS | PASS |
| runtime bitcode link | FAIL · bp-mq6 | FAIL · bp-mq6 |
| post-link LTO | BLOCKED · bp-mq6 | BLOCKED · bp-mq6 |
| reversible production | BLOCKED · bp-mq6 | BLOCKED · bp-mq6 |
| launch artefacts | BLOCKED · bp-mq6 | BLOCKED · bp-mq6 |
| native comparison | BLOCKED · bp-mq6 | BLOCKED · bp-mq6 |
| reference admission | BLOCKED · bp-mq6 | BLOCKED · bp-mq6 |
| reference forward = native | BLOCKED · bp-mq6 | BLOCKED · bp-mq6 |
| reference reset | BLOCKED · bp-mq6 | BLOCKED · bp-mq6 |
| reference tape replay | BLOCKED · bp-mq6 | BLOCKED · bp-mq6 |
| VM admission | BLOCKED · bp-mq6 | BLOCKED · bp-mq6 |
| VM forward = native | BLOCKED · bp-mq6 | BLOCKED · bp-mq6 |
| VM reset | BLOCKED · bp-mq6 | BLOCKED · bp-mq6 |
| VM tape replay | BLOCKED · bp-mq6 | BLOCKED · bp-mq6 |
| runtime libc/console audit (diagnostic) | FAIL · bp-mq6 | FAIL · bp-mq6 |
| R-mode probe (diagnostic) | FAIL · bp-is0 | FAIL · bp-is0 |
| driver probe (diagnostic) | FAIL · bennett-conformance-igc | FAIL · bennett-conformance-igc |
| ISA-0.8 R contract (prerequisite) | FAIL · b-isa-spec-fuk | FAIL · b-isa-spec-fuk |
| reference device/reset control (diagnostic) | PASS | PASS |
| reference replay API (diagnostic) | PASS | PASS |
| VM device/reset control (diagnostic) | PASS | PASS |
| VM replay API (diagnostic) | PASS | PASS |
