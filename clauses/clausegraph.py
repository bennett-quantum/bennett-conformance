#!/usr/bin/env python3
"""clausegraph: hierarchical clause graph with dependency edges for the Bennett specs.

Stdlib only; run as ``python3 -I clauses/clausegraph.py <command> ...``.
Design: docs/design/clause-graph.md. Usage: clauses/README.md.
"""
import argparse
import collections
import difflib
import hashlib
import json
import os
import re
import sys

SCHEMA = "bennett-clausegraph/1"


class SpecError(Exception):
    """Malformed input. Carries the offending line number (1-based) when known."""

    def __init__(self, msg, line=None, filename=None):
        self.msg = msg
        self.line = line
        self.filename = filename
        super().__init__(self.__str__())

    def __str__(self):
        loc = self.filename or "<input>"
        if self.line is not None:
            loc += ":%d" % self.line
        return "%s: %s" % (loc, self.msg)


class UsageError(Exception):
    """Bad query input (unknown id, malformed tags file)."""


# ---------------------------------------------------------------------------
# Lexical conventions (see docs/design/clause-graph.md, "What the parser recognises")

RE_H1 = re.compile(r"^# (\S.*)$")
RE_H2 = re.compile(r"^## (?:(\d+)\.|Appendix ([A-Z])\.)\s+(\S.*)$")
RE_H3 = re.compile(r"^### (\d+)\.(\d+)\s+(\S.*)$")
RE_ANY_HEADING = re.compile(r"^#{1,6}(\s|$)")
RE_FENCE = re.compile(r"^(\s*)(`{3,}|~{3,})(.*)$")
RE_HR = re.compile(r"^\s*(-{3,}|\*{3,}|_{3,})\s*$")
RE_LIST = re.compile(r"^(\s*)(?:([-*])|(\d+)\.)\s+")
RE_TABLE_SEP = re.compile(r"^\s*\|?\s*:?-{3,}:?\s*(\|\s*:?-{3,}:?\s*)*\|?\s*$")

# A rule label: PT3, N21, TDL-5, BD-E-RESIDUAL, BLK-1, E16.9, DB.
LABEL = r"[A-Z][A-Z0-9]*(?:[-.][A-Z0-9]+)*"
RE_LABEL_FULL = re.compile(r"^%s$" % LABEL)
# Table first cell that names a label: PT3, `BD-E-X`, **BLK-1** = ISA-4
RE_LABEL_CELL = re.compile(r"^(?:\*\*)?`?(%s)`?(?:\*\*)?(?:\s*=\s*.*)?$" % LABEL)
DEFINING_HEADERS = {"Id", "Rule", "Step", "Code", "Check", "Suite", "#"}
# Bold lead label: **TDL-5 Title.** / **C1 capture**: / **P-1.** / **BLOCK-IMAGE**
RE_BOLD_LEAD = re.compile(r"^\*\*(%s)(?=[\s.:*(])([^*]*)\*\*" % LABEL)
# List lead label: "- MC1: ..." or "- `BD-E-HANDLE-ESCAPE`: ..."
RE_LIST_LEAD = re.compile(r"^`?(%s)`?:\s" % LABEL)

RE_BOLD = re.compile(r"\*\*(.+?)\*\*", re.S)
TERM_STOP = {
    "every", "or", "and", "not", "no", "all", "only", "never", "any", "each", "both", "one",
    "none", "must", "may", "is", "are", "exactly", "also", "same", "either", "default",
    "Default", "MUST", "MUST NOT", "SHOULD", "MAY", "Date", "Status", "Deleted", "Note",
    "the", "a", "an", "in", "of", "to", "at", "per", "before", "after", "outside", "inside",
}
MAX_TERM_WORDS = 4
MAX_TERM_CHARS = 48

# Item markers: "(5b)", "(i)", "(a)" as list-item leads or inline enumerators.
ITEM = r"(?:\d{1,2}[a-z]?|[ivx]{1,4}|[a-z])"
RE_ITEM_LEAD = re.compile(r"^(?:\*\*)?\((%s)\)" % ITEM)
RE_ITEM_INLINE = re.compile(r"(?:(?<=^)|(?<=[\s;:,—(]))\((%s)\)(?=[\s,.;:)]|$)" % ITEM, re.M)

# Section references.
NUM = r"\d+(?:\.\d+)*"
RE_REF = re.compile(
    r"(?P<dbl>§§)(?P<list>%(n)s(?:\s*(?:,\s*and|,|/|and|&)\s*§?%(n)s)*)"
    r"|§(?P<one>%(n)s)(?P<tail>"
    r"(?:–§?(?P<rng>%(n)s))"
    r"|(?:\s?\((?P<pitem>%(i)s)\))"
    r"|(?:(?P<litem>[a-z])(?![\w']))"
    r"|(?:\s+(?:items?|rules?|parts?)\s+\(?(?P<witem>\d{1,2}[a-z]?)\)?"
    r"(?:(?:\s*(?:,|and|–)\s*)(?P<witem2>\d{1,2}[a-z]?))?)"
    r")?" % {"n": NUM, "i": ITEM})
RE_SEP_CARRY = re.compile(r"^\s*(?:,|/|–|and|or|,\s*and)\s*$")
RE_PREV_WORD = re.compile(r"(`[^`\s]+`|[A-Za-z][\w.\-]*|\d+(?:\.\d+)+)[\s(]*$")
SPEC_PREFIX = {"ISA": "ISA", "bennett-isa-spec": "ISA",
               "dialect": "DLT", "bennett-dialect-spec": "DLT"}
EXTERNAL_PREFIX = {"NS": "NS", "star": "north-star", "Bennett-NorthStar.md": "north-star",
                   "NorthStar": "north-star", "SQLite": "SQLite", "findings": "spike-findings",
                   "outline": "outline"}
RE_VERSION = re.compile(r"^\d+\.\d+(?:\.\d+)?$")

EDGE_DEP_KINDS = ("refs", "uses", "xspec")


def norm_ws(s):
    return " ".join(s.split())


def clause_hash(text):
    return hashlib.sha256(norm_ws(text).encode("utf-8")).hexdigest()[:12]


def split_cells(line):
    s = line.strip()
    if s.startswith("|"):
        s = s[1:]
    if s.endswith("|") and not s.endswith("\\|"):
        s = s[:-1]
    cells, cur, i = [], [], 0
    while i < len(s):
        c = s[i]
        if c == "\\" and i + 1 < len(s) and s[i + 1] == "|":
            cur.append("|")
            i += 2
            continue
        if c == "|":
            cells.append("".join(cur).strip())
            cur = []
        else:
            cur.append(c)
        i += 1
    cells.append("".join(cur).strip())
    return cells


# ---------------------------------------------------------------------------
# Parsing: lines -> sections and raw clauses


class Clause:
    __slots__ = ("section", "kind", "lines", "start", "end", "code_ranges", "item_marker",
                 "indent", "content_indent", "blank_pending", "table_header", "row_label",
                 "table_defining", "id", "aliases")

    def __init__(self, section, kind, start, indent=0, content_indent=0):
        self.section = section
        self.kind = kind
        self.lines = []          # (lineno, text)
        self.start = start
        self.end = start
        self.code_ranges = []    # [(first_lineno, last_lineno)] of attached code (fences incl.)
        self.item_marker = None
        self.indent = indent
        self.content_indent = content_indent
        self.blank_pending = False
        self.table_header = None
        self.row_label = None
        self.table_defining = False
        self.id = None
        self.aliases = []

    def add(self, lineno, text):
        self.lines.append((lineno, text))
        self.end = lineno

    @property
    def text(self):
        return "\n".join(t for _, t in self.lines)

    def is_code_line(self, lineno):
        return any(a <= lineno <= b for a, b in self.code_ranges)

    @property
    def prose(self):
        return "\n".join(t if not self.is_code_line(n) else "" for n, t in self.lines)

    @property
    def body(self):
        """First prose line stripped of the list marker (for lead detection)."""
        for n, t in self.lines:
            if self.is_code_line(n):
                continue
            s = t.strip()
            if self.kind == "item":
                m = RE_LIST.match(t)
                if m:
                    s = t[m.end():].strip()
            return s
        return ""


class Section:
    __slots__ = ("number", "title", "line", "parent", "clauses", "depth", "is_terminology")

    def __init__(self, number, title, line, parent, depth):
        self.number = number
        self.title = title
        self.line = line
        self.parent = parent
        self.depth = depth
        self.clauses = []
        self.is_terminology = title.strip().lower().startswith("terminology")


def parse_markdown(text, filename):
    lines = text.split("\n")
    if lines and lines[-1] == "":
        lines.pop()
    sections = collections.OrderedDict()
    front = Section("front", "front matter", 1, None, 1)
    sections["front"] = front
    cur_sec = front
    cur_h2 = None
    title = None
    cur = None  # open clause

    def close():
        nonlocal cur
        cur = None

    def new_clause(kind, lineno, indent=0, content_indent=0):
        nonlocal cur
        cur = Clause(cur_sec, kind, lineno, indent, content_indent)
        cur_sec.clauses.append(cur)
        return cur

    i = 0
    n = len(lines)
    while i < n:
        lineno = i + 1
        line = lines[i]
        fm = RE_FENCE.match(line)
        if fm:
            fence = fm.group(2)
            j = i + 1
            while j < n:
                cm = RE_FENCE.match(lines[j])
                if cm and cm.group(2)[0] == fence[0] and len(cm.group(2)) >= len(fence) \
                        and cm.group(3).strip() == "":
                    break
                j += 1
            if j >= n:
                raise SpecError("unterminated code fence opened here", lineno, filename)
            target = cur_sec.clauses[-1] if cur_sec.clauses else None
            if target is None:
                target = new_clause("code", lineno)
            for k in range(i, j + 1):
                target.add(k + 1, lines[k])
            target.code_ranges.append((lineno, j + 1))
            close()
            i = j + 1
            continue
        if RE_ANY_HEADING.match(line):
            close()
            m1, m2, m3 = RE_H1.match(line), RE_H2.match(line), RE_H3.match(line)
            if m1:
                if title is not None:
                    raise SpecError("second level-1 heading (title already at line %d)"
                                    % title[0], lineno, filename)
                title = (lineno, m1.group(1).strip())
            elif m2:
                num = m2.group(1) or m2.group(2)
                if num in sections:
                    raise SpecError("duplicate section %s (first at line %d)"
                                    % (num, sections[num].line), lineno, filename)
                cur_h2 = Section(num, m2.group(3).strip(), lineno, None, 1)
                sections[num] = cur_h2
                cur_sec = cur_h2
            elif m3:
                if cur_h2 is None or m3.group(1) != cur_h2.number:
                    raise SpecError("malformed heading: subsection %s.%s is not under section %s"
                                    % (m3.group(1), m3.group(2),
                                       cur_h2.number if cur_h2 else "(none)"), lineno, filename)
                num = "%s.%s" % (m3.group(1), m3.group(2))
                if num in sections:
                    raise SpecError("duplicate section %s (first at line %d)"
                                    % (num, sections[num].line), lineno, filename)
                cur_sec = Section(num, m3.group(3).strip(), lineno, cur_h2.number, 2)
                sections[num] = cur_sec
            else:
                raise SpecError("malformed heading (expected '# Title', '## N. Title', "
                                "'## Appendix X. Title' or '### N.M Title'): %r" % line,
                                lineno, filename)
            i += 1
            continue
        if line.strip() == "":
            if cur is not None:
                cur.blank_pending = True
            i += 1
            continue
        if RE_HR.match(line):
            close()
            i += 1
            continue
        if line.lstrip().startswith("|") and i + 1 < n and RE_TABLE_SEP.match(lines[i + 1]) \
                and "|" in lines[i + 1]:
            close()
            header = split_cells(line)
            rows = []
            j = i + 2
            while j < n and lines[j].lstrip().startswith("|"):
                rows.append((j + 1, lines[j]))
                j += 1
            labels = []
            for rl, rt in rows:
                m = RE_LABEL_CELL.match(split_cells(rt)[0])
                labels.append(m.group(1) if m else None)
            defining = (header[0] in DEFINING_HEADERS and rows
                        and all(lb is not None for lb in labels))
            prev = None
            for (rl, rt), lb in zip(rows, labels):
                if defining and prev is not None and prev.row_label == lb:
                    prev.add(rl, rt)
                    continue
                c = new_clause("row", rl)
                c.add(rl, rt)
                c.table_header = header
                c.row_label = lb
                c.table_defining = bool(defining)
                prev = c
            close()
            i = j
            continue
        lm = RE_LIST.match(line)
        indent = len(line) - len(line.lstrip())
        if lm:
            close()
            c = new_clause("item", lineno, indent=len(lm.group(1)), content_indent=lm.end())
            c.add(lineno, line)
            if lm.group(3):
                c.item_marker = lm.group(3)
            else:
                im = RE_ITEM_LEAD.match(line[lm.end():].strip())
                if im:
                    c.item_marker = im.group(1)
            i += 1
            continue
        if cur is not None and not cur.blank_pending:
            cur.add(lineno, line)  # continuation (lazy for list items)
            i += 1
            continue
        if cur is not None and cur.kind == "item" and cur.blank_pending \
                and indent >= max(2, cur.content_indent) and indent > cur.indent:
            cur.blank_pending = False
            cur.add(lineno, line)
            i += 1
            continue
        close()
        c = new_clause("para", lineno)
        c.add(lineno, line)
        im = RE_ITEM_LEAD.match(line.strip())
        if im:
            c.item_marker = im.group(1)
        i += 1
    return title, sections


# ---------------------------------------------------------------------------
# Graph construction


def section_node_id(spec, num):
    return "%s-%s" % (spec, num)


def build_graph(text, spec, version, filename="<input>", xspec=None):
    if not re.match(r"^[A-Z][A-Z0-9]*$", spec):
        raise SpecError("spec code must be uppercase alphanumeric, got %r" % spec)
    if xspec is not None:
        validate_graph(xspec, "xspec")
        if xspec["spec"] == spec:
            raise SpecError("--xspec graph is the same spec (%s) as the one being built" % spec)
    title, sections = parse_markdown(text, filename)
    root_id = spec
    nodes = [{"id": root_id, "kind": "root", "spec": spec, "version": version,
              "title": title[1] if title else None, "line": title[0] if title else 1}]
    clauses = []
    for num, sec in sections.items():
        if num == "front" and not sec.clauses:
            continue
        sid = section_node_id(spec, num)
        parent = section_node_id(spec, sec.parent) if sec.parent else root_id
        nodes.append({"id": sid, "kind": "section", "number": num, "title": sec.title,
                      "line": sec.line, "parent": parent, "depth": sec.depth})
        for k, c in enumerate(sec.clauses, 1):
            c.id = "%s-P%d" % (sid, k)
            clauses.append(c)
    by_id = {c.id: c for c in clauses}

    # --- aliases -------------------------------------------------------------
    aliases, alias_kind = define_aliases(spec, clauses, sections, filename)
    ambiguous = aliases.pop("__ambiguous__")
    for name, cid in aliases.items():
        by_id[cid].aliases.append(name)

    # --- markers -------------------------------------------------------------
    markers = {}
    for c in clauses:
        ms = []
        if c.item_marker:
            ms.append(c.item_marker)
        for m in RE_ITEM_INLINE.finditer(c.text):
            if m.group(1) not in ms:
                ms.append(m.group(1))
        markers[c.id] = ms

    for c in clauses:
        nd = {"id": c.id, "kind": "clause", "parent": section_node_id(spec, c.section.number),
              "clause_kind": c.kind, "line": c.start, "end_line": c.end,
              "hash": clause_hash(c.text), "aliases": sorted(c.aliases),
              "markers": markers[c.id], "text": c.text}
        if c.item_marker:
            nd["item_marker"] = c.item_marker
        if c.table_header is not None:
            nd["table_header"] = c.table_header
        nodes.append(nd)

    ctx = RefContext(spec, version, sections, markers,
                     {c.id: c.item_marker for c in clauses if c.item_marker}, xspec)
    edge_counts = collections.OrderedDict()
    unresolved = []

    def add_edge(src, dst, kind, line):
        key = (src, dst, kind)
        if key in edge_counts:
            edge_counts[key]["count"] += 1
        else:
            edge_counts[key] = {"src": src, "dst": dst, "kind": kind, "line": line, "count": 1}

    for c in clauses:
        for kind, dst, line, unres in scan_refs(c, ctx):
            if dst is not None:
                add_edge(c.id, dst, kind, line)
            if unres is not None:
                unres["src"] = c.id
                unresolved.append(unres)
    scan_uses(clauses, aliases, alias_kind, add_edge)

    edges_out = [{"src": n["parent"], "dst": n["id"], "kind": "contains"}
                 for n in nodes if n.get("parent")]
    edges_out.extend(edge_counts.values())

    stats = compute_stats(nodes, edges_out, unresolved, aliases, alias_kind, ambiguous)
    g = {"schema": SCHEMA, "spec": spec, "version": version,
         "source": {"file": os.path.basename(filename),
                    "sha256": hashlib.sha256(text.encode("utf-8")).hexdigest(),
                    "lines": text.count("\n") + (0 if text.endswith("\n") else 1)},
         "xspec": ({"spec": xspec["spec"], "version": xspec["version"],
                    "sha256": xspec["source"]["sha256"]} if xspec else None),
         "nodes": nodes, "edges": edges_out, "aliases": dict(sorted(aliases.items())),
         "alias_kinds": dict(sorted(alias_kind.items())),
         "ambiguous_terms": ambiguous, "unresolved": unresolved, "stats": stats}
    return g


def define_aliases(spec, clauses, sections, filename):
    """Return ({alias: clause_id, '__ambiguous__': [...]}, {alias: 'label'|'term'})."""
    primary = {}    # label -> clause (defining table rows)
    secondary = collections.OrderedDict()  # label -> [(clause, lead_text)]
    for c in clauses:
        if c.kind == "row" and c.table_defining and c.row_label and len(c.row_label) >= 2:
            lb = c.row_label
            if lb in primary:
                raise SpecError("duplicate label %s: defined at line %d and line %d"
                                % (lb, primary[lb].start, c.start), c.start, filename)
            primary[lb] = c
            continue
        if c.kind == "row":
            continue
        body = c.body
        m = RE_BOLD_LEAD.match(body)
        if m:
            lb = m.group(1)
            if len(lb) >= 2 and re.search(r"[\d-]", lb):
                secondary.setdefault(lb, []).append((c, norm_ws(m.group(0))))
                continue
        if c.kind == "item":
            m = RE_LIST_LEAD.match(body)
            if m and len(m.group(1)) >= 2 and re.search(r"[\d-]", m.group(1)):
                secondary.setdefault(m.group(1), []).append((c, norm_ws(m.group(0))))
    labels = {lb: c.id for lb, c in primary.items()}
    for lb, occ in secondary.items():
        seen = {}
        for c, lead in occ:
            if lead in seen:
                raise SpecError("duplicate label %s: identical definitions at line %d and line %d"
                                % (lb, seen[lead], c.start), c.start, filename)
            seen[lead] = c.start
        if lb not in labels:
            labels[lb] = occ[0][0].id

    # --- terms ---------------------------------------------------------------
    occurrences = collections.OrderedDict()  # term -> [(clause, in_terminology)]
    for c in clauses:
        prose = c.prose
        lead = c.body
        found = []
        for k, m in enumerate(RE_BOLD.finditer(prose)):
            raw = norm_ws(m.group(1))
            at_start = k == 0 and lead.startswith("**")
            term = term_of(raw, at_start, c.section.is_terminology)
            if term and term not in found:
                found.append(term)
        for t in found:
            occurrences.setdefault(t, []).append(c)
    terms = {}
    ambiguous = []
    for t, occ in occurrences.items():
        if t in labels:
            continue  # a label wins (e.g. bold label in a table cell)
        if len(occ) == 1:
            terms[t] = occ[0].id
            continue
        termin = [c for c in occ if c.section.is_terminology]
        if len(termin) == 1:
            terms[t] = termin[0].id
            continue
        ambiguous.append({"term": t, "clauses": [c.id for c in occ],
                          "lines": [c.start for c in occ]})
    out = dict(labels)
    out.update(terms)
    kinds = {k: "label" for k in labels}
    kinds.update({k: "term" for k in terms})
    out["__ambiguous__"] = ambiguous
    return out, kinds


def term_of(raw, at_start, in_terminology):
    """A bold span -> defined term, or None (see design doc for the rule)."""
    if at_start and raw.endswith(".") and not in_terminology:
        return None  # paragraph caption ("**Requirement.**")
    t = raw.strip().rstrip(".:,;").strip()
    if not t or t in TERM_STOP:
        return None
    if RE_LABEL_FULL.match(t.split()[0]) and re.search(r"[\d-]", t.split()[0]) \
            and len(t.split()[0]) >= 2:
        return None  # label lead, handled as a label
    if len(t.split()) > MAX_TERM_WORDS or len(t) > MAX_TERM_CHARS:
        return None
    if any(ch in t for ch in "§()=|\"[]{}<>*") or not re.match(r"^[A-Za-z`]", t):
        return None
    if not re.search(r"[A-Za-z]", t):
        return None
    if t.count("`") % 2:
        return None
    return t


# ---------------------------------------------------------------------------
# References


class RefContext:
    def __init__(self, spec, version, sections, markers, item_markers, xspec):
        self.spec = spec
        self.version = version
        self.local = SectionIndex(spec, [(num, s.depth) for num, s in sections.items()
                                         if num != "front"],
                                  {num: [c.id for c in s.clauses] for num, s in sections.items()},
                                  markers, item_markers)
        self.xspec_name = xspec["spec"] if xspec else None
        self.xspec = SectionIndex.from_graph(xspec) if xspec else None


class SectionIndex:
    """Section numbers in document order, their clauses, and clause markers."""

    def __init__(self, spec, ordered, clauses_by_section, markers, item_markers=None):
        self.spec = spec
        self.ordered = ordered  # [(num, depth)]
        self.depth = dict(ordered)
        self.pos = {num: k for k, (num, _) in enumerate(ordered)}
        self.clauses = clauses_by_section
        self.markers = markers
        self.item_markers = item_markers or {}

    @classmethod
    def from_graph(cls, g):
        ordered, cl, markers, items = [], collections.defaultdict(list), {}, {}
        for nd in g["nodes"]:
            if nd["kind"] == "section" and nd["number"] != "front":
                ordered.append((nd["number"], nd["depth"]))
            elif nd["kind"] == "clause":
                num = nd["parent"][len(g["spec"]) + 1:]
                cl[num].append(nd["id"])
                markers[nd["id"]] = nd.get("markers", [])
                if nd.get("item_marker"):
                    items[nd["id"]] = nd["item_marker"]
        return cls(g["spec"], ordered, dict(cl), markers, items)

    def has(self, num):
        return num in self.depth

    def sid(self, num):
        return section_node_id(self.spec, num)

    def subtree_clauses(self, num):
        out = list(self.clauses.get(num, []))
        for other, _ in self.ordered:
            if other.startswith(num + "."):
                out.extend(self.clauses.get(other, []))
        return out

    def find_item(self, num, marker, primary_of):
        cands = self.subtree_clauses(num)
        for cid in cands:
            if primary_of(cid) == marker:
                return cid
        for cid in cands:
            if marker in self.markers.get(cid, []):
                return cid
        return None

    def range(self, a, b):
        if not (self.has(a) and self.has(b)) or self.pos[b] < self.pos[a]:
            return None
        d = self.depth[a]
        return [num for num, dep in self.ordered[self.pos[a]:self.pos[b] + 1] if dep == d]


def line_at(c, offset, text):
    return c.start + text.count("\n", 0, offset)


def classify_prefix(text, pos, spec, version):
    """Return ('local'|'xspec'|'external', name) from the words before a § at pos."""
    before = text[max(0, pos - 80):pos]
    m = RE_PREV_WORD.search(before)
    if not m:
        return "local", None
    w = m.group(1).strip("`")
    if RE_VERSION.match(w):
        m2 = RE_PREV_WORD.search(before[:m.start()])
        w2 = m2.group(1).strip("`") if m2 else None
        if w2 in SPEC_PREFIX:
            target = SPEC_PREFIX[w2]
            if target == spec:
                return ("local", None) if w == version.split("-")[0] else \
                    ("external", "%s-%s" % (spec, w))
            return "xspec", target
        if w != version.split("-")[0]:
            return "external", "%s-%s" % (spec, w)
        return "local", None
    if w in SPEC_PREFIX:
        target = SPEC_PREFIX[w]
        return ("local", None) if target == spec else ("xspec", target)
    if w in EXTERNAL_PREFIX:
        return "external", EXTERNAL_PREFIX[w]
    return "local", None


def scan_refs(c, ctx):
    """Yield (kind, dst, line, unresolved_or_None) for every § reference in clause c."""
    text = c.text
    prev_end, prev_scope = None, None
    for m in RE_REF.finditer(text):
        line = line_at(c, m.start(), text)
        if prev_end is not None and RE_SEP_CARRY.match(text[prev_end:m.start()]):
            scope = prev_scope
        else:
            scope = classify_prefix(text, m.start(), ctx.spec, ctx.version)
        prev_end, prev_scope = m.end(), scope
        raw = norm_ws(m.group(0))
        kind_s, name = scope
        if kind_s == "external":
            yield "refs", None, line, {"line": line, "ref": "%s %s" % (name, raw)
                                       if name else raw, "reason": "external:%s" % name}
            continue
        if kind_s == "xspec":
            label = "%s %s" % (name, raw)
            if ctx.xspec is None or ctx.xspec_name != name:
                yield "xspec", None, line, {"line": line, "ref": label,
                                            "reason": "xspec-not-loaded"}
                continue
            idx, ekind = ctx.xspec, "xspec"
        else:
            label, idx, ekind = raw, ctx.local, "refs"
        if m.group("dbl"):
            nums = re.findall(NUM, m.group("list"))
            targets = [(nm, None) for nm in nums]
        else:
            one = m.group("one").rstrip(".")
            item = m.group("pitem") or m.group("litem") or m.group("witem")
            targets = [(one, item)]
            if m.group("witem2"):
                targets.append((one, m.group("witem2")))
            if m.group("rng"):
                rng = idx.range(one, m.group("rng").rstrip("."))
                if rng is None:
                    yield ekind, None, line, {"line": line, "ref": label,
                                              "reason": "bad-range"}
                else:
                    targets = [(nm, None) for nm in rng]
        for num, item in targets:
            num = num.rstrip(".")
            if not idx.has(num):
                yield ekind, None, line, {"line": line, "ref": label, "reason": "no-such-section"}
                continue
            if item is None:
                yield ekind, idx.sid(num), line, None
                continue
            hit = idx.find_item(num, item, idx.item_markers.get)
            if hit is None:
                yield ekind, idx.sid(num), line, {"line": line, "ref": label,
                                                  "reason": "item-not-found",
                                                  "fallback": idx.sid(num)}
            else:
                yield ekind, hit, line, None


# ---------------------------------------------------------------------------
# Uses (alias mentions)

LABEL_LEFT = r"(?<![A-Za-z0-9_\-.])"
LABEL_RIGHT = r"(?![A-Za-z0-9_]|-[A-Za-z0-9]|\.[0-9])"
TERM_LEFT = r"(?<![\w\-])"
TERM_RIGHT = r"(?:e?s)?(?![\w\-])"


def alias_regex(names, left, right):
    if not names:
        return None
    alts = "|".join(re.escape(n) for n in sorted(names, key=lambda s: (-len(s), s)))
    return re.compile("%s(%s)%s" % (left, alts, right))


RE_LABEL_SPLIT = re.compile(r"^(.*?)(\d+)$")


def scan_uses(clauses, aliases, alias_kind, add_edge):
    labels = [a for a, k in alias_kind.items() if k == "label"]
    terms = [a for a, k in alias_kind.items() if k == "term"]
    rl = alias_regex(labels, LABEL_LEFT, LABEL_RIGHT)
    rt = alias_regex(terms, TERM_LEFT, TERM_RIGHT)
    by_prefix = collections.defaultdict(dict)
    for lb in labels:
        m = RE_LABEL_SPLIT.match(lb)
        if m:
            by_prefix[m.group(1)][int(m.group(2))] = lb
    for c in clauses:
        text = c.text
        if rl:
            for m in rl.finditer(text):
                name = m.group(1)
                hits = [name]
                # range PT3–PT10 (en dash)
                rest = text[m.end():m.end() + 24]
                rm = re.match(r"–(%s)" % LABEL, rest)
                if rm:
                    a, b = RE_LABEL_SPLIT.match(name), RE_LABEL_SPLIT.match(rm.group(1))
                    if a and b and a.group(1) == b.group(1):
                        for k in range(int(a.group(2)) + 1, int(b.group(2))):
                            if k in by_prefix[a.group(1)]:
                                hits.append(by_prefix[a.group(1)][k])
                for h in hits:
                    dst = aliases[h]
                    if dst != c.id:
                        add_edge(c.id, dst, "uses", line_at(c, m.start(), text))
        if rt:
            prose = c.prose
            for m in rt.finditer(prose):
                dst = aliases[m.group(1)]
                if dst != c.id:
                    add_edge(c.id, dst, "uses", line_at(c, m.start(), prose))


def compute_stats(nodes, edges, unresolved, aliases, alias_kind, ambiguous):
    kinds = collections.Counter(n["kind"] for n in nodes)
    ek = collections.Counter(e["kind"] for e in edges)
    ur = collections.Counter(u["reason"] for u in unresolved)
    return {"nodes": len(nodes), "sections": kinds["section"], "clauses": kinds["clause"],
            "edges": len(edges), "edges_by_kind": dict(sorted(ek.items())),
            "unresolved": len(unresolved), "unresolved_by_reason": dict(sorted(ur.items())),
            "aliases": len(aliases),
            "labels": sum(1 for k in alias_kind.values() if k == "label"),
            "terms": sum(1 for k in alias_kind.values() if k == "term"),
            "ambiguous_terms": len(ambiguous)}


def validate_graph(g, what="graph"):
    if not isinstance(g, dict) or g.get("schema") != SCHEMA:
        raise UsageError("%s is not a %s file" % (what, SCHEMA))
    for k in ("spec", "version", "nodes", "edges", "aliases", "unresolved"):
        if k not in g:
            raise UsageError("%s lacks key %r" % (what, k))
