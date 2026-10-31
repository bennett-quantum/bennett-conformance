# INV-2: no clone, no alias

Spec: ISA 0.8 (`clauses/ISA-0.8.json`). Format: `proofs/FORMAT.md`.

## Informal statement

The ISA never duplicates quantum state, at either level where duplication could
happen. By name: in structured code every wire belongs to exactly one live
linear value, and in CC every device wire is reachable through at most one usable
`qreg` handle, so no instruction sees a wire twice or under two names. By state:
every Q semantics is linear and isometric, so no instruction sequence maps
ψ ⊗ |0⟩ to ψ ⊗ ψ for two distinct non-orthogonal ψ. The instructions that
"copy" (`qcopy`, `qload`, oracle `tgt` writes) copy computational-basis values
only: they entangle target with source, and a copy into workspace is a `perm`
value bound to be uncomputed before the region exits.

## Precise statement

DEFINE `Seg`, `[[S]]_{Π,κ}` as in INV-1. DEFINE for a program point p of a
well-formed region body: `Live(p)` = the set of live linear wire values
(`ISA-3.3-P2.S1`), and `wires(v)` = the physical wires of value v. DEFINE for a CC
program point: a handle is *usable* iff it is live (`ISA-3.3-P6.S2`).

THEOREM INV-2.

- (A) No alias in structured code. For every well-formed region body and point
  p, the sets `wires(v)`, v ∈ Live(p), are pairwise disjoint, and every v ∈
  Live(p) has exactly one producer: an interface binding, an allocation
  (`qalloc`, `qborrow`, `qregion`), a `produce`-mode result, or the successor
  of an `in`-mode operand.
- (B) No alias inside an instruction. The wires of the wire operands of one
  instruction are pairwise disjoint and disjoint from its effective control
  context.
- (C) No alias of CC handles. At every CC point, the wire sets of usable handles
  are pairwise disjoint.
- (D) No clone. There are no S ∈ Seg, Π, κ, wires a, b (b allocated clean by S
  or a clean bundle), and states ψ1, ψ2 of a with 0 < |⟨ψ1|ψ2⟩| < 1 such that
  `[[S]](ψi ⊗ |0⟩_b ⊗ e) = ψi ⊗ ψi ⊗ e_i` for i = 1, 2.
- (E) Basis copies are entangled and obligated. `d = qcopy s` maps
  Σ_x α_x |x⟩_s |0⟩_d to Σ_x α_x |x⟩_s |x⟩_d, which equals ψ ⊗ ψ only when ψ
  is a basis state; d is a `perm` value whose dependency set contains s, and
  d must be released clean before the region exits.

## Hypotheses

ISA-2-P2.S1, ISA-2-P9.S1, ISA-3.2-P1.S1, ISA-3.3-P1.S1, ISA-3.3-P2.S1, ISA-3.3-P2.S3,
ISA-3.3-P3.S1, ISA-3.3-P3.S2, ISA-3.3-P6.S2, ISA-3.3-P7.S1, ISA-3.3-P8.S1,
ISA-3.3-P9.S1, ISA-3.3-P10.S1, ISA-3.3-P11.S1, ISA-3.3-P12.S1, ISA-3.3-P13.S1,
ISA-3.3-P13.S2, ISA-3.3-P14.S2, ISA-3.5-P1.S1, ISA-3.7-P2.S1, ISA-4.3-P1.S1,
ISA-4.3-P2.S1, ISA-4.4-P1.S1, ISA-4.5-P1.S1, ISA-4.6-P1.S1, ISA-4.7-P1.S2,
ISA-5.1-P1.S1, ISA-5.1-P1.S3, ISA-5.1-P1.S5, ISA-5.1-P1.S6, ISA-5.2-P2.S3,
ISA-5.3-P1.S1, ISA-6.1-P3.S1, ISA-6.2-P1.S3, ISA-10-P3.S2, ISA-14.2-P1.S2.

## Proof

```
<1>1. (A) holds at every point of a well-formed region body.
      PROOF:
  <2>1. At entry, Live = the interface values, and "A region's interface
        bundles and regions are pairwise disjoint".
        BY ISA-3.3-P3.S2, ISA-5.1-P1.S1
  <2>2. A value is "produced once and consumed once"; `in` mode consumes and
        re-produces a successor on the same wires; "a name is not used after
        `consume`".
        BY ISA-3.3-P2.S1
  <2>3. `qalloc` and `qborrow` add new tensor factors to the region state, so
        their wires are disjoint from every live value's wires; `qregion`
        cells likewise at function entry.
        BY ISA-14.2-P1.S2, ISA-4.3-P1.S1, ISA-4.3-P2.S1, ISA-4.6-P1.S1
  <2>4. The producers listed in (A) are the only producers; in particular a
        `regs` declaration in an action or permutation action produces no
        wire value.
        ASSUMED: `ISA-3.3-P1.S1` says `reg[N]` is "a location (oracle bodies,
        lprocs); not linear" and `ISA-3.7-P2.S1` says oracles and lprocs declare
        `regs`, but the `regs` grammar admits any type in any function and no
        clause forbids a wire-typed `regs` entry in an action (council
        SOL-R11-B01).
  <2>5. An indexed operand `w[i]` "borrows wire i for one instruction" and
        creates no new live value.
        BY ISA-3.3-P2.S3
  <2>6. QED
        BY <2>1, <2>2, <2>3, <2>4, <2>5; induction over program points
<1>2. (B) holds for every instruction of a well-formed body.
      PROOF:
  <2>1. "The wire operands of one instruction are pairwise distinct; every
        wire of the effective control context is distinct from every target
        and from every consumed or produced operand".
        BY ISA-3.3-P3.S1, ISA-2-P9.S1
  <2>2. "Distinct" is about wires, not operand spellings: a bundle `w` and its
        indexed wire `w[i]` (or two indexed operands `w[i]`, `w[i]`) in one
        instruction overlap and are excluded.
        ASSUMED: `ISA-3.3-P3.S1` does not say whether distinctness is judged on
        names or on the wires they denote; only the wire reading makes the gate
        semantics of `ISA-4.4-P1.S1` well-defined (a `cx a a`
        has no unitary).
  <2>3. A wire of the effective control context may still be read by the
        instruction as a source or `in` operand (it is neither target nor
        consumed nor produced).
        BY ISA-3.3-P3.S1
  <2>4. QED
        BY <2>1, <2>2, <2>3 (reads of control wires are not aliases: no wire
        is both written and read under two names)
<1>3. (C) holds at every CC point.
      PROOF:
  <2>1. "A handle is in exactly one state: live, viewed … or consumed"; an
        instruction whose precondition fails is a CC trap.
        BY ISA-3.3-P6.S2
  <2>2. Each transition preserves disjointness of usable wire sets:
        `qreg.new` gives fresh wires; `qreg.slice` makes the parent viewed
        (unusable: "any use of q while viewed is ill-formed") and the view
        live over a subset; after its one region call the view is consumed and
        the parent live again; `qreg.concat` consumes both parts;
        `qreg.free` and `measure` consume; a region call leaves q unchanged.
        BY ISA-3.3-P7.S1, ISA-3.3-P8.S1, ISA-3.3-P9.S1, ISA-3.3-P10.S1,
        ISA-3.3-P11.S1, ISA-3.3-P12.S1
  <2>3. Reversal of a `qreg.*` frame restores the handle states before that
        frame, where disjointness held; a `measure` frame is never reversed.
        BY ISA-3.3-P13.S1, ISA-3.3-P13.S2
  <2>4. `qreg.new` wires are distinct from the wires of every other live or
        viewed handle, and host-supplied register ids name pairwise disjoint
        registers.
        ASSUMED: `ISA-3.3-P7.S1` says only "q live, clean"; `ISA-3.3-P14.S2` makes
        host ids `qreg` values and `ISA-10-P3.S2` gives a run exclusive ownership
        of the registers it touches, but no clause says two ids passed to one
        run denote disjoint wires.
  <2>5. Handle *values* can be duplicated by classical dataflow (CC is dynamic
        SSA), so the same handle can reach two operand slots of one region
        call; this is an alias of the call's interface bundles.
        BY ISA-3.2-P1.S1, ISA-4.7-P1.S2, ISA-3.3-P11.S1
  <2>6. Such a call is a CC trap with no effect.
        ASSUMED: `ISA-3.3-P3.S2` makes overlapping interface bundles ill-formed
        in Q, and `ISA-3.3-P6.S2` traps failed handle preconditions, but the
        region-call row `ISA-3.3-P11.S1` lists only "q live (a view included)" as
        precondition; disjointness of the call's qreg operands is not stated
        as a CC precondition.
  <2>7. QED
        BY <2>1, <2>2, <2>3, <2>4, <2>6; induction over CC execution
<1>4. (D) no clone.
      PROOF:
  <2>1. [[S]] is linear and isometric on the states where every release in S
        is of a clean bundle: gates are unitary, `qalloc` tensors |0⟩,
        `qfree` removes clean factors, borrowed factors see U ⊗ I, oracle and
        action calls are unitary, and no region contains `measure`.
        BY ISA-14.2-P1.S2, ISA-4.4-P1.S1, ISA-5.2-P2.S3, ISA-5.3-P1.S1,
        ISA-5.1-P1.S3, ISA-3.5-P1.S1
  <2>2. If [[S]](ψi ⊗ 0 ⊗ e) = ψi ⊗ ψi ⊗ e_i for i = 1, 2 then
        ⟨ψ1|ψ2⟩ = ⟨ψ1|ψ2⟩² ⟨e_1|e_2⟩, so |⟨ψ1|ψ2⟩| ≤ |⟨ψ1|ψ2⟩|², i.e.
        |⟨ψ1|ψ2⟩| ∈ {0} or ≥ 1.
        BY <2>1; algebra
  <2>3. QED
        BY <2>2 (contradicts 0 < |⟨ψ1|ψ2⟩| < 1)
<1>5. (E) basis copies.
      PROOF:
  <2>1. `d = qcopy s` allocates d clean and applies `cx s[i] d[i]` under the
        predicate, giving Σ α_x |x⟩|x⟩; this is a product state iff at most
        one α_x ≠ 0.
        BY ISA-4.5-P1.S1, ISA-3.5-P1.S1; algebra
  <2>2. d has provenance perm with S = {s} ∪ Π.
        BY ISA-6.1-P3.S1
  <2>3. "Every perm value MUST be cleaned within its window and before its
        region exits", and every `qalloc` is matched by a `qfree` of a clean
        bundle; so no workspace copy survives the region.
        BY ISA-6.2-P1.S3, ISA-5.1-P1.S5, ISA-5.3-P1.S1
  <2>4. `in` wires of actions are readable only as controls, diagonal targets,
        `qcopy`/`qxor` sources or `in` call arguments, so a copy never
        modifies its source.
        BY ISA-5.1-P1.S6
  <2>5. QED
        BY <2>1, <2>2, <2>3, <2>4
<1>6. QED
      BY <1>1, <1>2, <1>3, <1>4, <1>5
```

## Witnesses

| Witness | Steps |
|---|---|
| `suite/cases/registers` (concat/free, views through region calls, unclean free traps, measurement) | 1.3 |
| `suite/cases/actions` (`flip`: d = p copied, XORed into t, uncomputed) | 1.5 |
| `tests/structured.rs`: `tier_arity_aliasing_and_input_roles`, `cleanup_windows_and_linear_lifetimes` | 1.1, 1.2, 1.5.4 |
| `tests/parser.rs`: `r11_action_regs_do_not_produce_wires`, `r11_register_types_follow_function_kind`; `tests/cli_xam.rs::r11_cli_action_regs_reject_before_execution` | 1.1.4 (ASSUMED) |
| `tests/discipline.rs::release_cannot_consume_its_effective_control` | 1.2.1 |
| `tests/registers.rs`: `register_handles_enforce_views_consumption_and_cleanliness`, `viewed_parent_and_view_only_one_call_are_static_errors`, `allocation_views_and_region_calls_reverse_with_their_frames`, `r4_dynamic_interface_alias_is_an_atomic_reversible_cc_and_r_fault`, `r4_dynamic_control_target_alias_is_an_atomic_reversible_cc_and_r_fault`, `r4_dynamic_control_can_overlap_a_readonly_input`, `r4_dynamic_oracle_input_output_alias_is_an_atomic_reversible_cc_and_r_fault` | 1.3 (1.3.6 ASSUMED), 1.2.3 |
| `tests/quantum.rs`: `actions_copy_cleanup_quantum_memory_loops_and_adjoint`, `sol_r5_b03_consumed_workspace_names_keep_their_own_widths` | 1.5, 1.1.2 |

## Council blockers as instances

- SOL-R11-B01 (an action obtains a live wire from its `regs` line with no
  producer): a direct violation of (A)'s single-producer clause, admitted
  because step 1.1.4 is not in the spec.
- R11-B01 (an action `regs` name used as a rotation angle): not an alias, but
  the same missing kind restriction as 1.1.4, on the classical side
  (`ISA-3.1-P2.S3` "classical arguments fixed at region entry").

## Spec gaps

1. (1.1.4, next to `ISA-3.7-P2` (`ISA-3.7-P2.S1`) and
   `ISA-3.3-P1` (`ISA-3.3-P1.S1`)) ASSUMED. Needed sentence: "Only
   oracles and lprocs declare locations; the `regs` list of an action, permutation
   action, CC or R function MUST be empty (it may carry `bounds`)."
2. (1.2.2, next to `ISA-3.3-P3` (`ISA-3.3-P3.S1`)) ASSUMED. Needed sentence: "Distinctness is
   judged on wires: a bundle operand and an indexed operand of it, or two
   indexed operands naming the same wire, are not distinct."
3. (1.3.4, next to `ISA-3.3-P7` (`ISA-3.3-P7.S1`) and `ISA-3.3-P14`
   (`ISA-3.3-P14.S2`)) ASSUMED. Needed sentence:
   "`qreg.new` yields wires disjoint from every live or viewed register; the
   register ids passed to one run MUST denote pairwise disjoint registers, else
   `bennett_run` fails with `E_ARG`."
4. (1.3.6, next to `ISA-3.3-P11` (`ISA-3.3-P11.S1`)) ASSUMED. Needed sentence: "A region call in CC
   or R whose `qreg` operands share a wire is a CC trap before any effect."
