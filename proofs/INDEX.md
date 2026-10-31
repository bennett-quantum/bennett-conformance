# Proof index

Spec: ISA 0.8 (`clauses/ISA-0.8.json`). Format: [`FORMAT.md`](FORMAT.md). Leaf counts are
BY / ASSUMED / OPEN inside the proof block (QED steps counted as BY).

| Theorem | Statement | Status | Leaves BY/ASSUMED/OPEN | Gaps |
|---|---|---|---|---|
| [INV-1](INV-1-matched-pairs.md) | Matched compute/uncompute pairs are the identity | proved to depth 3 except qif row | 42 / 3 / 1 (refuted) | 4 |
| [INV-2](INV-2-no-clone.md) | No clone, no alias (names, handles, states) | proved to depth 2 under readings | 22 / 4 / 0 | 4 |
| [INV-3](INV-3-discipline-soundness.md) | Soundness of the static uncomputation discipline (T5 + borrow frames) | perm and borrow proved to depth 3 under readings; `other` refuted | 30 / 7 / 2 (1 refuted) | 9 |
| [INV-4](INV-4-envelope-injective.md) | Identity envelope injective on structural layout leaves | holds except `interface_regions`; refuted for CC/R | 19 / 1 / 3 (2 refuted) + 1 outside proof | 5 |
| [INV-5](INV-5-schedule-selection.md) | Schedule selection soundness, admission exactness, §17 defaults as written values | (A), (B) proved to depth 3 for bound admission under readings; (B) refuted for unbound admission; (C) refuted for `epsilon_share` and uncovered defaults | 37 / 9 / 8 (4 refuted) | 17 |
| [INV-6](INV-6-certificate-soundness.md) | Certificate label soundness, evidence semantics, resource honesty (§12.1–12.2) | (A) proved to depth 3 under readings (name-set classifier refuted); (B) proved for exhaustive claims, sampled fraction undefined, "no conclusion outside" refuted for co-singleton bijections; (C) only as a reading: 0.8 has no host-limit outcome | 41 / 12 / 2 (1 refuted) | 12 |

## Statements cited

- **INV-1** (52, of which 1 whole): ISA-2-P9.S1, ISA-3.1-P2.S3, ISA-3.2-P8.S1, ISA-3.3-P2.S1, ISA-3.3-P2.S2, ISA-3.3-P3.S1, ISA-3.3-P3.S2, ISA-3.5-P1.S1, ISA-3.5-P1.S2, ISA-3.5-P1.S3, ISA-4.3-P1.S1, ISA-4.3-P2.S1, ISA-4.3-P3.S1, ISA-4.3-P4.S1, ISA-4.4-P1.S1, ISA-4.4-P2.S2, ISA-4.5-P1.S1, ISA-4.5-P2.S1, ISA-4.5-P3.S1, ISA-4.5-P4.S1, ISA-4.5-P5.S1, ISA-4.5-P5.S2, ISA-4.6-P1.S1, ISA-4.6-P2.S1, ISA-4.6-P3.S1, ISA-4.6-P4.S1, ISA-4.6-P5.S1, ISA-4.6-P6.S1, ISA-4.6-P7.S1, ISA-4.6-P8.S1, ISA-4.6-P9.S1, ISA-4.7-P1.S1, ISA-5.1-P1 (whole clause), ISA-5.1-P1.S7, ISA-5.2-P2.S1, ISA-5.2-P2.S2, ISA-5.2-P2.S3, ISA-5.2-P4.S1, ISA-5.3-P1.S1, ISA-5.3-P2.S1, ISA-5.3-P4.S1, ISA-5.3-P4.S2, ISA-5.3-P5.S1, ISA-5.4-P1.S4, ISA-5.5-P1.S1, ISA-5.5-P1.S3, ISA-5.6-P1.S1, ISA-5.6-P2.S1, ISA-5.6-P2.S2, ISA-6.2-P1.S2, ISA-14.2-P1.S1, ISA-14.2-P1.S2
- **INV-2** (37): ISA-2-P2.S1, ISA-2-P9.S1, ISA-3.1-P2.S3, ISA-3.2-P1.S1, ISA-3.3-P1.S1, ISA-3.3-P2.S1, ISA-3.3-P2.S3, ISA-3.3-P3.S1, ISA-3.3-P3.S2, ISA-3.3-P6.S2, ISA-3.3-P7.S1, ISA-3.3-P8.S1, ISA-3.3-P9.S1, ISA-3.3-P10.S1, ISA-3.3-P11.S1, ISA-3.3-P12.S1, ISA-3.3-P13.S1, ISA-3.3-P13.S2, ISA-3.3-P14.S2, ISA-3.5-P1.S1, ISA-3.7-P2.S1, ISA-4.3-P1.S1, ISA-4.3-P2.S1, ISA-4.4-P1.S1, ISA-4.5-P1.S1, ISA-4.6-P1.S1, ISA-4.7-P1.S2, ISA-5.1-P1.S1, ISA-5.1-P1.S3, ISA-5.1-P1.S5, ISA-5.1-P1.S6, ISA-5.2-P2.S3, ISA-5.3-P1.S1, ISA-6.1-P3.S1, ISA-6.2-P1.S3, ISA-10-P3.S2, ISA-14.2-P1.S2
- **INV-3** (42, of which 3 whole): ISA-2-P9.S1, ISA-3.3-P3.S1, ISA-3.5-P1.S1, ISA-3.5-P1.S3, ISA-4.3-P3.S1, ISA-4.4-P1.S1, ISA-4.5-P3.S1, ISA-4.6-P1.S1, ISA-5.1-P1 (whole clause), ISA-5.1-P1.S5, ISA-5.2-P2.S1, ISA-5.3-P1.S1, ISA-5.3-P5.S1, ISA-5.6-P1.S1, ISA-5.6-P2.S1, ISA-5.6-P2.S2, ISA-5.6-P3.S1, ISA-5.6-P4.S1, ISA-5.6-P5.S1, ISA-5.6-P6.S1, ISA-5.6-P7.S1, ISA-6.1 (whole section), ISA-6.1-P1.S1, ISA-6.1-P1.S2, ISA-6.1-P1.S3, ISA-6.1-P2.S1, ISA-6.1-P3.S1, ISA-6.1-P9.S1, ISA-6.1-P10.S1, ISA-6.1-P11.S1, ISA-6.1-P14.S1, ISA-6.2-P1 (whole clause), ISA-6.2-P1.S1, ISA-6.2-P1.S2, ISA-6.2-P1.S4, ISA-6.3-P1.S1, ISA-6.4-P1.S1, ISA-6.4-P1.S2, ISA-6.4-P1.S4, ISA-14.2-P1.S1, ISA-14.3-P3.S1, ISA-14.3-P5.S1
- **INV-4** (14): ISA-9.1-P1.S1, ISA-9.1-P1.S3, ISA-9.1-P2.S1, ISA-9.2-P1.S3, ISA-9.2-P2.S1, ISA-9.2-P2.S2, ISA-9.2-P2.S3, ISA-9.2-P2.S4, ISA-9.2-P2.S5, ISA-9.2-P2.S6, ISA-9.2-P2.S7, ISA-16.1-P1.S2, ISA-16.1-P1.S3, ISA-16.2-P1.S1
- **INV-5** (42): ISA-3.2-P10.S1, ISA-3.6-P1.S1, ISA-3.6-P4.S1, ISA-3.6-P6.S1, ISA-3.6-P11.S1, ISA-3.7-P20.S2, ISA-4.2-P1.S2, ISA-4.2-P21.S1, ISA-4.6-P8.S1, ISA-4.6-P9.S1, ISA-4.7-P1.S1, ISA-5.1-P1.S7, ISA-5.2-P1.S1, ISA-5.2-P2.S2, ISA-5.2-P3.S1.2, ISA-5.2-P3.S1.4, ISA-5.4-P1.S3, ISA-8.1-P1.S1, ISA-8.1-P10.S1, ISA-8.1-P10.S2, ISA-9.2-P2.S13, ISA-10-P1.S4, ISA-10-P4.S1, ISA-10-P7.S2, ISA-11-P2.S1, ISA-11-P2.S2, ISA-11-P2.S3, ISA-11-P3.S2, ISA-11-P4.S1, ISA-11-P4.S2, ISA-11-P4.S3, ISA-11-P4.S4, ISA-11-P4.S5, ISA-11-P4.S6, ISA-11-P4.S9, ISA-12.1-P2.S1, ISA-17-P7.S2, ISA-17-P8.S2, ISA-17-P10.S1, ISA-17-P10.S2, ISA-17-P11.S1, ISA-17-P14.S2
- **INV-6** (35): ISA-1.2-P4.S1, ISA-2-P1.S2, ISA-2-P7.S1, ISA-3.3-P1.S1, ISA-3.3-P3.S2, ISA-3.3-P14.S1, ISA-4.3-P2.S1, ISA-4.4-P1.S1, ISA-4.4-P2.S1, ISA-5.1-P1.S1, ISA-5.1-P1.S2, ISA-5.2-P1.S1, ISA-5.2-P2.S1, ISA-5.2-P2.S2, ISA-5.2-P2.S3, ISA-5.2-P4.S1, ISA-5.3-P1.S1, ISA-5.3-P1.S4, ISA-5.6-P1.S1, ISA-5.6-P2.S2, ISA-9.2-P1.S1, ISA-9.2-P2.S15, ISA-9.4-P2.S1, ISA-9.4-P3.S1, ISA-12.1-P1.S1, ISA-12.1-P2.S1, ISA-12.1-P5.S1, ISA-12.1-P7.S5, ISA-12.1-P8.S1, ISA-12.2-P1.S1, ISA-12.2-P1.S2, ISA-12.3-P2.S3, ISA-14.1-P1.S1, ISA-14.2-P1.S2, ISA-14.3-P3.S1

Steps cite statement ids (`ISA-5.3-P4.S2`); a clause or section id appears only where a step
needs the whole clause or section, marked `(whole clause)` / `(whole section)`. Statement ids
count as strong coverage at statement level; whole-clause ids as weak. The machine-readable form
is [`tags.json`](tags.json) (`{"INV-n": [ids]}`, generated from every id the proof cites outside
its Witnesses table). Classifier problems met while retagging (statements kept `uncertain` or
misclassified): [`STATEMENT-NOTES.md`](STATEMENT-NOTES.md).

## Coverage

```
env -u DISPLAY -u WAYLAND_DISPLAY -u DBUS_SESSION_BUS_ADDRESS -u BROWSER TMPDIR=$PWD/.tmp \
    python3 -I clauses/clausegraph.py coverage clauses/ISA-0.8.json --tags proofs/tags.json --statements
```

Result (2026-10-31, after INV-6, proofs only, statement level): `statements of ISA-0.8: 541; strong 169, weak (clause or section tag) 10, uncovered 362`
(clause level, same run: `314 clauses; strong 112, weak (section tag only) 9, uncovered 193`).

## Refuted steps (spec text false as written)

- INV-1 1.3.6.4: the `qif` row of the Adjoint table (`ISA-5.3-P4.S2`) is ill-formed for cross-arm lifetimes.
- INV-3 1.4.2: `other` values released by their adjoint while a reader is outstanding are not clean (`ISA-6.2-P1.S4`).
- INV-4 1.5.2: (5b) has no tie order for coincident dotted paths, so CC/R envelopes are not unique (`ISA-9.2-P1.S3`).
- INV-4 1.5.3: coincident dotted paths make the CC/R envelope non-injective even with a tie order (`ISA-9.2-P1.S3`, `ISA-16.1-P1.S2`).
- INV-5 1.1.3: the invocation chain through `lift.call` is not a function of the generated operation (`ISA-11-P4.S1`).
- INV-5 1.4.3: no admission that sees only the object and the schedule decides the maximal-match condition, because nested token budgets are unknown (`ISA-11-P4.S9`, `ISA-9.2-P2.S13`).
- INV-5 1.6.4.3: the default `epsilon_share` ε / N is in general no finite decimal, so it has no written equivalent (`ISA-17-P10.S2`).
- INV-5 1.6.7: defaults of uncovered nested operations cannot be written as entries without changing other operations (`ISA-11-P4.S3`, `ISA-11-P4.S6`); this limits the theorem, not the spec.
- INV-6 1.6.8: agreement on all but one element of D forces agreement on the last when both sides are bijections of D, so "Evidence implies nothing outside the enumerated elements" is false as mathematics (`ISA-12.2-P1.S2`); this limits the theorem, not the spec.
