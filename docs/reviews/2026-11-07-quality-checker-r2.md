# Quality-checker review — round 2

**BLOCK — P1-1 (oracle), P1-2 (rung 4).** Reviewed head
a00ab1876db7da9861dd6313ab55b9be6f71b3ed. Identified the batch with
git log --reverse main..job/quality-checker and
git diff main...job/quality-checker (main
e39501e30bcbc0e67046fce24d3354f775a8f78f); this final round closely reviews
only c45b16fe68a56ef67b3f234dce9ef27579599a15..job/quality-checker
(07beada and a00ab18), plus the round-1 blocker reproductions.

**Round-1 P1-1, bennett-conformance-mge: its exact witnesses are fixed.**
Both "regs { unused:i04 }" and "bounds { 2 }" now reject with
E_CERT_OBJECT at §16.1. The canonical four-row identity still receives
overall verified and claim proof, and the pinned interpreter accepts it.
The broader admission bug class remains open below.

**P1-1 — NAME atoms bypass mandatory numeric contexts
(oracle), bennett-conformance-7v5.**
checker/src/object/lexical.rs:296 validates numbers only for
Atom::Numeric; :265 classifies alphabetic spellings as words, and :308
accepts them. This object still receives **verified/proof, no errors**:

~~~text
perm_action @a (t:wire mut) (c:wire[1] in) () () ()
regs {  } bounds {2}
^entry():
qloop 2 {
cx c t
}
action @unused (w:wire mut) () () () ()
^entry():
rx NaN w
~~~

The pinned interpreter rejects the identical bytes with
"§16.1: angle must be finite FLOAT"; inf behaves identically.
Replacing the last line with "rx 1.0 w" retains verification and interpreter
acceptance. The certificate binds a, with independently specified
identity rows 00→00, 01→01, 10→10, 11→11, full one-bit domains for t/c,
empty borrowed/region components, explicit args=[] and four zero budget
components. Signatures: a has mut=[1], in=[1], bounds=[2]; unused has
mut=[1], in=[], bounds=[]; both have args=[], mem=[], abi=0.9, tier=q and
their displayed kind. Symbol hashes, all four certificate bindings, region
key and claim subject were refreshed. Full recipe is in the bead.

**P1-2 — indexed control prefixes reject canonical region calls
(rung 4), bennett-conformance-7jy.**
checker/src/object/lexical.rs:216 skips exactly two tokens for ctl,
although ctl c[0] occupies five. It selects "[" as the opcode, and
:348 rejects it as "noncanonical dotted opcode". In the witness above,
retain the canonical unused action, replace "cx c t" with
"ctl c[0] action_call @flip (t) () () ()", and append:

~~~text
perm_action @flip (x:wire mut) () () () ()
^entry():
x x
~~~

Flip's signature has mut=[1], in=[], bounds=[], args=[], mem=[], abi=0.9,
kind=perm_action, tier=q. Two controlled flips independently yield
the same identity table. After refreshing symbol hashes through the pinned
interpreter's black-box hash command, its check **accepts** the object;
the checker rejects it with E_CERT_OBJECT at §16.1. The scalar ctl c
control verifies and is interpreter-accepted. This narrows canonical
controlled region calls needed by rung 4. Main-side parsing has no new
lexical precheck; its evaluator passes the intact whitespace token c[0]
to single_wire. The new prefix accounting causes the rejection.

Both findings were run with the stdin-compiled, independent-row probe:

~~~sh
timeout 60s "$CARGO_TARGET_DIR/review-proof-probe"
~~~

**Differential integrity.** The test is real: compare_candidate at
checker/tests/lexical_differential.rs:484 compares parser admission in
both directions; :491 additionally forbids verified/proof for
interpreter-rejected candidates. The oracle is the validated gate-pinned
release binary, invoked only through hash/check; independent hash
rebinding prevents stale metadata from masking accepted mutations.
However, covering eight role names does **not** cover every numeric or
punctuation context. numeric_spellings at :442 omits alphabetic
substitutions, and the seed contains no indexed ctl prefix.

To check the detector itself without editing repository sources, compiled
an in-memory copy of that same differential with only "NaN".into() and
"inf".into() added to its mutation vector. Running

~~~sh
timeout 30s "$CARGO_TARGET_DIR/review-extended-differential" generated_lexical_admission --test-threads=4 --nocapture
~~~

fails as expected (exit 101): **2,037 candidates, 70 admission/proof
disagreements**. Mandatory allocation/region/count/index sites in unused
functions also admit these spellings and grant proofs. Thus the unchanged
1,933-case pass does not establish complete lexical admission.

Validation used DISPLAY/WAYLAND_DISPLAY/DBUS_SESSION_BUS_ADDRESS/BROWSER
unset, TMPDIR=$PWD/.tmp/rev-quality-checker-r2/tmp,
CARGO_TARGET_DIR=$PWD/.tmp/rev-quality-checker-r2/target, and
BENNETT_ISA_SPEC=$PWD/.tmp/gate/cache/bennett-isa-spec@3b1090790d82/bennett-isa-spec
(pin 3b1090790d82a00ac95d30b8da6b42d1d7a9f5d1):

~~~sh
timeout 180s cargo test --offline --manifest-path checker/Cargo.toml -j3 --test conformance lexical -- --test-threads=4 --nocapture
timeout 150s cargo test --offline --manifest-path checker/Cargo.toml -j3 --test conformance -- control_flow:: mcx:: numerical:: signatures:: --test-threads=4
timeout 30s cargo fmt --manifest-path checker/Cargo.toml -- --check
timeout 60s python3 -B scripts/quality_ratchet.py --self-test
timeout 60s python3 -B scripts/quality_ratchet.py
git diff --check main...job/quality-checker
~~~

Results: **5 + 42 = 47 existing tests pass**; unchanged matrix reports
52 numeric sites, 183 punctuation sites, 1,933 candidates, 4 accepted,
zero disagreements. Formatting, whitespace and both ratchet checks pass.
Read main/pre-fix code to assess red-first behavior; did not switch branches
or execute tests at main. No full gate, sibling gate, full-suite timing or
fresh coverage run. No implementation edits.

P2+ nits: none newly filed; prior round's nits are outside this fix-only review.

Coverage: closely read the complete fix diff, lexical reader, generated
differential, all changed tests, certificate/hash helpers and relevant
parser/verdict/control paths; compared pinned ISA §§16.1–16.2 and main-side
code. Checked every COMMON anti-slop category; the demonstrated gaps concern
semantic admission and tests missing their bug class. Skimmed unchanged
semantics and remaining batch context already reviewed in round 1.
Provenance: implementer **gpt-6.1-sol**; reviewer **gpt-6.1-sol, fresh session**, D9.

