| Stage | LLVM 18 | LLVM 23 |
|---|---|---|
| fixtures | PASS | PASS |
| plugin build | PASS | PASS |
| VM build | PASS | PASS |
| driver build | PASS | PASS |
| musl C front end | FAIL · brt-1ft | FAIL · brt-1ft |
| runtime bitcode link | BLOCKED · brt-1ft | BLOCKED · brt-1ft |
| post-link LTO | BLOCKED · brt-1ft | BLOCKED · brt-1ft |
| reversible production | BLOCKED · brt-1ft | BLOCKED · brt-1ft |
| launch artefacts | BLOCKED · brt-1ft | BLOCKED · brt-1ft |
| native comparison | BLOCKED · brt-1ft | BLOCKED · brt-1ft |
| reference admission | BLOCKED · brt-1ft | BLOCKED · brt-1ft |
| reference forward = native | BLOCKED · brt-1ft | BLOCKED · brt-1ft |
| reference reset | BLOCKED · brt-1ft | BLOCKED · brt-1ft |
| reference tape replay | BLOCKED · brt-1ft | BLOCKED · brt-1ft |
| VM admission | BLOCKED · brt-1ft | BLOCKED · brt-1ft |
| VM forward = native | BLOCKED · brt-1ft | BLOCKED · brt-1ft |
| VM reset | BLOCKED · brt-1ft | BLOCKED · brt-1ft |
| VM tape replay | BLOCKED · brt-1ft | BLOCKED · brt-1ft |
| runtime libc/console audit (diagnostic) | FAIL · brt-1ft, brt-cgb | FAIL · brt-1ft, brt-cgb |
| R-mode probe (diagnostic) | FAIL · bp-ivy | FAIL · bp-ivy |
| driver probe (diagnostic) | FAIL · bennett-driver-cyg | FAIL · bennett-driver-cyg |
| ISA-0.8 R contract (prerequisite) | FAIL · b-isa-spec-221 | FAIL · b-isa-spec-221 |
| reference device/reset control (diagnostic) | PASS | PASS |
| reference replay API (diagnostic) | FAIL · b-isa-spec-04d | FAIL · b-isa-spec-04d |
| VM device/reset control (diagnostic) | PASS | PASS |
| VM replay API (diagnostic) | FAIL · bennett-vm-u8m | FAIL · bennett-vm-u8m |
