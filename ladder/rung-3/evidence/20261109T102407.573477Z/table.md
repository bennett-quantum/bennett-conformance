| Stage | LLVM 18 | LLVM 23 |
|---|---|---|
| fixtures | PASS | PASS |
| plugin build | PASS | PASS |
| VM build | PASS | PASS |
| driver build | PASS | PASS |
| R contract (prerequisite) | PASS | PASS |
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
| reference corrupt-tape rejection | BLOCKED · bp-mq6 | BLOCKED · bp-mq6 |
| VM admission | BLOCKED · bp-mq6 | BLOCKED · bp-mq6 |
| VM forward = native | BLOCKED · bp-mq6 | BLOCKED · bp-mq6 |
| VM reset | BLOCKED · bp-mq6 | BLOCKED · bp-mq6 |
| VM tape replay | BLOCKED · bp-mq6 | BLOCKED · bp-mq6 |
| VM corrupt-tape rejection | BLOCKED · bp-mq6 | BLOCKED · bp-mq6 |
| printf_fmt: musl C front end | PASS | PASS |
| printf_fmt: runtime bitcode link | FAIL · bp-mq6 | FAIL · bp-mq6 |
| printf_fmt: post-link LTO | BLOCKED · bp-mq6 | BLOCKED · bp-mq6 |
| printf_fmt: reversible production | BLOCKED · bp-mq6 | BLOCKED · bp-mq6 |
| printf_fmt: launch artefacts | BLOCKED · bp-mq6 | BLOCKED · bp-mq6 |
| printf_fmt: native comparison | BLOCKED · bp-mq6 | BLOCKED · bp-mq6 |
| printf_fmt: reference admission | BLOCKED · bp-mq6 | BLOCKED · bp-mq6 |
| printf_fmt: reference forward = native | BLOCKED · bp-mq6 | BLOCKED · bp-mq6 |
| printf_fmt: reference reset | BLOCKED · bp-mq6 | BLOCKED · bp-mq6 |
| printf_fmt: reference tape replay | BLOCKED · bp-mq6 | BLOCKED · bp-mq6 |
| printf_fmt: reference corrupt-tape rejection | BLOCKED · bp-mq6 | BLOCKED · bp-mq6 |
| printf_fmt: VM admission | BLOCKED · bp-mq6 | BLOCKED · bp-mq6 |
| printf_fmt: VM forward = native | BLOCKED · bp-mq6 | BLOCKED · bp-mq6 |
| printf_fmt: VM reset | BLOCKED · bp-mq6 | BLOCKED · bp-mq6 |
| printf_fmt: VM tape replay | BLOCKED · bp-mq6 | BLOCKED · bp-mq6 |
| printf_fmt: VM corrupt-tape rejection | BLOCKED · bp-mq6 | BLOCKED · bp-mq6 |
| runtime libc/console audit (diagnostic) | FAIL · bp-mq6 | FAIL · bp-mq6 |
| R-mode probe (diagnostic) | PASS | PASS |
| driver probe (diagnostic) | FAIL · bp-mq6 | FAIL · bp-mq6 |
| reference device/reset control (diagnostic) | PASS | PASS |
| reference replay API (diagnostic) | PASS | PASS |
| VM device/reset control (diagnostic) | PASS | PASS |
| VM replay API (diagnostic) | PASS | PASS |
