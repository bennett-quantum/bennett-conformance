# clauses/: the clause dependency graph of the Bennett specs

`clausegraph.py` turns a spec markdown file into a hierarchical clause graph with
dependency edges, so we can ask "what depends on this clause" and "are these two
changes independent", track a spec across versions without editing its bytes, and
measure which clauses have no test or proof witness. Stdlib-only Python; always run
it as `python3 -I`. Design, parser rules, measured results and limits:
[`docs/design/clause-graph.md`](../docs/design/clause-graph.md).

## Committed graphs

| File | Source | Built with |
|---|---|---|
| `ISA-0.8.json`, `.dot` | `bennett-isa-spec/spec/Bennett-ISA-Spec.md` (accepted 0.8) | `build --spec ISA --version 0.8` |
| `ISA-0.9-draft.json`, `.dot` | `bennett-isa-spec/spec/Bennett-ISA-Spec-0.9-draft.md` (working draft) | `build --spec ISA --version 0.9-draft` |
| `DLT-0.2.10.json`, `.dot` | `bennett-dialect-spec/spec/Bennett-Dialect-Spec.md` (0.2.10) | `build --spec DLT --version 0.2.10 --xspec clauses/ISA-0.8.json` |
| `<SPEC>-<version>.statements.json`, `.md` | the graph JSON of the same name | `statements clauses/<SPEC>-<version>.json` |

The `.statements.*` files are the statement layer (atomic Lamport-numbered statements,
below); each records the source sha256 of the graph it was derived from, and queries refuse
a stale one. Rebuild them after every `build`.

Each JSON records the source file's sha256 (`source.sha256`); a graph whose sha256 no
longer matches its source is stale and must be rebuilt (the test suite checks this for
the accepted ISA 0.8). The DLT graph also records the sha256 of the ISA graph it was
resolved against (`xspec`), and queries refuse to combine it with a different ISA graph.

## Usage

Run from the repository root, headless:

```sh
W=..   # the workspace holding the spec repositories
python3 -I clauses/clausegraph.py build --spec ISA --version 0.8 \
    --file $W/bennett-isa-spec/spec/Bennett-ISA-Spec.md
python3 -I clauses/clausegraph.py build --spec ISA --version 0.9-draft \
    --file $W/bennett-isa-spec/spec/Bennett-ISA-Spec-0.9-draft.md
python3 -I clauses/clausegraph.py build --spec DLT --version 0.2.10 \
    --file $W/bennett-dialect-spec/spec/Bennett-Dialect-Spec.md --xspec clauses/ISA-0.8.json

for g in ISA-0.8 ISA-0.9-draft DLT-0.2.10; do
    python3 -I clauses/clausegraph.py statements clauses/$g.json   # DLT: --xspec defaults to clauses/ISA-0.8.json
done

python3 -I clauses/clausegraph.py diff clauses/ISA-0.8.json clauses/ISA-0.9-draft.json [--list] [--json] [--statements]
python3 -I clauses/clausegraph.py impact clauses/DLT-0.2.10.json PT3 --with clauses/ISA-0.8.json
python3 -I clauses/clausegraph.py impact clauses/ISA-0.8.json ISA-5.6 --with clauses/DLT-0.2.10.json
python3 -I clauses/clausegraph.py independent clauses/DLT-0.2.10.json TDL-5 N21
python3 -I clauses/clausegraph.py coverage clauses/ISA-0.8.json --tags tags.json [--json] [--statements]
python3 -I clauses/clausegraph.py impact clauses/ISA-0.8.json ISA-5.1-P1.S3 --with clauses/DLT-0.2.10.json
python3 -I clauses/clausegraph.py independent clauses/ISA-0.8.json ISA-5.1-P1.S3 ISA-5.1-P1.S7 --containers ignore
python3 -I clauses/clausegraph.py top clauses/DLT-0.2.10.json -n 10 --with clauses/ISA-0.8.json
```

- `build` writes `<out-dir>/<SPEC>-<version>.json` and `.dot` (default out-dir: this
  directory). `--svg` additionally runs `dot -Tsvg` into the same directory when
  Graphviz is installed, and says so when it is not. No viewer is ever started.
- `impact` and `independent` take `--closure anchored|full` (default `anchored`) and
  `--max-depth N`; see "Dependency semantics" in the design doc. `--with` loads further
  graphs so that cross-spec edges are followed (ISA clauses gain dialect dependents).
- `independent` prints `independent` and exits 0, or prints `dependent` with the shared
  forward and reverse cones and exits 1. `--containers ignore` stops a citation of a
  section or clause that encloses both ids from making them dependent (the default,
  `count`, is the strict reading).
- `statements` writes `<SPEC>-<version>.statements.json` and `.md` next to the graph (or
  into `--out-dir`) and prints counts per kind; unresolved `[deps: ...]` markers are
  warnings, never failures.
- `--statements` (on `impact`, `independent`, `coverage`, `diff`) loads the sibling
  `.statements.json` of each graph; a statement id argument or tag implies it. With it,
  `impact` lists dependent statements, `coverage` adds statement-level strong / weak /
  uncovered counts per kind, and `diff` adds added / removed / moved / changed statements.
- Exit code 2: malformed spec (message `file:line: ...`), unknown id or alias, bad tags
  file, stale cross-spec combination. Unresolved references never fail a build; they
  are listed in the JSON `unresolved` array with a reason.

Any id argument may be a statement id (`ISA-5.1-P1.S3`), a clause id (`ISA-5.6-P2`), a section id (`ISA-5.6`), or an alias
(`PT3`, `TDL-5`, `borrow frames`); write `SPEC:alias` when an alias exists in two loaded
specs.

## ID scheme

- Root: the spec code, `ISA` or `DLT`.
- Section: `<SPEC>-<number>` from the heading: `ISA-5`, `ISA-5.6`, `DLT-12.2`; appendices
  are `DLT-A`, `DLT-B`, `DLT-C`; text before the first `##` heading is `<SPEC>-front`.
- Clause: `<SPEC>-<section>-P<ordinal>`, ordinal counted within the section (not its
  subsections) in document order: `ISA-5.2-P3`, `DLT-5.2-P4`. One clause per paragraph,
  list item (any nesting level) or table row; a fenced code block belongs to the clause
  before it.
- `hash`: first 12 hex digits of sha256 over the clause text with all whitespace runs
  collapsed to one space. Identical text gives an identical hash in every version.
- `aliases`: the rule labels (`PT3`, `N21`, `TDL-5`, `BD-E-RESIDUAL`, `C1`, `T3`) and bold
  defined terms (`borrow frames`, `lproc`) the clause defines. Unique per spec.

- Statement: `<clause-id>.S<k>`, the k-th sentence-level statement of the clause
  (`ISA-5.1-P1.S3`, Lamport `<1>3`); `<clause-id>.S<k>.<j>` is item `(j)` of an
  enumeration inside statement k (Lamport `<2>j`). Each has a kind (`requirement`,
  `definition`, `non-normative`, `example`, `uncertain`), the rule that chose it (K1-K16),
  a 12-hex hash, inferred dependencies and the spec's declared ones: a trailing
  `[deps: ISA-3.3-P2.S1, ISA-5.6]` marker in the sentence (0.9 draft onward). Splitting
  and classification rules: design doc, "Statement layer".

Clause ordinals are positional: inserting a paragraph renumbers the rest of its
section. Version tracking therefore goes through `diff` (hash first, then
(section, alias), then text similarity), never through the ordinal alone.

## Tagging tests and proofs

A tags file is a JSON object from a test or proof name to one id or a list of ids:

```json
{
  "ladder/rung1/inc8_exhaustive": ["ISA-5.2-P2", "ISA-5.2-P3"],
  "checker/borrow_frames_negative": ["borrow frames", "ISA-5.6-P2"],
  "proofs/T3_permutation": "T3",
  "corpus/legality_smoke": "DLT-3"
}
```

- A statement id is a **strong** witness for that statement (and its enumerated items),
  and for its clause in the clause-level report. Prefer statement ids for new tags.
- A clause id or an alias is a **strong** witness for that clause; at statement level it is
  **weak** coverage of each of the clause's statements.
- A section id counts for every clause under it (subsections included) but only as
  **weak** coverage; `coverage` reports such clauses separately from uncovered ones.
- Prefer aliases where a clause has one: they survive renumbering. For clauses without
  an alias, re-target tags after a spec revision with `diff --list`.
- Ids of another spec (e.g. `DLT-*` in a run against the ISA graph) are listed as
  ignored; an id that does not exist in the graph's spec is an error.

## Tests

```sh
mkdir -p .tmp
env -u DISPLAY -u WAYLAND_DISPLAY -u DBUS_SESSION_BUS_ADDRESS -u BROWSER TMPDIR=$PWD/.tmp \
    python3 -I -m unittest discover -s clauses/tests
```

`test_statements.py` covers the splitter (abbreviations, `§` references, decimals, code
spans, captions, markers, enumerations), each classifier rule and the uncertain cases,
declared-dependency parsing and statement-level queries, and checks that the committed
`.statements.json` files regenerate byte-identically from the committed graphs.
Synthetic snippets cover each parser rule and query; integration tests parse the real
specs (skipped, with a message, when the sibling spec repositories are absent) and check
sizes, `§5.6`, `PT3`, `N21`, `TDL-5`, at least 100 resolved dialect→ISA edges, and that
the committed `ISA-0.8.json` matches its source.
