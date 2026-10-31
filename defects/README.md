# Defect reports

A defect is a place where a published spec (`bennett-isa-spec` or
`bennett-dialect-spec`, at the version in `SPEC_VERSIONS.toml`) is wrong,
ambiguous, or self-contradictory, so that a conforming implementation cannot be
determined from the text. A defect is not a change of design.

## Where defects live

Defects are tracked as beads issues in this repo, not as files. Every defect
issue carries:

- the label `defect`;
- exactly one spec label: `isa` or `dialect`;
- exactly one status label: `open`, `ready`, `resolved`, `errata`, `editorial`
  or `not-a-defect`.

Each defect's description names:

- the clause IDs affected, taken from `clauses/` (e.g. `ISA-5.2-P3`);
- the spec version it was found against (e.g. `0.8`);
- the quoted text or a reproduction;
- the proposed resolution, if any.

## Status labels

- `open`: reported, not yet triaged.
- `ready`: triaged; the fix is clear and goes into the next spec revision.
- `resolved`: fixed in a spec revision; name the revision in the notes.
- `errata`: a correction that applies retroactively to the accepted version,
  the way C++ Defect Reports do. Implementations claiming the old version must
  read the clause as corrected.
- `editorial`: wording only, no semantic change.
- `not-a-defect` (NAD): the text is right as written. Record why in the notes.

## Not defects

A proposal for new semantics is not a defect, and must not carry the `defect`
label. Proposals go through a council round on new material.

## Filing

Create a defect with its labels in one command:

```
bd create "<short title>" -l defect,<isa|dialect>,open \
  -d "Clauses: <clause IDs>. Spec version: <x.y[.z]>. Text/reproduction: <...>. Proposed fix: <...>."
```

Change status by replacing the status label, for example:

```
bd update <id> --remove-label open --add-label ready
```
