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
_CELL1 = r"(?:\*\*)?`?%s`?(?:\*\*)?" % LABEL
RE_LABEL_CELL = re.compile(r"^(%s(?:,\s*%s)*)(?:\s*=\s*.*)?$" % (_CELL1, _CELL1))
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
    r"(?:(?P<wsep>\s*(?:,|and|–)\s*)(?P<witem2>\d{1,2}[a-z]?))?)"
    r")?" % {"n": NUM, "i": ITEM})
RE_SEP_CARRY = re.compile(r"^\s*(?:,|/|–|and|or|,\s*and)\s*$")
RE_PREV_WORD = re.compile(r"(`[^`\s]+`|[A-Za-z][\w.\-]*?(?:['’]s)?|§*\d+(?:\.\d+)+)[\s(]*$")
SPEC_PREFIX = {"ISA": "ISA", "bennett-isa-spec": "ISA",
               "dialect": "DLT", "bennett-dialect-spec": "DLT"}
EXTERNAL_PREFIX = {"NS": "NS", "star": "north-star", "Bennett-NorthStar.md": "north-star",
                   "NorthStar": "north-star", "SQLite": "SQLite", "findings": "spike-findings",
                   "outline": "outline", "DL": "dialect-outline"}
RE_VERSION = re.compile(r"^\d+\.\d+(?:\.\d+)?$")

EDGE_DEP_KINDS = ("refs", "uses", "xspec")
CLOSURES = ("anchored", "full")


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
                labels.append(tuple(re.findall(LABEL, m.group(1))) if m else None)
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
        if c.kind == "row" and c.table_defining and c.row_label:
            for lb in c.row_label:
                if len(lb) < 2:
                    continue  # single letters (ISA suites A-H) collide with prose
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
    if len(t) < 2 or t.split()[0].lower() in TERM_STOP:
        return None
    if not in_terminology and " " not in t and re.match(r"^[A-Za-z]+$", t) \
            and not re.match(r"^[A-Z][A-Z0-9]+s?$", t):
        return None  # single plain word outside Terminology: emphasis or caption
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
    w = re.sub(r"['’]s$", "", m.group(1).strip("`"))
    if w.startswith("§"):
        return "local", None  # "§3.3 (§3.1": the previous token is a reference itself
    if RE_VERSION.match(w):
        head = before[:m.start()]
        m2 = RE_PREV_WORD.search(head)
        w2 = m2.group(1).strip("`") if m2 else None
        if w2 == "draft" and m2:
            m3 = RE_PREV_WORD.search(head[:m2.start()])
            w2 = m3.group(1).strip("`") if m3 else None
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
                a, b = m.group("witem"), m.group("witem2")
                if "–" in m.group("wsep") and a.isdigit() and b.isdigit() and int(a) < int(b):
                    targets.extend((one, str(k)) for k in range(int(a) + 1, int(b) + 1))
                else:
                    targets.append((one, b))
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


# ---------------------------------------------------------------------------
# Loaded graphs and queries


class Merged:
    """One or more graphs (e.g. DLT + the ISA graph it was built against) as one digraph."""

    def __init__(self, graphs):
        self.graphs = graphs
        self.nodes = {}
        self.children = collections.defaultdict(list)
        self.out = collections.defaultdict(set)
        self.inn = collections.defaultdict(set)
        self.aliases = {}
        by_spec = {}
        for g in graphs:
            validate_graph(g)
            if g["spec"] in by_spec:
                raise UsageError("two graphs for spec %s (%s and %s); load one version per spec"
                                 % (g["spec"], by_spec[g["spec"]]["version"], g["version"]))
            by_spec[g["spec"]] = g
        for g in graphs:
            xs = g.get("xspec")
            if xs and xs["spec"] in by_spec:
                other = by_spec[xs["spec"]]
                if other["source"]["sha256"] != xs["sha256"]:
                    raise UsageError(
                        "%s-%s was built against %s-%s with source sha256 %s..., but the loaded "
                        "%s graph is %s-%s with sha256 %s...; rebuild with the matching --xspec"
                        % (g["spec"], g["version"], xs["spec"], xs["version"], xs["sha256"][:12],
                           xs["spec"], other["spec"], other["version"],
                           other["source"]["sha256"][:12]))
            for n in g["nodes"]:
                self.nodes[n["id"]] = n
                if n.get("parent"):
                    self.children[n["parent"]].append(n["id"])
            for e in g["edges"]:
                if e["kind"] in EDGE_DEP_KINDS:
                    self.out[e["src"]].add(e["dst"])
                    self.inn[e["dst"]].add(e["src"])
            self.aliases[g["spec"]] = g["aliases"]
        self.by_spec = by_spec

    def kind(self, nid):
        n = self.nodes.get(nid)
        return n["kind"] if n else "external"

    def resolve(self, token):
        if token in self.nodes:
            return token
        if ":" in token:
            spec, alias = token.split(":", 1)
            if spec in self.aliases and alias in self.aliases[spec]:
                return self.aliases[spec][alias]
            raise UsageError("unknown alias %r in spec %s" % (alias, spec))
        hits = [a[token] for a in self.aliases.values() if token in a]
        if len(hits) == 1:
            return hits[0]
        if len(hits) > 1:
            raise UsageError("alias %r is defined in several loaded specs (%s); write SPEC:%s"
                             % (token, ", ".join(sorted(hits)), token))
        spec = token.split("-", 1)[0]
        hint = "" if spec in self.by_spec else \
            " (spec %s is not loaded; add its graph with --with)" % spec
        raise UsageError("unknown clause, section or alias %r%s" % (token, hint))

    def descendants(self, nid):
        out, stack = [], [nid]
        while stack:
            x = stack.pop()
            out.append(x)
            stack.extend(self.children.get(x, ()))
        return out

    def ancestors(self, nid):
        out = []
        n = self.nodes.get(nid)
        while n is not None and n.get("parent"):
            p = self.nodes.get(n["parent"])
            if p is None or p["kind"] == "root":
                break
            out.append(p["id"])
            n = p
        return out

    # Closure modes (docs/design/clause-graph.md, "Dependency semantics"):
    #   anchored: a reference to a section cites every clause in it, but section references
    #             are followed only at the query end; name-level edges (uses, item refs,
    #             xspec to clauses) chain transitively.
    #   full:     every reached clause is lifted to its enclosing sections at every step
    #             (the transitive closure of "cites X or a section containing X").
    @staticmethod
    def _check(closure, max_depth):
        if closure not in CLOSURES:
            raise UsageError("closure must be one of %s, got %r" % ("/".join(CLOSURES), closure))
        if max_depth is not None and max_depth < 1:
            raise UsageError("max depth must be >= 1, got %r" % max_depth)

    def _reverse(self, nid, closure="anchored", max_depth=None):
        self._check(closure, max_depth)
        start = set(self.descendants(nid)) | set(self.ancestors(nid))
        visited = set(start)
        levels, frontier = [], start
        while frontier and (max_depth is None or len(levels) < max_depth):
            nxt = set()
            for x in frontier:
                for m in self.inn.get(x, ()):
                    if m in visited:
                        continue
                    visited.add(m)
                    nxt.add(m)
                    if closure == "full":
                        for a in self.ancestors(m):
                            if a not in visited:
                                visited.add(a)
                                nxt.add(a)
            if not nxt:
                break
            levels.append(nxt)
            frontier = nxt
        return start, levels

    def impact(self, nid, closure="anchored", max_depth=None):
        """Transitive dependents (clauses), grouped by depth (index 0 = depth 1)."""
        _, levels = self._reverse(nid, closure, max_depth)
        out = [sorted([x for x in lv if self.kind(x) == "clause"], key=natural_key)
               for lv in levels]
        return [lv for lv in out if lv]

    def reverse_cone(self, nid, closure="anchored", max_depth=None):
        start, levels = self._reverse(nid, closure, max_depth)
        allx = set(start).union(*levels) if levels else set(start)
        return {x for x in allx if self.kind(x) not in ("section", "root")}

    def forward_cone(self, nid, closure="anchored", max_depth=None):
        self._check(closure, max_depth)
        visited = set(self.descendants(nid))
        frontier, depth = list(visited), 0
        while frontier and (max_depth is None or depth < max_depth):
            nxt = []
            for x in frontier:
                for m in self.out.get(x, ()):
                    if self.kind(m) == "section" and closure == "anchored" and depth > 0:
                        continue
                    for k in self.descendants(m):
                        if k not in visited:
                            visited.add(k)
                            nxt.append(k)
            frontier, depth = nxt, depth + 1
        return {x for x in visited if self.kind(x) not in ("section", "root")}

    def independent(self, a, b, closure="anchored", max_depth=None):
        fa = self.forward_cone(a, closure, max_depth)
        fb = self.forward_cone(b, closure, max_depth)
        ra = self.reverse_cone(a, closure, max_depth)
        rb = self.reverse_cone(b, closure, max_depth)
        fwd = sorted(fa & fb, key=natural_key)
        rev = sorted(ra & rb, key=natural_key)
        return (not fwd and not rev), fwd, rev

    def top(self, spec, n, closure="anchored", max_depth=None):
        """[(clause, transitive dependents, direct dependents)], most dependents first."""
        rows = []
        for nid, nd in self.nodes.items():
            if nd["kind"] == "clause" and nid.startswith(spec + "-"):
                lv = self.impact(nid, closure, max_depth)
                rows.append((nid, sum(len(x) for x in lv), len(lv[0]) if lv else 0))
        rows.sort(key=lambda r: (-r[1], -r[2], natural_key(r[0])))
        return rows[:n]


def natural_key(s):
    return [int(t) if t.isdigit() else t for t in re.split(r"(\d+)", s)]


RE_FOREIGN_ID = re.compile(r"^([A-Z][A-Z0-9]*)(?::.+|-(?:front|\d+(?:\.\d+)?|[A-Z])(?:-P\d+)?)$")


def coverage(g, tags):
    validate_graph(g)
    m = Merged([g])
    if not isinstance(tags, dict):
        raise UsageError("tags must be a JSON object mapping test/proof names to ids")
    strong = collections.defaultdict(list)
    weak = collections.defaultdict(list)
    foreign = []
    for name, targets in tags.items():
        if isinstance(targets, str):
            targets = [targets]
        if not isinstance(targets, list) or not all(isinstance(t, str) for t in targets):
            raise UsageError("tag %r: value must be an id or a list of ids" % name)
        for t in targets:
            try:
                nid = m.resolve(t)
            except UsageError as e:
                fm = RE_FOREIGN_ID.match(t)
                if fm and fm.group(1) != g["spec"]:
                    foreign.append(t)  # an id or SPEC:alias of another spec: listed, not used
                    continue
                raise UsageError("tag %r: %s" % (name, e))
            kind = m.kind(nid)
            if kind == "clause":
                strong[nid].append(name)
            else:
                for d in m.descendants(nid):
                    if m.kind(d) == "clause":
                        weak[d].append(name)
    sections = []
    totals = {"clauses": 0, "strong": 0, "weak": 0, "uncovered": 0}
    for nd in g["nodes"]:
        if nd["kind"] != "section":
            continue
        cl = [c for c in m.children.get(nd["id"], ()) if m.kind(c) == "clause"]
        if not cl:
            continue
        cl.sort(key=natural_key)
        st = [c for c in cl if c in strong]
        wk = [c for c in cl if c not in strong and c in weak]
        un = [c for c in cl if c not in strong and c not in weak]
        sections.append({"section": nd["id"], "title": nd["title"], "clauses": len(cl),
                         "strong": st, "weak": wk, "uncovered": un})
        totals["clauses"] += len(cl)
        totals["strong"] += len(st)
        totals["weak"] += len(wk)
        totals["uncovered"] += len(un)
    return {"spec": g["spec"], "version": g["version"], "totals": totals,
            "sections": sections, "foreign": sorted(set(foreign)),
            "witnesses": {k: sorted(v) for k, v in sorted(strong.items())}}


# ---------------------------------------------------------------------------
# Diff between two versions of one spec

SIMILAR_RATIO = 0.6


def diff_graphs(a, b):
    validate_graph(a, "first graph")
    validate_graph(b, "second graph")
    if a["spec"] != b["spec"]:
        raise UsageError("diff needs two graphs of the same spec, got %s and %s"
                         % (a["spec"], b["spec"]))
    ca = [n for n in a["nodes"] if n["kind"] == "clause"]
    cb = [n for n in b["nodes"] if n["kind"] == "clause"]
    sec = lambda n: n["parent"][len(a["spec"]) + 1:]  # noqa: E731
    pairs = {}  # a_id -> (b_id, how)
    used_b = set()
    # 1. identical hash, paired in document order
    hb = collections.defaultdict(list)
    for n in cb:
        hb[n["hash"]].append(n["id"])
    for n in ca:
        lst = hb.get(n["hash"])
        if lst:
            bid = lst.pop(0)
            pairs[n["id"]] = (bid, "hash")
            used_b.add(bid)
    # 2. (section, alias), then alias alone (labels are unique per spec)
    for same_section in (True, False):
        idx = {}
        for n in cb:
            if n["id"] in used_b:
                continue
            for al in n.get("aliases", []):
                idx.setdefault((sec(n) if same_section else None, al), n["id"])
        for n in ca:
            if n["id"] in pairs:
                continue
            for al in n.get("aliases", []):
                bid = idx.get((sec(n) if same_section else None, al))
                if bid and bid not in used_b:
                    pairs[n["id"]] = (bid, "alias")
                    used_b.add(bid)
                    break
    # 3. similar text within the same section number
    ra = collections.defaultdict(list)
    rb = collections.defaultdict(list)
    for n in ca:
        if n["id"] not in pairs:
            ra[sec(n)].append(n)
    for n in cb:
        if n["id"] not in used_b:
            rb[sec(n)].append(n)
    for s_, lst in ra.items():
        cands = []
        for x in lst:
            tx = norm_ws(x["text"])
            for y in rb.get(s_, ()):
                ty = norm_ws(y["text"])
                sm = difflib.SequenceMatcher(None, tx, ty, autojunk=False)
                if sm.real_quick_ratio() < SIMILAR_RATIO or sm.quick_ratio() < SIMILAR_RATIO:
                    continue
                r = sm.ratio()
                if r >= SIMILAR_RATIO:
                    cands.append((-r, natural_key(x["id"]), natural_key(y["id"]),
                                  x["id"], y["id"]))
        cands.sort()
        for _, _, _, xa, yb in cands:
            if xa in pairs or yb in used_b:
                continue
            pairs[xa] = (yb, "similar")
            used_b.add(yb)
    hash_a = {n["id"]: n["hash"] for n in ca}
    hash_b = {n["id"]: n["hash"] for n in cb}
    moved, changed = [], []
    unchanged = 0
    for aid in sorted(pairs, key=natural_key):
        bid, how = pairs[aid]
        same_id, same_hash = aid == bid, hash_a[aid] == hash_b[bid]
        if same_id and same_hash:
            unchanged += 1
        if not same_id:
            moved.append([aid, bid, how])
        if not same_hash:
            changed.append([aid, bid, how])
    added = sorted((n["id"] for n in cb if n["id"] not in used_b), key=natural_key)
    removed = sorted((n["id"] for n in ca if n["id"] not in pairs), key=natural_key)
    per_section = collections.OrderedDict()

    def bump(secid, key):
        top = secid.split(".")[0]
        per_section.setdefault(top, collections.Counter())[key] += 1

    for aid, (bid, how) in pairs.items():
        same_id, same_hash = aid == bid, hash_a[aid] == hash_b[bid]
        k = "unchanged" if same_id and same_hash else (
            "moved" if same_hash else ("changed" if same_id else "moved+changed"))
        bump(sec(next(n for n in cb if n["id"] == bid)), k)
    for x in added:
        bump(x.rsplit("-P", 1)[0][len(a["spec"]) + 1:], "added")
    for x in removed:
        bump(x.rsplit("-P", 1)[0][len(a["spec"]) + 1:], "removed")
    how = collections.Counter(h for _, h in pairs.values())
    summary = {"a": "%s-%s" % (a["spec"], a["version"]), "b": "%s-%s" % (b["spec"], b["version"]),
               "clauses_a": len(ca), "clauses_b": len(cb), "matched": len(pairs),
               "matched_by": dict(sorted(how.items())), "unchanged": unchanged,
               "moved": len(moved), "changed": len(changed),
               "moved_only": sum(1 for x in moved if hash_a[x[0]] == hash_b[x[1]]),
               "changed_in_place": sum(1 for x in changed if x[0] == x[1]),
               "added": len(added), "removed": len(removed)}
    secs = [{"section": k, **{c: v.get(c, 0) for c in
                              ("unchanged", "moved", "changed", "moved+changed", "added",
                               "removed")}}
            for k, v in sorted(per_section.items(), key=lambda kv: natural_key(kv[0]))]
    return {"summary": summary, "sections": secs, "added": added, "removed": removed,
            "moved": moved, "changed": changed}


# ---------------------------------------------------------------------------
# DOT output


def dot_q(s):
    return '"%s"' % s.replace("\\", "\\\\").replace('"', '\\"')


def to_dot(g):
    m = Merged([g])
    spec = g["spec"]
    out = ["digraph %s {" % dot_q("%s-%s" % (spec, g["version"])),
           "  graph [rankdir=LR, compound=true, fontsize=10, label=%s];"
           % dot_q("%s %s clause graph" % (spec, g["version"])),
           "  node [shape=box, fontsize=8, height=0.2];",
           "  edge [arrowsize=0.5];"]

    def emit(nid, indent):
        nd = m.nodes[nid]
        pad = "  " * indent
        if nd["kind"] == "section":
            out.append("%ssubgraph %s {" % (pad, dot_q("cluster_" + nid)))
            out.append("%s  label=%s;" % (pad, dot_q("§%s %s" % (nd["number"], nd["title"]))))
            out.append("%s  %s [shape=folder, label=%s];"
                       % (pad, dot_q(nid), dot_q("§" + nd["number"])))
            for c in m.children.get(nid, ()):
                emit(c, indent + 1)
            out.append("%s}" % pad)
        elif nd["kind"] == "clause":
            lab = nid[len(nd["parent"]) + 1:]
            if nd.get("aliases"):
                lab += "\\n" + ", ".join(nd["aliases"][:3]).replace('"', "'")
            out.append("%s%s [label=\"%s\"];" % (pad, dot_q(nid), lab.replace("\\n", "\\n")))

    for c in m.children.get(spec, ()):
        emit(c, 1)
    externals = sorted({e["dst"] for e in g["edges"] if e["kind"] == "xspec"}, key=natural_key)
    for x in externals:
        out.append("  %s [shape=ellipse, style=filled, fillcolor=lightblue];" % dot_q(x))
    style = {"refs": "", "uses": " [style=dashed, color=gray40]",
             "xspec": " [color=blue, penwidth=1.5]"}
    for e in g["edges"]:
        if e["kind"] in style:
            out.append("  %s -> %s%s;" % (dot_q(e["src"]), dot_q(e["dst"]), style[e["kind"]]))
    out.append("}")
    return "\n".join(out) + "\n"


# ---------------------------------------------------------------------------
# CLI


def load_json(path):
    try:
        with open(path, encoding="utf-8") as f:
            return json.load(f)
    except OSError as e:
        raise UsageError("cannot read %s: %s" % (path, e.strerror))
    except json.JSONDecodeError as e:
        raise UsageError("%s:%d: invalid JSON: %s" % (path, e.lineno, e.msg))


def load_graph(path):
    g = load_json(path)
    try:
        validate_graph(g, path)
    except UsageError as e:
        raise UsageError(str(e))
    return g


def write_text(path, text):
    tmp = path + ".part"
    with open(tmp, "w", encoding="utf-8") as f:
        f.write(text)
    os.replace(tmp, path)


def cmd_build(args):
    try:
        with open(args.file, encoding="utf-8") as f:
            text = f.read()
    except OSError as e:
        raise UsageError("cannot read %s: %s" % (args.file, e.strerror))
    xs = load_graph(args.xspec) if args.xspec else None
    g = build_graph(text, args.spec, args.version, args.file, xspec=xs)
    os.makedirs(args.out_dir, exist_ok=True)
    stem = os.path.join(args.out_dir, "%s-%s" % (args.spec, args.version))
    write_text(stem + ".json", json.dumps(g, indent=1, ensure_ascii=False) + "\n")
    write_text(stem + ".dot", to_dot(g))
    st = g["stats"]
    print("%s-%s: %d nodes (%d sections, %d clauses), %d edges %s, %d unresolved %s, "
          "%d aliases (%d labels, %d terms), %d ambiguous terms"
          % (args.spec, args.version, st["nodes"], st["sections"], st["clauses"], st["edges"],
             json.dumps(st["edges_by_kind"]), st["unresolved"],
             json.dumps(st["unresolved_by_reason"]), st["aliases"], st["labels"], st["terms"],
             st["ambiguous_terms"]))
    for t in g["ambiguous_terms"]:
        print("warning: bold term %r is bolded in %s (lines %s); not an alias"
              % (t["term"], ", ".join(t["clauses"]), ", ".join(map(str, t["lines"]))),
              file=sys.stderr)
    print("wrote %s.json and %s.dot" % (stem, stem))
    if args.svg:
        import shutil
        import subprocess
        if shutil.which("dot") is None:
            print("dot not found on PATH; skipped SVG rendering")
        else:
            try:
                subprocess.run(["dot", "-Tsvg", "-o", stem + ".svg", stem + ".dot"], check=True,
                               timeout=args.svg_timeout)
                print("wrote %s.svg" % stem)
            except subprocess.TimeoutExpired:
                print("dot -Tsvg exceeded %ds; skipped SVG" % args.svg_timeout)
    return 0


def merged_from(args):
    return Merged([load_graph(args.graph)] + [load_graph(p) for p in (args.with_ or [])])


def describe(m, nid):
    nd = m.nodes.get(nid, {})
    extra = ""
    if nd.get("aliases"):
        extra = " [%s]" % ", ".join(nd["aliases"])
    if nd.get("kind") == "section":
        extra = " (§%s %s)" % (nd["number"], nd["title"])
    return nid + extra


def cmd_impact(args):
    m = merged_from(args)
    nid = m.resolve(args.id)
    levels = m.impact(nid, args.closure, args.max_depth)
    total = sum(len(x) for x in levels)
    if args.json:
        print(json.dumps({"id": nid, "closure": args.closure, "max_depth": args.max_depth,
                          "total": total, "levels": levels}, indent=1))
        return 0
    print("impact of %s (closure %s%s): %d dependent clauses"
          % (describe(m, nid), args.closure,
             ", max depth %d" % args.max_depth if args.max_depth else "", total))
    for d, lv in enumerate(levels, 1):
        print("depth %d (%d): %s" % (d, len(lv), ", ".join(describe(m, x) for x in lv)))
    return 0


def cmd_independent(args):
    m = merged_from(args)
    a, b = m.resolve(args.a), m.resolve(args.b)
    ok, fwd, rev = m.independent(a, b, args.closure, args.max_depth)
    if ok:
        print("independent")
        return 0
    print("dependent")
    print("shared forward cone (%d): %s" % (len(fwd), ", ".join(fwd)))
    print("shared reverse cone (%d): %s" % (len(rev), ", ".join(rev)))
    return 1


def cmd_coverage(args):
    g = load_graph(args.graph)
    rep = coverage(g, load_json(args.tags))
    if args.json:
        print(json.dumps(rep, indent=1))
        return 0
    t = rep["totals"]
    print("coverage of %s-%s: %d clauses; strong %d, weak (section tag only) %d, uncovered %d"
          % (rep["spec"], rep["version"], t["clauses"], t["strong"], t["weak"], t["uncovered"]))
    if rep["foreign"]:
        print("ignored tags for other specs: %s" % ", ".join(rep["foreign"]))
    for s_ in rep["sections"]:
        line = "%s %s: %d clauses, strong %d, weak %d, uncovered %d" % (
            s_["section"], s_["title"], s_["clauses"], len(s_["strong"]), len(s_["weak"]),
            len(s_["uncovered"]))
        if s_["weak"]:
            line += "; weak: " + ", ".join(s_["weak"])
        if s_["uncovered"]:
            line += "; uncovered: " + ", ".join(s_["uncovered"])
        print(line)
    return 0


def diff_table(d):
    rows = ["| § | unchanged | moved | changed | moved+changed | added | removed |",
            "|---|---|---|---|---|---|---|"]
    tot = collections.Counter()
    for s_ in d["sections"]:
        rows.append("| %s | %d | %d | %d | %d | %d | %d |" % (
            s_["section"], s_["unchanged"], s_["moved"], s_["changed"], s_["moved+changed"],
            s_["added"], s_["removed"]))
        for k in ("unchanged", "moved", "changed", "moved+changed", "added", "removed"):
            tot[k] += s_[k]
    rows.append("| total | %d | %d | %d | %d | %d | %d |" % (
        tot["unchanged"], tot["moved"], tot["changed"], tot["moved+changed"], tot["added"],
        tot["removed"]))
    return "\n".join(rows)


def cmd_diff(args):
    d = diff_graphs(load_graph(args.a), load_graph(args.b))
    if args.json:
        print(json.dumps(d, indent=1))
        return 0
    s_ = d["summary"]
    print("diff %s -> %s: %d -> %d clauses; matched %d %s; unchanged %d; moved %d "
          "(renumbered only %d); changed %d (in place %d); added %d; removed %d"
          % (s_["a"], s_["b"], s_["clauses_a"], s_["clauses_b"], s_["matched"],
             json.dumps(s_["matched_by"]), s_["unchanged"], s_["moved"], s_["moved_only"],
             s_["changed"], s_["changed_in_place"], s_["added"], s_["removed"]))
    print()
    print(diff_table(d))
    if args.list:
        for k in ("added", "removed"):
            print("%s: %s" % (k, ", ".join(d[k])))
        for k in ("moved", "changed"):
            print("%s: %s" % (k, ", ".join("%s->%s(%s)" % tuple(x) for x in d[k])))
    return 0


def cmd_top(args):
    m = merged_from(args)
    spec = args.spec or m.graphs[0]["spec"]
    print("| clause | transitive dependents | direct dependents | aliases |")
    print("|---|---|---|---|")
    for nid, cnt, direct in m.top(spec, args.n, args.closure, args.max_depth):
        print("| %s | %d | %d | %s |" % (nid, cnt, direct,
                                        ", ".join(m.nodes[nid].get("aliases", [])[:4])))
    return 0


def add_closure_args(p):
    p.add_argument("--closure", choices=CLOSURES, default="anchored",
                   help="anchored (default): section refs count at the query end only; "
                        "full: lift every reached clause to its sections (conservative)")
    p.add_argument("--max-depth", type=int, default=None, help="stop after this many hops")


def make_parser():
    p = argparse.ArgumentParser(prog="clausegraph", description=__doc__.splitlines()[0])
    sub = p.add_subparsers(dest="cmd", required=True)
    b = sub.add_parser("build", help="parse a spec into <out-dir>/<SPEC>-<version>.json/.dot")
    b.add_argument("--spec", required=True, help="spec code, e.g. ISA or DLT")
    b.add_argument("--version", required=True, help="e.g. 0.8, 0.9-draft, 0.2.10")
    b.add_argument("--file", required=True, help="spec markdown file")
    b.add_argument("--xspec", help="graph JSON of the spec this one cites (resolves 'ISA §x')")
    b.add_argument("--out-dir", default=os.path.dirname(os.path.abspath(__file__)))
    b.add_argument("--svg", action="store_true", help="also run dot -Tsvg if dot exists")
    b.add_argument("--svg-timeout", type=int, default=120)
    b.set_defaults(fn=cmd_build)
    d = sub.add_parser("diff", help="match clauses between two versions of one spec")
    d.add_argument("a")
    d.add_argument("b")
    d.add_argument("--json", action="store_true")
    d.add_argument("--list", action="store_true", help="list every added/removed/moved/changed id")
    d.set_defaults(fn=cmd_diff)
    for name, fn, hlp in (("impact", cmd_impact, "transitive dependents by depth"),
                          ("independent", cmd_independent, "are two clauses' cones disjoint")):
        q = sub.add_parser(name, help=hlp)
        q.add_argument("graph")
        if name == "impact":
            q.add_argument("id")
            q.add_argument("--json", action="store_true")
        else:
            q.add_argument("a")
            q.add_argument("b")
        q.add_argument("--with", dest="with_", action="append",
                       help="also load this graph (e.g. the ISA graph for a DLT query)")
        add_closure_args(q)
        q.set_defaults(fn=fn)
    c = sub.add_parser("coverage", help="clauses with zero witnesses per section")
    c.add_argument("graph")
    c.add_argument("--tags", required=True, help="JSON {test-or-proof-name: id | [ids]}")
    c.add_argument("--json", action="store_true")
    c.set_defaults(fn=cmd_coverage)
    t = sub.add_parser("top", help="clauses ranked by number of transitive dependents")
    t.add_argument("graph")
    t.add_argument("-n", type=int, default=10)
    t.add_argument("--spec", help="rank clauses of this spec (default: the first graph's)")
    t.add_argument("--with", dest="with_", action="append")
    add_closure_args(t)
    t.set_defaults(fn=cmd_top)
    return p


def main(argv=None):
    args = make_parser().parse_args(argv)
    try:
        return args.fn(args)
    except SpecError as e:
        print("clausegraph: error: %s" % e, file=sys.stderr)
        return 2
    except UsageError as e:
        print("clausegraph: error: %s" % e, file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
