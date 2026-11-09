| Stage | LLVM 18 | LLVM 23 |
|---|---|---|
| fixtures | PASS | PASS |
| plugin build | PASS | PASS |
| VM build | PASS | PASS |
| driver build | PASS | PASS |
| R contract (prerequisite) | PASS | PASS |
| musl C front end | PASS | PASS |
| runtime bitcode link | PASS | PASS |
| post-link LTO | PASS | PASS |
| reversible production | PASS | PASS |
| launch artefacts | PASS | PASS |
| native comparison | FAIL · request-driver-r-contract | FAIL · request-driver-r-contract |
| reference admission | PASS | PASS |
| reference forward = native | BLOCKED · request-driver-r-contract | BLOCKED · request-driver-r-contract |
| reference reset | BLOCKED · request-driver-r-contract | BLOCKED · request-driver-r-contract |
| reference tape replay | BLOCKED · request-driver-r-contract | BLOCKED · request-driver-r-contract |
| reference corrupt-tape rejection | BLOCKED · request-driver-r-contract | BLOCKED · request-driver-r-contract |
| VM admission | PASS | PASS |
| VM forward = native | BLOCKED · request-driver-r-contract | BLOCKED · request-driver-r-contract |
| VM reset | BLOCKED · request-driver-r-contract | BLOCKED · request-driver-r-contract |
| VM tape replay | BLOCKED · request-driver-r-contract | BLOCKED · request-driver-r-contract |
| VM corrupt-tape rejection | BLOCKED · request-driver-r-contract | BLOCKED · request-driver-r-contract |
| printf_fmt: musl C front end | PASS | PASS |
| printf_fmt: runtime bitcode link | PASS | PASS |
| printf_fmt: post-link LTO | PASS | PASS |
| printf_fmt: reversible production | FAIL · bp-owg | FAIL · bp-owg |
| printf_fmt: launch artefacts | BLOCKED · bp-owg | BLOCKED · bp-owg |
| printf_fmt: native comparison | BLOCKED · bp-owg | BLOCKED · bp-owg |
| printf_fmt: reference admission | BLOCKED · bp-owg | BLOCKED · bp-owg |
| printf_fmt: reference forward = native | BLOCKED · bp-owg | BLOCKED · bp-owg |
| printf_fmt: reference reset | BLOCKED · bp-owg | BLOCKED · bp-owg |
| printf_fmt: reference tape replay | BLOCKED · bp-owg | BLOCKED · bp-owg |
| printf_fmt: reference corrupt-tape rejection | BLOCKED · bp-owg | BLOCKED · bp-owg |
| printf_fmt: VM admission | BLOCKED · bp-owg | BLOCKED · bp-owg |
| printf_fmt: VM forward = native | BLOCKED · bp-owg | BLOCKED · bp-owg |
| printf_fmt: VM reset | BLOCKED · bp-owg | BLOCKED · bp-owg |
| printf_fmt: VM tape replay | BLOCKED · bp-owg | BLOCKED · bp-owg |
| printf_fmt: VM corrupt-tape rejection | BLOCKED · bp-owg | BLOCKED · bp-owg |
| runtime libc/console audit (diagnostic) | PASS | PASS |
| R-mode probe (diagnostic) | PASS | PASS |
| driver probe (diagnostic) | FAIL · request-driver-r-contract | FAIL · request-driver-r-contract |
| reference device/reset control (diagnostic) | PASS | PASS |
| reference replay API (diagnostic) | PASS | PASS |
| VM device/reset control (diagnostic) | PASS | PASS |
| VM replay API (diagnostic) | PASS | PASS |
