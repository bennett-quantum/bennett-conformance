| Language / stage | LLVM 18 | LLVM 23 |
|---|---|---|
| C / toolchain | PASS | PASS |
| C / fixtures | PASS | PASS |
| C / plugin build | PASS | PASS |
| C / VM build | PASS | PASS |
| C / native O0 = O2 | PASS | PASS |
| C / frontend | PASS | PASS |
| C / runtime bitcode link | PASS | PASS |
| C / post-link LTO | PASS | PASS |
| C / pipeline | PASS | PASS |
| C / materialised native comparison | FAIL · bp-75y | FAIL · bp-75y |
| C / reference admission | PASS | PASS |
| C / VM admission | PASS | PASS |
| C / reference input exhaustive = native (diagnostic) | PASS | PASS |
| C / VM input exhaustive = native (diagnostic) | PASS | PASS |
| C / reference input zero faults (diagnostic) | PASS | PASS |
| C / VM input zero faults (diagnostic) | PASS | PASS |
| C / reference input byte-exact reversal (diagnostic) | PASS | PASS |
| C / VM input byte-exact reversal (diagnostic) | PASS | PASS |
| C / reference full D interface | PASS | PASS |
| C / VM full D interface | PASS | PASS |
| C / reference exhaustive full D = native | BLOCKED · bp-75y | BLOCKED · bp-75y |
| C / VM exhaustive full D = native | BLOCKED · bp-75y | BLOCKED · bp-75y |
| C / reference zero faults on D | BLOCKED · bp-75y | BLOCKED · bp-75y |
| C / VM zero faults on D | BLOCKED · bp-75y | BLOCKED · bp-75y |
| C / reference byte-exact reversal on D | BLOCKED · bp-75y | BLOCKED · bp-75y |
| C / VM byte-exact reversal on D | BLOCKED · bp-75y | BLOCKED · bp-75y |
| Rust / toolchain | PASS | PASS |
| Rust / fixtures | PASS | PASS |
| Rust / plugin build | PASS | PASS |
| Rust / VM build | PASS | PASS |
| Rust / native O0 = O2 | PASS | PASS |
| Rust / frontend | PASS | PASS |
| Rust / runtime bitcode link | FAIL · bennett-conformance-oc2 | PASS |
| Rust / post-link LTO | BLOCKED · bennett-conformance-oc2 | PASS |
| Rust / pipeline | BLOCKED · bennett-conformance-oc2 | FAIL · bp-xb8 |
| Rust / materialised native comparison | BLOCKED · bennett-conformance-oc2 | BLOCKED · bp-xb8 |
| Rust / reference admission | BLOCKED · bennett-conformance-oc2 | BLOCKED · bp-xb8 |
| Rust / VM admission | BLOCKED · bennett-conformance-oc2 | BLOCKED · bp-xb8 |
| Rust / reference input exhaustive = native (diagnostic) | BLOCKED · bennett-conformance-oc2 | BLOCKED · bp-xb8 |
| Rust / VM input exhaustive = native (diagnostic) | BLOCKED · bennett-conformance-oc2 | BLOCKED · bp-xb8 |
| Rust / reference input zero faults (diagnostic) | BLOCKED · bennett-conformance-oc2 | BLOCKED · bp-xb8 |
| Rust / VM input zero faults (diagnostic) | BLOCKED · bennett-conformance-oc2 | BLOCKED · bp-xb8 |
| Rust / reference input byte-exact reversal (diagnostic) | BLOCKED · bennett-conformance-oc2 | BLOCKED · bp-xb8 |
| Rust / VM input byte-exact reversal (diagnostic) | BLOCKED · bennett-conformance-oc2 | BLOCKED · bp-xb8 |
| Rust / reference full D interface | BLOCKED · bennett-conformance-oc2 | BLOCKED · bp-xb8 |
| Rust / VM full D interface | BLOCKED · bennett-conformance-oc2 | BLOCKED · bp-xb8 |
| Rust / reference exhaustive full D = native | BLOCKED · bennett-conformance-oc2 | BLOCKED · bp-xb8 |
| Rust / VM exhaustive full D = native | BLOCKED · bennett-conformance-oc2 | BLOCKED · bp-xb8 |
| Rust / reference zero faults on D | BLOCKED · bennett-conformance-oc2 | BLOCKED · bp-xb8 |
| Rust / VM zero faults on D | BLOCKED · bennett-conformance-oc2 | BLOCKED · bp-xb8 |
| Rust / reference byte-exact reversal on D | BLOCKED · bennett-conformance-oc2 | BLOCKED · bp-xb8 |
| Rust / VM byte-exact reversal on D | BLOCKED · bennett-conformance-oc2 | BLOCKED · bp-xb8 |
| Julia / toolchain | PASS | PASS |
| Julia / fixtures | PASS | PASS |
| Julia / plugin build | PASS | PASS |
| Julia / VM build | PASS | PASS |
| Julia / native O0 = O2 | PASS | PASS |
| Julia / frontend | PASS | PASS |
| Julia / runtime bitcode link | FAIL · bennett-conformance-oc2 | PASS |
| Julia / post-link LTO | BLOCKED · bennett-conformance-oc2 | PASS |
| Julia / pipeline | BLOCKED · bennett-conformance-oc2 | FAIL · bp-zeg |
| Julia / materialised native comparison | BLOCKED · bennett-conformance-oc2 | BLOCKED · bp-zeg |
| Julia / reference admission | BLOCKED · bennett-conformance-oc2 | BLOCKED · bp-zeg |
| Julia / VM admission | BLOCKED · bennett-conformance-oc2 | BLOCKED · bp-zeg |
| Julia / reference input exhaustive = native (diagnostic) | BLOCKED · bennett-conformance-oc2 | BLOCKED · bp-zeg |
| Julia / VM input exhaustive = native (diagnostic) | BLOCKED · bennett-conformance-oc2 | BLOCKED · bp-zeg |
| Julia / reference input zero faults (diagnostic) | BLOCKED · bennett-conformance-oc2 | BLOCKED · bp-zeg |
| Julia / VM input zero faults (diagnostic) | BLOCKED · bennett-conformance-oc2 | BLOCKED · bp-zeg |
| Julia / reference input byte-exact reversal (diagnostic) | BLOCKED · bennett-conformance-oc2 | BLOCKED · bp-zeg |
| Julia / VM input byte-exact reversal (diagnostic) | BLOCKED · bennett-conformance-oc2 | BLOCKED · bp-zeg |
| Julia / reference full D interface | BLOCKED · bennett-conformance-oc2 | BLOCKED · bp-zeg |
| Julia / VM full D interface | BLOCKED · bennett-conformance-oc2 | BLOCKED · bp-zeg |
| Julia / reference exhaustive full D = native | BLOCKED · bennett-conformance-oc2 | BLOCKED · bp-zeg |
| Julia / VM exhaustive full D = native | BLOCKED · bennett-conformance-oc2 | BLOCKED · bp-zeg |
| Julia / reference zero faults on D | BLOCKED · bennett-conformance-oc2 | BLOCKED · bp-zeg |
| Julia / VM zero faults on D | BLOCKED · bennett-conformance-oc2 | BLOCKED · bp-zeg |
| Julia / reference byte-exact reversal on D | BLOCKED · bennett-conformance-oc2 | BLOCKED · bp-zeg |
| Julia / VM byte-exact reversal on D | BLOCKED · bennett-conformance-oc2 | BLOCKED · bp-zeg |
