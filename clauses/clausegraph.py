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

    def __init__(self, graphs, statements=None):
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
        st_by_spec = {}
        for st in statements or ():
            validate_statements(st)
            g = by_spec.get(st["spec"])
            if g is None:
                raise UsageError("statements of %s-%s given without the %s graph"
                                 % (st["spec"], st["version"], st["spec"]))
            if g["version"] != st["version"] or \
                    g["source"]["sha256"] != st["graph"]["source_sha256"]:
                raise UsageError("statements file %s-%s is stale for graph %s-%s; rerun "
                                 "`clausegraph.py statements`" % (st["spec"], st["version"],
                                                                g["spec"], g["version"]))
            st_by_spec[st["spec"]] = st
        self.statements = st_by_spec
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
                # with the statement layer loaded, its statement edges replace clause edges
                if e["kind"] in EDGE_DEP_KINDS and g["spec"] not in st_by_spec:
                    self.out[e["src"]].add(e["dst"])
                    self.inn[e["dst"]].add(e["src"])
            self.aliases[g["spec"]] = g["aliases"]
        for st in st_by_spec.values():
            for x in st["statements"]:
                par = x["parent"] or x["clause"]
                self.nodes[x["id"]] = {"id": x["id"], "kind": "statement", "parent": par,
                                       "stmt_kind": x["kind"], "n": x["n"]}
                self.children[par].append(x["id"])
            for x in st["statements"]:
                for dst in statement_targets(x):
                    self.out[x["id"]].add(dst)
                    self.inn[dst].add(x["id"])
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
        if RE_STMT_ID.match(token) and spec in self.by_spec and spec not in self.statements:
            hint = " (a statement id: load the statement layer, --statements)"
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

    def _reverse(self, nid, closure="anchored", max_depth=None, skip=()):
        self._check(closure, max_depth)
        start = (set(self.descendants(nid)) | set(self.ancestors(nid))) - set(skip)
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
        out = [sorted([x for x in lv if self.kind(x) in ("clause", "statement")],
                      key=natural_key) for lv in levels]
        return [lv for lv in out if lv]

    def reverse_cone(self, nid, closure="anchored", max_depth=None, skip=()):
        _, levels = self._reverse(nid, closure, max_depth, skip)
        # the query's own ancestors (its sections; for a statement also its clause) are
        # context, not dependents
        allx = set(self.descendants(nid)).union(*levels)
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

    def independent(self, a, b, closure="anchored", max_depth=None, containers="count"):
        """containers='ignore': citations of a section or clause enclosing both a and b do not
        make them dependent (such a citation cannot tell the two apart)."""
        if containers not in ("count", "ignore"):
            raise UsageError("containers must be count or ignore, got %r" % containers)
        skip = set(self.ancestors(a)) & set(self.ancestors(b)) if containers == "ignore" \
            else set()
        fa = self.forward_cone(a, closure, max_depth)
        fb = self.forward_cone(b, closure, max_depth)
        ra = self.reverse_cone(a, closure, max_depth, skip)
        rb = self.reverse_cone(b, closure, max_depth, skip)
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


def coverage(g, tags, statements=None):
    validate_graph(g)
    m = Merged([g], [statements] if statements else None)
    if not isinstance(tags, dict):
        raise UsageError("tags must be a JSON object mapping test/proof names to ids")
    strong = collections.defaultdict(list)
    weak = collections.defaultdict(list)
    s_strong = collections.defaultdict(list)
    s_weak = collections.defaultdict(list)
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
                fm = RE_FOREIGN_ID.match(t) or RE_STMT_ID.match(t)
                if fm and fm.group(1).split("-")[0] != g["spec"]:
                    foreign.append(t)  # an id or SPEC:alias of another spec: listed, not used
                    continue
                raise UsageError("tag %r: %s" % (name, e))
            kind = m.kind(nid)
            if kind == "statement":
                for d in m.descendants(nid):
                    s_strong[d].append(name)
                strong[RE_STMT_ID.match(nid).group("clause")].append(name)
                continue
            for d in m.descendants(nid):
                if m.kind(d) == "statement":
                    s_weak[d].append(name)
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
    rep = {"spec": g["spec"], "version": g["version"], "totals": totals,
           "sections": sections, "foreign": sorted(set(foreign)),
           "witnesses": {k: sorted(v) for k, v in sorted(strong.items())}}
    if statements:
        rep["statements"] = _statement_coverage(statements, s_strong, s_weak)
    return rep


def _statement_coverage(st, s_strong, s_weak):
    """Statement-granularity coverage: strong = the statement (or its enumerating parent)
    is tagged; weak = only its clause or a section is tagged."""
    def status(x):
        return "strong" if x in s_strong else ("weak" if x in s_weak else "uncovered")

    tot = collections.Counter()
    by_kind = collections.OrderedDict((k, collections.Counter()) for k in KINDS)
    by_sec = collections.OrderedDict()
    for x in st["statements"]:
        k = status(x["id"])
        tot["statements"] += 1
        tot[k] += 1
        by_kind[x["kind"]]["statements"] += 1
        by_kind[x["kind"]][k] += 1
        sec = by_sec.setdefault(x["section"], {"section": x["section"], "statements": 0,
                                               "strong": [], "weak": 0, "uncovered": 0})
        sec["statements"] += 1
        if k == "strong":
            sec["strong"].append(x["id"])
        else:
            sec[k] += 1
    keys = ("statements", "strong", "weak", "uncovered")
    return {"totals": {k: tot.get(k, 0) for k in keys},
            "by_kind": {kd: {k: c.get(k, 0) for k in keys} for kd, c in by_kind.items()},
            "sections": list(by_sec.values())}


# ---------------------------------------------------------------------------
# Diff between two versions of one spec

SIMILAR_RATIO = 0.6


def match_clauses(a, b):
    """{a clause id: (b clause id, 'hash'|'alias'|'similar')} between two versions."""
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
    return pairs


def diff_graphs(a, b):
    pairs = match_clauses(a, b)
    ca = [n for n in a["nodes"] if n["kind"] == "clause"]
    cb = [n for n in b["nodes"] if n["kind"] == "clause"]
    sec = lambda n: n["parent"][len(a["spec"]) + 1:]  # noqa: E731
    used_b = {bid for bid, _ in pairs.values()}
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
# Statement layer: clauses -> atomic Lamport-style statements
# (docs/design/clause-graph.md, "Statement layer"). Derived from a graph JSON; never
# reads or edits spec bytes.

STMT_SCHEMA = "bennett-statements/1"
RE_STMT_ID = re.compile(
    r"^(?P<clause>(?P<spec>[A-Z][A-Z0-9]*)-[A-Za-z0-9.]+-P\d+)\.S(?P<k>\d+)(?:\.(?P<j>\d+))?$")
RE_DEPS = re.compile(r"\[deps:\s*([^\]]*)\]")
RE_DEPS_STRIP = re.compile(r"\s*\[deps:[^\]]*\]\s*")
RE_DEPS_NONE = re.compile(r"^(?:none|-|–|—)?$")
ABBREVIATIONS = {"e.g", "i.e", "cf", "vs", "viz", "approx", "resp", "al", "incl", "esp",
                 "Fig", "fig", "Eq", "eq"}
RE_ABBR = re.compile(r"(?<![\w.])([A-Za-z]+(?:\.[A-Za-z]+)*)$")
SENT_CLOSERS = "*_\"”’'"
SENT_OPENERS = "*_(\"“‘'"
RE_CAPTION = re.compile(r"^\*\*[^*]+\*\*(?:\s*\([^()]*\))?[.:]?$")
RE_CODE_SPAN = re.compile(r"(`+)(.+?)\1", re.S)
RE_MODAL = re.compile(r"\b(MUST NOT|MUST|SHOULD NOT|SHOULD|MAY|SHALL NOT|SHALL|REQUIRED|"
                      r"RECOMMENDED|OPTIONAL)\b")
RE_LOWER_MODAL = re.compile(r"\b(must not|must|cannot|shall not|shall|may only|may not|never)\b")
DECL_VERBS = (
    "is", "are", "was", "were", "be", "been", "has", "have", "had", "holds", "hold", "returns",
    "rejects", "traps", "faults", "takes", "produces", "emits", "contains", "requires", "means",
    "denotes", "equals", "matches", "writes", "reads", "sets", "clears", "applies", "ends",
    "starts", "begins", "consists", "includes", "gives", "records", "follows", "carries",
    "keeps", "runs", "executes", "computes", "checks", "validates", "accepts", "refuses",
    "fails", "succeeds", "restores", "releases", "allocates", "frees", "maps", "becomes",
    "remains", "stays", "exists", "exist", "can", "does", "do", "governs", "defines",
    "appears", "appear", "counts", "enters", "fixes", "chooses", "reverses", "retains", "may",
    "should", "will", "gets", "passes", "lifts", "lowers", "rewrites", "replaces", "creates",
    "adds", "removes", "compiles", "measures", "raises", "treats", "belongs", "differs",
    "depends", "satisfies", "implies", "precedes", "preserves", "needs", "uses", "lists",
    "names", "yields", "resolves", "refers", "marks", "enforces", "forbids", "permits",
    "allows", "selects", "stores", "loads", "calls", "invokes", "owns", "consumes",
    "enumerates", "describes", "reports", "introduces", "commits", "aborts", "survives",
    "agree", "agrees", "apply", "hold", "contain", "carry", "occur", "occurs", "reach",
    "reaches", "rejected", "accepted", "trapped", "faulted", "lie", "lies",
    # base forms after plural subjects, and further third-person forms seen in the specs
    "fail", "produce", "retain", "take", "use", "revise", "revises", "append", "appends",
    "declare", "declares", "supply", "supplies", "return", "reject", "enter", "leave", "keep",
    "run", "write", "read", "follow", "become", "remain", "release", "allocate", "match",
    "mean", "denote", "need", "give", "cover", "covers", "require", "emit", "compute",
    "check", "validate", "accept", "restore", "replace", "create", "add", "remove", "treat",
    "preserve", "satisfy", "imply", "precede", "yield", "resolve", "refer", "enforce",
    "forbid", "permit", "allow", "select", "store", "load", "invoke", "own", "consume",
    "receive", "receives", "share", "shares", "differ", "count", "fix", "choose", "reverse",
    "admits", "admit", "admitted", "drops", "drop", "ignores", "ignore", "costs", "cost",
    "wraps", "wrap", "pairs", "sees", "see", "lands", "land", "matters", "states", "state",
    "says", "say", "lets", "let", "makes", "make", "goes", "go", "comes", "come", "tests",
    "test", "names", "name", "lists", "list", "maps", "map", "records", "record",
)
RE_DECL = re.compile(r"\b(%s)\b" % "|".join(sorted(set(DECL_VERBS))), re.I)
RE_EXAMPLE_LEAD = re.compile(r"^(?:\*\*|\*|_)?(?:e\.g\.|For example|For instance|Examples?\b)")
RE_NN_MARK = re.compile(r"^\*?\((?:non-normative|deferred)\b[^)]*\)\*?")
RE_INLINE_ASIDE = re.compile(r"\*\((?:non-normative|deferred)\b[^)]*\)\*")
RE_NN_TITLE = re.compile(r"\((?:non-normative|deferred)\)|^Changes from", re.I)
RE_EXAMPLE_TITLE = re.compile(r"^(?:Examples?\b|Worked examples)|\bExamples$")
RE_ENUM = re.compile(r"(?:(?<=^)|(?<=[\s;:,—(]))\((\d{1,2}|[ivx]{1,4}|[a-z])\)(?=[\s,.;:)]|$)")
RE_ENUM_REFWORD = re.compile(
    r"(?:§\S+|\b(?:parts?|items?|steps?|rules?|cases?|in|of|see|by|from|per))\s*$", re.I)
RE_ENUM_TAIL = re.compile(r"[\s,;]*(?:\b(?:and|or)\b)?[\s,;]*$")
ROMAN = ["i", "ii", "iii", "iv", "v", "vi", "vii", "viii", "ix", "x"]
KIND_ABBR = collections.OrderedDict([("requirement", "req"), ("definition", "def"),
                                     ("non-normative", "non"), ("example", "ex"),
                                     ("uncertain", "unc")])
KINDS = tuple(KIND_ABBR)
# Classifier rules, first match wins (listed in the design doc).
CLASSIFIER_RULES = collections.OrderedDict([
    ("K1", "example: the statement is a fenced code block"),
    ("K2", "non-normative: the section or an enclosing section is front matter, a change log "
           "(title 'Changes from ...') or titled *(non-normative)* / *(deferred)*"),
    ("K3", "non-normative: the sentence starts with *(non-normative...)* or *(deferred...)*, or "
           "follows such a sentence in the same paragraph; uncertain if it contains an RFC 2119 "
           "keyword"),
    ("K4", "requirement: contains an upper-case RFC 2119 keyword (MUST, MUST NOT, SHOULD, MAY, "
           "SHALL, REQUIRED, ...)"),
    ("K5", "definition: introduces a defined term (bold) or a rule label of its clause"),
    ("K6", "example: starts with e.g. / For example / Example, contains e.g., or lies in an "
           "examples section"),
    ("K7", "requirement: lower-case normative word (must, must not, cannot, shall, may only, "
           "may not, never)"),
    ("K8", "requirement: a declarative verb from the fixed list (is, are, returns, rejects, "
           "traps, ...)"),
    ("K9", "requirement: a table row (normative table: instruction, default, error-code "
           "rows state their content in columns, not sentences)"),
    ("K10", "uncertain: none of the above (verbless fragment, list-item fragment)"),
    ("K11", "sub-statement classified K10 inherits the kind of its enumerating sentence"),
])
UNCERTAIN_REASON = {"K3": "RFC 2119 keyword inside a *(non-normative)* passage",
                    "K10": "no RFC keyword, defined term, example marker or declarative verb"}


def _code_spans(s):
    """[(start, end)] of inline code spans: a backtick run closed by a run of equal length."""
    out, i, n = [], 0, len(s)
    while i < n:
        if s[i] != "`":
            i += 1
            continue
        j = i
        while j < n and s[j] == "`":
            j += 1
        run, k, close = j - i, j, -1
        while k < n:
            if s[k] == "`":
                e = k
                while e < n and s[e] == "`":
                    e += 1
                if e - k == run:
                    close = e
                    break
                k = e
            else:
                k += 1
        if close < 0:
            i = j
            continue
        out.append((i, close))
        i = close
    return out


def _protected(p):
    """Mask of positions inside code spans or matched ()/[] pairs (no sentence split there)."""
    prot = bytearray(len(p))
    for a, b in _code_spans(p):
        prot[a:b] = b"\x01" * (b - a)
    stack = []
    for i, ch in enumerate(p):
        if prot[i]:
            continue
        if ch in "([":
            stack.append((ch, i))
        elif ch in ")]" and stack and stack[-1][0] == ("(" if ch == ")" else "["):
            _, o = stack.pop()
            prot[o + 1:i] = b"\x01" * (i - o - 1)
    return prot


def split_sentences(p):
    """Sentence spans [(start, end)] of one prose paragraph p (rules: design doc)."""
    n = len(p)
    prot = _protected(p)
    cuts = []
    for m in RE_DEPS.finditer(p):
        if prot[m.start()]:
            continue
        e = m.end()
        while e < n and p[e] in ".!?":
            e += 1
        cuts.append(e)
    for i, ch in enumerate(p):
        if ch not in ".!?" or prot[i]:
            continue
        if ch == "." and ((i + 1 < n and p[i + 1] == ".") or (i > 0 and p[i - 1] == ".")):
            continue  # ellipsis
        j = i + 1
        while j < n and p[j] in SENT_CLOSERS:
            j += 1
        if j >= n or not p[j].isspace():
            continue
        k = j
        while k < n and p[k].isspace():
            k += 1
        if k >= n:
            continue
        if ch == ".":
            am = RE_ABBR.search(p[max(0, i - 24):i])
            if am and am.group(1) in ABBREVIATIONS:
                continue
        if p.startswith("[deps:", k):
            continue  # the marker closes the sentence (cut above)
        q = k
        while q < n and p[q] in SENT_OPENERS:
            q += 1
        if (q < n and (p[q].isupper() or p[q].isdigit() or p[q] in "`§["))                 or RE_NN_MARK.match(p[k:k + 48]):
            cuts.append(j)
    spans, a = [], 0
    for c in sorted(set(c for c in cuts if 0 < c < n)) + [n]:
        s, e = a, c
        while s < e and p[s].isspace():
            s += 1
        while e > s and p[e - 1].isspace():
            e -= 1
        if s < e:
            spans.append((s, e))
        a = c
    # a sentence that is only a bold caption ("**Requirement.**") joins the next one
    out = []
    for sp in spans:
        if out and RE_CAPTION.match(norm_ws(p[out[-1][0]:out[-1][1]])) \
                and out[-1][2] == "caption":
            out[-1] = (out[-1][0], sp[1], "")
            continue
        out.append((sp[0], sp[1], "caption" if RE_CAPTION.match(norm_ws(p[sp[0]:sp[1]])) else ""))
    return [(s, e) for s, e, _ in out]


def _enum_succ(a, style):
    if style == "num":
        return str(int(a) + 1)
    if style == "roman":
        k = ROMAN.index(a) if a in ROMAN else -1
        return ROMAN[k + 1] if 0 <= k < len(ROMAN) - 1 else None
    return chr(ord(a) + 1) if a < "z" else None


def split_enumeration(text):
    """[(start, end)] of the enumerated items (1)…(n) / (a)… / (i)… inside one sentence,
    or [] when there is no run of at least two consecutive enumerators."""
    code = _code_spans(text)
    cands = []
    for m in RE_ENUM.finditer(text):
        if any(a <= m.start() < b for a, b in code):
            continue
        if RE_ENUM_REFWORD.search(text[max(0, m.start() - 24):m.start()]):
            continue  # "§9.2 (5b)", "part (4)": a reference, not an enumerator
        cands.append((m.start(), m.group(1)))
    for k, (pos, mk) in enumerate(cands):
        style = {"1": "num", "a": "letter", "i": "roman"}.get(mk)
        if style is None:
            continue
        run, want = [pos], _enum_succ(mk, style)
        for pos2, mk2 in cands[k + 1:]:
            if mk2 == want:
                run.append(pos2)
                want = _enum_succ(mk2, style)
        if len(run) >= 2:
            ends = run[1:] + [len(text)]
            return [(a, b) for a, b in zip(run, ends)]
    return []


def _clause_blocks(nd):
    """[(kind 'prose'|'code', start, end, masked_text)] of a clause node's text."""
    text = nd["text"]
    lines = text.split("\n")
    offs, o = [], 0
    for ln in lines:
        offs.append(o)
        o += len(ln) + 1
    blocks, prose, code, fence = [], [], None, None

    def flush():
        if prose:
            blocks.append(("prose", offs[prose[0]], offs[prose[-1]] + len(lines[prose[-1]])))
            prose.clear()

    for i, ln in enumerate(lines):
        if fence is not None:
            m = RE_FENCE.match(ln)
            if m and m.group(2)[0] == fence[0] and len(m.group(2)) >= len(fence) \
                    and not m.group(3).strip():
                blocks.append(("code", code, offs[i] + len(ln)))
                fence = None
            continue
        m = RE_FENCE.match(ln)
        if m:
            flush()
            fence, code = m.group(2), offs[i]
            continue
        if not ln.strip():
            flush()
            continue
        prose.append(i)
    flush()
    if fence is not None:
        blocks.append(("code", code, len(text)))
    out = []
    for k, (kind, a, b) in enumerate(blocks):
        p = text[a:b]
        if kind == "prose" and a == 0 and nd.get("clause_kind") == "item":
            m = RE_LIST.match(p)
            if m:
                p = " " * m.end() + p[m.end():]
        if kind == "prose" and nd.get("clause_kind") == "row":
            s, e = p.find("|"), p.rfind("|")
            if s >= 0 and p[:s].strip() == "":
                p = p[:s] + " " + p[s + 1:]
            if e > s and p[e + 1:].strip() == "":
                p = p[:e] + " " + p[e + 1:]
        out.append((kind, a, b, p))
    return out


def _clean(raw):
    t = norm_ws(RE_DEPS_STRIP.sub(" ", raw))
    return re.sub(r"\s+([.!?])$", r"\1", t)


def _plain(text):
    return RE_INLINE_ASIDE.sub(" ", RE_CODE_SPAN.sub(" ", text))


def classify_statement(text, code=False, nn_section=False, nn_passage=False,
                       example_section=False, defines=(), row=False):
    """(kind, rule, modal keywords). Rules K1-K10: CLASSIFIER_RULES."""
    if code:
        return "example", "K1", []
    plain = _plain(text)
    modal = sorted(set(RE_MODAL.findall(plain)))
    if nn_section:
        return "non-normative", "K2", modal
    if nn_passage:
        return ("uncertain" if modal else "non-normative"), "K3", modal
    if modal:
        return "requirement", "K4", modal
    if defines:
        return "definition", "K5", modal
    if RE_EXAMPLE_LEAD.match(plain.lstrip()) or "e.g." in plain or example_section:
        return "example", "K6", modal
    if RE_LOWER_MODAL.search(plain):
        return "requirement", "K7", modal
    if RE_DECL.search(plain):
        return "requirement", "K8", modal
    if row:
        return "requirement", "K9", modal
    return "uncertain", "K10", modal


def _section_flags(g):
    """{section id: (non-normative, example section, terminology)} with title inheritance."""
    secs = {n["id"]: n for n in g["nodes"] if n["kind"] == "section"}
    out = {}
    for sid, n in secs.items():
        nn = n["number"] == "front"
        ex = False
        cur = n
        while cur is not None:
            t = cur["title"]
            if RE_NN_TITLE.search(t):
                nn = True
            if RE_EXAMPLE_TITLE.search(t) and "(normative" not in t:
                ex = True
            cur = secs.get(cur.get("parent"))
        out[sid] = (nn, ex)
    return out


class _RefCtx:
    def __init__(self, g, xg):
        self.spec = g["spec"]
        self.version = g["version"]
        self.local = SectionIndex.from_graph(g)
        self.xspec_name = xg["spec"] if xg else None
        self.xspec = SectionIndex.from_graph(xg) if xg else None


class _Span:
    """Minimal clause stand-in for scan_refs."""

    def __init__(self, text, start):
        self.text = text
        self.start = start


def _check_xspec(g, xg):
    xs = g.get("xspec")
    if not xs:
        return None
    if xg is None:
        raise UsageError("%s-%s was built against %s-%s; pass that graph with --xspec"
                         % (g["spec"], g["version"], xs["spec"], xs["version"]))
    validate_graph(xg, "xspec")
    if xg["spec"] != xs["spec"] or xg["source"]["sha256"] != xs["sha256"]:
        raise UsageError("%s-%s was built against %s-%s (sha256 %s...), not against the given "
                         "%s-%s (sha256 %s...)" % (g["spec"], g["version"], xs["spec"],
                                                   xs["version"], xs["sha256"][:12], xg["spec"],
                                                   xg["version"], xg["source"]["sha256"][:12]))
    return xg


def build_statements(g, xg=None):
    """Statement layer of graph g (and the --xspec graph xg it was resolved against)."""
    validate_graph(g)
    xg = _check_xspec(g, xg)
    spec = g["spec"]
    secs = collections.OrderedDict((n["id"], n) for n in g["nodes"] if n["kind"] == "section")
    flags = _section_flags(g)
    clauses = [n for n in g["nodes"] if n["kind"] == "clause"]
    nodes = {n["id"]: n for n in g["nodes"]}
    kinds = g.get("alias_kinds", {})
    clause_terms = collections.defaultdict(list)
    clause_labels = collections.defaultdict(list)
    for al, cid in g["aliases"].items():
        (clause_labels if kinds.get(al) == "label" else clause_terms)[cid].append(al)
    for amb in g.get("ambiguous_terms", []):
        for cid in amb["clauses"]:
            clause_terms[cid].append(amb["term"])
    stmts = []
    for nd in clauses:
        top = []
        for kind, a, b, p in _clause_blocks(nd):
            if kind == "code":
                top.append({"raw": nd["text"][a:b], "a": a, "code": True, "nn": False})
                continue
            nn = False
            for s, e in split_sentences(p):
                raw = p[s:e]
                txt = norm_ws(raw)
                if RE_NN_MARK.match(txt):
                    nn = True
                top.append({"raw": raw, "a": a + s, "code": False, "nn": nn})
        # defined terms and labels: the first statement of the clause that carries them
        placed = set()
        for st in top:
            st["defines"] = []
            if st["code"]:
                continue
            txt = _clean(st["raw"])
            bold = {norm_ws(m.group(1)).strip().rstrip(".:,;").strip()
                    for m in RE_BOLD.finditer(txt)}
            for t in clause_terms.get(nd["id"], ()):
                if t in bold and t not in placed:
                    st["defines"].append(t)
                    placed.add(t)
            for lb in clause_labels.get(nd["id"], ()):
                if lb not in placed and re.search(LABEL_LEFT + re.escape(lb) + LABEL_RIGHT, txt):
                    st["defines"].append(lb)
                    placed.add(lb)
        rest = [x for x in list(clause_terms.get(nd["id"], ())) + list(clause_labels.get(nd["id"], ()))
                if x not in placed]
        if rest and top:
            first = next((x for x in top if not x["code"]), top[0])
            first["defines"].extend(rest)
        nn_sec, ex_sec = flags[nd["parent"]]
        for k, st in enumerate(top, 1):
            sid = "%s.S%d" % (nd["id"], k)
            text = st["raw"] if st["code"] else _clean(st["raw"])
            kind, rule, modal = classify_statement(
                text, code=st["code"], nn_section=nn_sec, nn_passage=st["nn"],
                example_section=ex_sec, defines=st["defines"],
                row=nd.get("clause_kind") == "row")
            markers = [] if st["code"] else RE_DEPS.findall(st["raw"])
            line = nd["line"] + nd["text"].count("\n", 0, st["a"])
            rec = {"id": sid, "clause": nd["id"], "section": nd["parent"], "n": "<1>%d" % k,
                   "level": 1, "parent": None, "kind": kind, "rule": rule, "modal": modal,
                   "defines": sorted(set(st["defines"])), "text": text,
                   "hash": clause_hash(text), "line": line,
                   "marker": [norm_ws(x) for x in markers] if markers else None,
                   "_raw": st["raw"], "_code": st["code"]}
            stmts.append(rec)
            if st["code"]:
                continue
            for j, (ea, eb) in enumerate(split_enumeration(text), 1):
                sub = text[ea:eb].strip()
                if eb < len(text):
                    sub = RE_ENUM_TAIL.sub("", sub)
                skind, srule, smodal = classify_statement(
                    sub, nn_section=nn_sec, nn_passage=st["nn"], example_section=ex_sec)
                if srule == "K10" and kind != "uncertain":
                    skind, srule = kind, "K11"
                stmts.append({"id": "%s.%d" % (sid, j), "clause": nd["id"],
                              "section": nd["parent"], "n": "<2>%d" % j, "level": 2,
                              "parent": sid, "kind": skind, "rule": srule, "modal": smodal,
                              "defines": [], "text": sub, "hash": clause_hash(sub),
                              "line": line, "marker": None, "_raw": sub, "_code": False})
    for s in stmts:
        if s["kind"] == "uncertain":
            s["uncertain"] = UNCERTAIN_REASON[s["rule"]]
    _statement_deps(g, xg, stmts, nodes)
    for s in stmts:
        del s["_raw"], s["_code"]
    unresolved = [u for s in stmts for u in s.pop("_unresolved")]
    by_kind = collections.Counter(s["kind"] for s in stmts)
    by_section = collections.OrderedDict()
    for sid in secs:
        c = collections.Counter(s["kind"] for s in stmts if s["section"] == sid)
        if c:
            by_section[sid] = {k: c.get(k, 0) for k in KINDS}
    stats = {"statements": len(stmts), "top_level": sum(1 for s in stmts if s["level"] == 1),
             "sub_statements": sum(1 for s in stmts if s["level"] == 2),
             "clauses": len(clauses), "by_kind": {k: by_kind.get(k, 0) for k in KINDS},
             "by_rule": dict(sorted(collections.Counter(s["rule"] for s in stmts).items(),
                                    key=lambda kv: natural_key(kv[0]))),
             "markers": sum(1 for s in stmts if s["marker"] is not None),
             "declared_edges": sum(len(s["deps"]["declared"]) for s in stmts),
             "inferred_edges": sum(len(s["deps"]["inferred"]) for s in stmts),
             "superseded_edges": sum(1 for s in stmts for d in s["deps"]["inferred"]
                                     if d["status"] == "superseded"),
             "unresolved_markers": len(unresolved), "by_section": by_section}
    return {"schema": STMT_SCHEMA, "spec": spec, "version": g["version"],
            "graph": {"source_file": g["source"]["file"], "source_sha256": g["source"]["sha256"]},
            "xspec": g.get("xspec"), "rules": CLASSIFIER_RULES, "statements": stmts,
            "unresolved": unresolved, "stats": stats}


def _statement_deps(g, xg, stmts, nodes):
    """Fill s['deps'] = {declared, inferred} and s['_unresolved'] for every statement."""
    spec = g["spec"]
    by_id = {s["id"]: s for s in stmts}
    defining = {}  # alias -> defining statement id
    for s in stmts:
        for al in s["defines"]:
            defining.setdefault(al, s["id"])
    for al, cid in g["aliases"].items():
        defining.setdefault(al, "%s.S1" % cid if "%s.S1" % cid in by_id else cid)
    kinds = g.get("alias_kinds", {})
    labels = [a for a, k in kinds.items() if k == "label"]
    terms = [a for a, k in kinds.items() if k == "term"]
    rl = alias_regex(labels, LABEL_LEFT, LABEL_RIGHT)
    rt = alias_regex(terms, TERM_LEFT, TERM_RIGHT)
    by_prefix = collections.defaultdict(dict)
    for lb in labels:
        m = RE_LABEL_SPLIT.match(lb)
        if m:
            by_prefix[m.group(1)][int(m.group(2))] = lb
    ctx = _RefCtx(g, xg)
    xnodes = {n["id"]: n for n in xg["nodes"]} if xg else {}

    def related(a, b):  # same statement, or one is the other's enumerating parent
        return a == b or by_id[a]["parent"] == b or by_id.get(b, {}).get("parent") == a

    for s in stmts:
        inferred = collections.OrderedDict()

        def add(dst, via, name):
            if dst is None or (dst in by_id and related(s["id"], dst)):
                return
            if dst not in inferred:
                inferred[dst] = {"dst": dst, "via": via, "name": name, "status": "inferred"}

        # the marker is metadata, not spec prose: never infer from its own ids
        text = RE_DEPS.sub(lambda m: " " * len(m.group(0)), s["_raw"])
        if rl:
            for m in rl.finditer(text):
                hits = [m.group(1)]
                rm = re.match(r"–(%s)" % LABEL, text[m.end():m.end() + 24])
                if rm:
                    a, b = RE_LABEL_SPLIT.match(m.group(1)), RE_LABEL_SPLIT.match(rm.group(1))
                    if a and b and a.group(1) == b.group(1):
                        hits.extend(by_prefix[a.group(1)][k]
                                    for k in range(int(a.group(2)) + 1, int(b.group(2)))
                                    if k in by_prefix[a.group(1)])
                for h in hits:
                    add(defining[h], "label", h)
        if rt and not s["_code"]:
            for m in rt.finditer(text):
                add(defining[m.group(1)], "term", m.group(1))
        for kind, dst, _line, _unres in scan_refs(_Span(text, s["line"]), ctx):
            if dst is not None:
                add(dst, kind, None)
        declared, unresolved = [], []
        if s["marker"] is not None:
            for body in s["marker"]:
                body = body.strip()
                if RE_DEPS_NONE.match(body):
                    continue
                for tok in [t.strip().strip("`") for t in body.split(",")]:
                    dst, why = _resolve_declared(tok, spec, by_id, nodes, g["aliases"],
                                                 defining, xg, xnodes)
                    if dst is not None and dst == s["id"]:
                        dst, why = None, "self"
                    if dst is None:
                        unresolved.append({"src": s["id"], "line": s["line"], "ref": tok,
                                           "reason": why})
                    elif dst not in [d["dst"] for d in declared]:
                        declared.append({"dst": dst, "ref": tok})
            dset = [d["dst"] for d in declared]
            for d in inferred.values():
                covered = any(d["dst"] == x or d["dst"].startswith(x + ".")
                              or d["dst"].startswith(x + "-") for x in dset)
                d["status"] = "confirmed" if covered else "superseded"
        for d in inferred.values():
            if d["name"] is None:
                del d["name"]
        s["deps"] = {"declared": declared, "inferred": list(inferred.values())}
        s["_unresolved"] = unresolved


def _resolve_declared(tok, spec, by_id, nodes, aliases, defining, xg, xnodes):
    """Declared dependency token -> (target id, None) or (None, reason)."""
    if not tok:
        return None, "empty"
    if tok in by_id:
        return tok, None
    if tok in nodes and nodes[tok]["kind"] in ("clause", "section"):
        return tok, None
    if ":" in tok:
        sp, al = tok.split(":", 1)
        if sp == spec and al in aliases:
            return defining[al], None
        if xg and sp == xg["spec"] and al in xg["aliases"]:
            return xg["aliases"][al], None
        return None, "unknown-alias"
    if tok in aliases:
        return defining[tok], None
    m = RE_STMT_ID.match(tok)
    head = tok.split("-", 1)[0]
    if xg and head == xg["spec"]:
        if tok in xnodes or (m and m.group("clause") in xnodes):
            return tok, None
        return None, "no-such-id"
    if head != spec and re.match(r"^[A-Z][A-Z0-9]*$", head) and "-" in tok:
        return None, "xspec-not-loaded"
    if m and m.group("clause") in nodes:
        return None, "no-such-statement"
    return None, "no-such-id"


def validate_statements(st, what="statements"):
    if not isinstance(st, dict) or st.get("schema") != STMT_SCHEMA:
        raise UsageError("%s is not a %s file" % (what, STMT_SCHEMA))
    for k in ("spec", "version", "graph", "statements"):
        if k not in st:
            raise UsageError("%s lacks key %r" % (what, k))


def statement_targets(s):
    """Dependency targets a query follows: declared, plus inferred not superseded."""
    out = [d["dst"] for d in s["deps"]["declared"]]
    out.extend(d["dst"] for d in s["deps"]["inferred"] if d["status"] != "superseded")
    return out


def _md_text(s):
    if s["rule"] == "K1":
        body = [ln for ln in s["text"].split("\n") if ln.strip() and not RE_FENCE.match(ln)]
        first = norm_ws(body[0]) if body else ""
        return "(code block, %d lines) %s" % (s["text"].count("\n") + 1,
                                              first[:80] + ("…" if len(first) > 80 else ""))
    return s["text"]


def statements_markdown(st, g):
    secs = collections.OrderedDict((n["id"], n) for n in g["nodes"] if n["kind"] == "section")
    name = "%s-%s" % (st["spec"], st["version"])
    out = ["# Statements of %s" % name, "",
           "Generated by `python3 -I clauses/clausegraph.py statements clauses/%s.json` from "
           "`%s` (sha256 %s...). Do not edit; rebuild. Rules and ids: "
           "`docs/design/clause-graph.md`, \"Statement layer\"."
           % (name, st["graph"]["source_file"], st["graph"]["source_sha256"][:12]), "",
           "Kinds: `[req]` requirement, `[def]` definition, `[non]` non-normative, `[ex]` "
           "example, `[unc]` uncertain. `deps:` inferred dependencies with the term, label or "
           "`§` reference behind each (`~` = superseded by a declared marker); `declared:` the "
           "spec's own `[deps: ...]` markers.", "", "## Counts", ""]
    t = st["stats"]
    out.append("%d statements (%d top-level, %d enumerated sub-statements) in %d clauses; "
               "%s; %d declared markers, %d unresolved."
               % (t["statements"], t["top_level"], t["sub_statements"], t["clauses"],
                  ", ".join("%s %d" % (k, t["by_kind"][k]) for k in KINDS), t["markers"],
                  t["unresolved_markers"]))
    out += ["", "| section | %s | total |" % " | ".join(KIND_ABBR.values()),
            "|---|%s---|" % ("---|" * len(KINDS))]
    for sid, c in t["by_section"].items():
        out.append("| %s | %s | %d |" % (sid, " | ".join(str(c[k]) for k in KINDS),
                                         sum(c.values())))
    if st["unresolved"]:
        out += ["", "Unresolved markers: " + "; ".join(
            "%s `%s` (%s)" % (u["src"], u["ref"], u["reason"]) for u in st["unresolved"])]
    out += ["", "## Listing", ""]
    by_sec = collections.defaultdict(list)
    for s in st["statements"]:
        by_sec[s["section"]].append(s)
    for sid, sec in secs.items():
        if sid not in by_sec:
            continue
        head = "Front matter" if sec["number"] == "front" else "§%s %s" % (sec["number"],
                                                                         sec["title"])
        out += ["### %s   [%s]" % (head, sid), "", "````text"]
        cur = None
        for s in by_sec[sid]:
            if s["clause"] != cur:
                cur = s["clause"]
                out.append(cur)
            line = "%s%s [%s] %s" % ("  " * s["level"], s["n"], KIND_ABBR[s["kind"]],
                                     _md_text(s))
            inf = ["%s%s%s" % ("~" if d["status"] == "superseded" else "", d["dst"],
                               " (%s)" % d["name"] if d.get("name") else "")
                   for d in s["deps"]["inferred"]]
            if inf:
                line += "   deps: " + ", ".join(inf)
            if s["deps"]["declared"]:
                line += "   declared: " + ", ".join(d["dst"] for d in s["deps"]["declared"])
            out.append(line)
        out += ["````", ""]
    return "\n".join(out)


def statements_path(graph_path):
    if not graph_path.endswith(".json") or graph_path.endswith(".statements.json"):
        raise UsageError("expected a graph .json path, got %s" % graph_path)
    return graph_path[:-len(".json")] + ".statements.json"


def load_statements(graph_path, required=True):
    path = statements_path(graph_path)
    if not os.path.exists(path):
        if required:
            raise UsageError("%s not found; run `clausegraph.py statements %s` first"
                             % (path, graph_path))
        return None
    st = load_json(path)
    validate_statements(st, path)
    return st


def match_statements(a, b, sa, sb):
    """Statement pairs between two versions: hash, then similar text inside matched clauses."""
    pairs_c = match_clauses(a, b)
    la, lb = sa["statements"], sb["statements"]
    pairs, used = {}, set()
    hb = collections.defaultdict(list)
    for s in lb:
        hb[s["hash"]].append(s["id"])
    for s in la:
        lst = hb.get(s["hash"])
        if lst:
            pairs[s["id"]] = (lst.pop(0), "hash")
            used.add(pairs[s["id"]][0])
    ra, rb = collections.defaultdict(list), collections.defaultdict(list)
    for s in la:
        if s["id"] not in pairs:
            ra[s["clause"]].append(s)
    for s in lb:
        if s["id"] not in used:
            rb[s["clause"]].append(s)
    cands = []
    for ca, (cb, _) in pairs_c.items():
        for x in ra.get(ca, ()):
            for y in rb.get(cb, ()):
                sm = difflib.SequenceMatcher(None, norm_ws(x["text"]), norm_ws(y["text"]),
                                             autojunk=False)
                if sm.real_quick_ratio() < SIMILAR_RATIO or sm.quick_ratio() < SIMILAR_RATIO:
                    continue
                r = sm.ratio()
                if r >= SIMILAR_RATIO:
                    cands.append((-r, natural_key(x["id"]), natural_key(y["id"]), x["id"],
                                  y["id"]))
    cands.sort()
    for _, _, _, xa, yb in cands:
        if xa in pairs or yb in used:
            continue
        pairs[xa] = (yb, "similar")
        used.add(yb)
    return pairs, used


def diff_statements(a, b, sa, sb):
    for st, g, what in ((sa, a, "first"), (sb, b, "second")):
        validate_statements(st, "%s statements" % what)
        if st["graph"]["source_sha256"] != g["source"]["sha256"]:
            raise UsageError("%s statements file is stale for %s-%s; rerun `statements`"
                             % (what, g["spec"], g["version"]))
    pairs, used = match_statements(a, b, sa, sb)
    ka = {s["id"]: s for s in sa["statements"]}
    kb = {s["id"]: s for s in sb["statements"]}
    moved, changed, unchanged = [], [], 0
    for aid in sorted(pairs, key=natural_key):
        bid, how = pairs[aid]
        same_id, same_hash = aid == bid, ka[aid]["hash"] == kb[bid]["hash"]
        if same_id and same_hash:
            unchanged += 1
        if not same_id:
            moved.append([aid, bid, how])
        if not same_hash:
            changed.append([aid, bid, how])
    added = sorted((x for x in kb if x not in used), key=natural_key)
    removed = sorted((x for x in ka if x not in pairs), key=natural_key)
    kc = lambda ids, k: dict(sorted(collections.Counter(k[x]["kind"] for x in ids).items()))  # noqa
    summary = {"statements_a": len(ka), "statements_b": len(kb), "matched": len(pairs),
               "matched_by": dict(sorted(collections.Counter(h for _, h in pairs.values())
                                         .items())),
               "unchanged": unchanged, "moved": len(moved), "changed": len(changed),
               "moved_only": sum(1 for x in moved if ka[x[0]]["hash"] == kb[x[1]]["hash"]),
               "added": len(added), "removed": len(removed),
               "added_by_kind": kc(added, kb), "removed_by_kind": kc(removed, ka),
               "changed_by_kind": kc([x[1] for x in changed], kb)}
    return {"summary": summary, "added": added, "removed": removed, "moved": moved,
            "changed": changed}


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


def merged_from(args, ids=()):
    graphs = [load_graph(args.graph)] + [load_graph(p) for p in (args.with_ or [])]
    sts = None
    if getattr(args, "statements", False) or any(RE_STMT_ID.match(x) for x in ids):
        sts = [load_statements(args.graph)]
        sts += [x for x in (load_statements(p, required=False) for p in (args.with_ or []))
                if x is not None]
    return Merged(graphs, sts)


def cmd_statements(args):
    g = load_graph(args.graph)
    xpath = args.xspec
    if g.get("xspec") and not xpath:
        xpath = os.path.join(os.path.dirname(os.path.abspath(args.graph)),
                             "%s-%s.json" % (g["xspec"]["spec"], g["xspec"]["version"]))
    xg = load_graph(xpath) if xpath else None
    st = build_statements(g, xg)
    out_dir = args.out_dir or os.path.dirname(os.path.abspath(args.graph))
    os.makedirs(out_dir, exist_ok=True)
    stem = os.path.join(out_dir, "%s-%s.statements" % (g["spec"], g["version"]))
    write_text(stem + ".json", json.dumps(st, indent=1, ensure_ascii=False) + "\n")
    write_text(stem + ".md", statements_markdown(st, g) + "\n")
    t = st["stats"]
    print("%s-%s: %d statements (%d top-level, %d sub) in %d clauses; %s; %d declared "
          "markers (%d edges, %d inferred edges superseded), %d unresolved"
          % (g["spec"], g["version"], t["statements"], t["top_level"], t["sub_statements"],
             t["clauses"], json.dumps(t["by_kind"]), t["markers"], t["declared_edges"],
             t["superseded_edges"], t["unresolved_markers"]))
    for u in st["unresolved"]:
        print("warning: %s: unresolved declared dependency %r (%s)"
              % (u["src"], u["ref"], u["reason"]), file=sys.stderr)
    print("wrote %s.json and %s.md" % (stem, stem))
    return 0


def describe(m, nid):
    nd = m.nodes.get(nid, {})
    extra = ""
    if nd.get("aliases"):
        extra = " [%s]" % ", ".join(nd["aliases"])
    if nd.get("kind") == "section":
        extra = " (§%s %s)" % (nd["number"], nd["title"])
    if nd.get("kind") == "statement":
        extra = " [%s]" % KIND_ABBR[nd["stmt_kind"]]
    return nid + extra


def cmd_impact(args):
    m = merged_from(args, [args.id])
    nid = m.resolve(args.id)
    levels = m.impact(nid, args.closure, args.max_depth)
    total = sum(len(x) for x in levels)
    if args.json:
        print(json.dumps({"id": nid, "closure": args.closure, "max_depth": args.max_depth,
                          "total": total, "levels": levels}, indent=1))
        return 0
    what = "dependent clauses"
    if m.statements:
        flat = [x for lv in levels for x in lv]
        nst = sum(1 for x in flat if m.kind(x) == "statement")
        ncl = len({RE_STMT_ID.match(x).group("clause") if m.kind(x) == "statement" else x
                   for x in flat})
        what = "dependents (%d statements, %d clauses without a statement layer; %d clauses " \
               "in all)" % (nst, total - nst, ncl)
    print("impact of %s (closure %s%s): %d %s"
          % (describe(m, nid), args.closure,
             ", max depth %d" % args.max_depth if args.max_depth else "", total, what))
    for d, lv in enumerate(levels, 1):
        print("depth %d (%d): %s" % (d, len(lv), ", ".join(describe(m, x) for x in lv)))
    return 0


def cmd_independent(args):
    m = merged_from(args, [args.a, args.b])
    a, b = m.resolve(args.a), m.resolve(args.b)
    ok, fwd, rev = m.independent(a, b, args.closure, args.max_depth, args.containers)
    if ok:
        print("independent")
        return 0
    print("dependent")
    print("shared forward cone (%d): %s" % (len(fwd), ", ".join(fwd)))
    print("shared reverse cone (%d): %s" % (len(rev), ", ".join(rev)))
    return 1


def cmd_coverage(args):
    g = load_graph(args.graph)
    tags = load_json(args.tags)
    want = args.statements or (isinstance(tags, dict) and any(
        RE_STMT_ID.match(t) for v in tags.values() for t in ([v] if isinstance(v, str) else
                                                              v if isinstance(v, list) else [])
        if isinstance(t, str)))
    rep = coverage(g, tags, load_statements(args.graph) if want else None)
    if args.json:
        print(json.dumps(rep, indent=1))
        return 0
    t = rep["totals"]
    print("coverage of %s-%s: %d clauses; strong %d, weak (section tag only) %d, uncovered %d"
          % (rep["spec"], rep["version"], t["clauses"], t["strong"], t["weak"], t["uncovered"]))
    if rep["foreign"]:
        print("ignored tags for other specs: %s" % ", ".join(rep["foreign"]))
    if "statements" in rep:
        st = rep["statements"]
        t = st["totals"]
        print("statements of %s-%s: %d; strong %d, weak (clause or section tag) %d, "
              "uncovered %d" % (rep["spec"], rep["version"], t["statements"], t["strong"],
                                t["weak"], t["uncovered"]))
        print("by kind: " + "; ".join("%s %d (strong %d, weak %d, uncovered %d)"
                                      % (k, c["statements"], c["strong"], c["weak"],
                                         c["uncovered"]) for k, c in st["by_kind"].items()))
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
    ga, gb = load_graph(args.a), load_graph(args.b)
    d = diff_graphs(ga, gb)
    if args.statements:
        d["statements"] = diff_statements(ga, gb, load_statements(args.a),
                                          load_statements(args.b))
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
    if "statements" in d:
        sd, t = d["statements"], d["statements"]["summary"]
        print()
        print("statements %s -> %s: %d -> %d; matched %d %s; unchanged %d; moved %d "
              "(renumbered only %d); changed %d; added %d; removed %d"
              % (s_["a"], s_["b"], t["statements_a"], t["statements_b"], t["matched"],
                 json.dumps(t["matched_by"]), t["unchanged"], t["moved"], t["moved_only"],
                 t["changed"], t["added"], t["removed"]))
        print("by kind: added %s; removed %s; changed %s"
              % (json.dumps(t["added_by_kind"]), json.dumps(t["removed_by_kind"]),
                 json.dumps(t["changed_by_kind"])))
        if args.list:
            for k in ("added", "removed"):
                print("statements %s: %s" % (k, ", ".join(sd[k])))
            for k in ("moved", "changed"):
                print("statements %s: %s" % (k, ", ".join("%s->%s(%s)" % tuple(x)
                                                          for x in sd[k])))
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
    d.add_argument("--statements", action="store_true",
                   help="also diff the statement layers (<graph>.statements.json siblings)")
    d.set_defaults(fn=cmd_diff)
    s = sub.add_parser("statements", help="split clauses into Lamport-style statements; write "
                                          "<SPEC>-<version>.statements.json/.md")
    s.add_argument("graph")
    s.add_argument("--xspec", help="graph the input was built against (default: sibling file)")
    s.add_argument("--out-dir", default=None, help="default: the graph's directory")
    s.set_defaults(fn=cmd_statements)
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
            q.add_argument("--containers", choices=("count", "ignore"), default="count",
                           help="count (default): a citation of a section or clause enclosing "
                                "both ids makes them dependent; ignore: it does not")
        q.add_argument("--with", dest="with_", action="append",
                       help="also load this graph (e.g. the ISA graph for a DLT query)")
        q.add_argument("--statements", action="store_true",
                       help="query the statement layer (implied by a statement id)")
        add_closure_args(q)
        q.set_defaults(fn=fn)
    c = sub.add_parser("coverage", help="clauses with zero witnesses per section")
    c.add_argument("graph")
    c.add_argument("--tags", required=True, help="JSON {test-or-proof-name: id | [ids]}")
    c.add_argument("--json", action="store_true")
    c.add_argument("--statements", action="store_true",
                   help="also report statement-level coverage (implied by a statement id tag)")
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
