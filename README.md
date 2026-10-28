# bennett-conformance

The demo ladder as executable gates, plus the tooling to judge any
implementation of the specs. A **consumer only**: nothing depends on it, and it
depends on everything through published contracts and binary edges.

- `ladder/rung-1` ... `ladder/rung-8` acceptance tests from North Star section 8.
- `checker/` Rust crate `bennett-cert-checker`: certificate checker (ISA spec section 12).
- legality corpus runner (dialect spec corpus; future).
- `harness/` differential harness against native execution, with instrumented
  classifiers (UBSan / ASan / MSan) deciding which inputs are in the defined domain.
