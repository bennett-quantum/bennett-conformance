"""Parse and re-emit Bennett object text (ISA §16). Byte-exact round trip; malformed input is loud.

This is structure only (sections, function boundaries): executors validate semantics, the harness
needs enough shape to derive invocations, shrink witnesses and recompute §9.2 identity.
"""
import json
import re

SECTIONS = ("symbols", "imports", "functions", "data", "devices", "layout", "schedule", "certificate", "coverage")
JSON_SECTIONS = {"symbols", "imports", "devices", "layout", "schedule", "certificate", "coverage"}
FN_HEAD = re.compile(r"^(fn (cc|r)|oracle|action|perm_action|lproc) @([A-Za-z_][A-Za-z0-9_.]*) ")
HEADER = re.compile(r"^bennett-object isa=(\S+) object=\S+ launch=\S+ sched=\S+ cert=\S+ coverage=\S+ target=\S+ ptr_width=(-?\d+)$")


class ObjectError(ValueError):
    pass


def jcs(value):
    """RFC 8785 for the value domain objects use (integers, strings, arrays, objects)."""
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False)


class Function:
    def __init__(self, lines):
        m = FN_HEAD.match(lines[0])
        if not m:
            raise ObjectError("not a function header: %r" % lines[0])
        self.kind = m.group(2) or m.group(1)
        self.name = m.group(3)
        self.lines = list(lines)

    def canonical(self):
        return "\n".join(self.lines)

    def copy(self):
        return Function(self.lines)


class ObjectFile:
    def __init__(self, header, isa, ptr_width, order, sections, functions, data):
        self.header = header
        self.isa = isa
        self.ptr_width = ptr_width
        self.order = order        # section names in file order
        self.sections = sections  # name -> parsed JSON for JSON sections
        self.raw = {}             # name -> raw JSON line, kept so foreign bytes round-trip exactly
        self.functions = functions
        self.data = data          # list of (name, hex) in file order

    def symbols(self):
        return self.sections.get("symbols", {})

    def symbol_table(self):
        return {s["name"]: s for s in self.symbols().values()}

    def layout(self):
        return self.sections.get("layout")

    def function(self, name):
        for f in self.functions:
            if f.name == name:
                return f
        raise ObjectError("no function @%s" % name)

    def text(self):
        out = [self.header, ""]
        first = True
        for name in self.order:
            if not first:
                out.append("")
            first = False
            out.append("section " + name)
            if name == "functions":
                for f in self.functions:
                    out.extend(f.lines)
            elif name == "data":
                out.extend("%s %s" % d for d in self.data)
            else:
                line = self.raw.get(name)
                out.append(line if line is not None else jcs(self.sections[name]))
        return "\n".join(out) + "\n"

    def set_section(self, name, value):
        """Replace a JSON section (drops the raw bytes; the value is re-serialised as JCS)."""
        if name not in JSON_SECTIONS or name not in self.order:
            raise ObjectError("cannot set section %s" % name)
        self.sections[name] = value
        self.raw.pop(name, None)

    def copy(self):
        return parse(self.text())


def parse(text):
    if not text.endswith("\n"):
        raise ObjectError("object does not end in LF")
    lines = text[:-1].split("\n")
    m = HEADER.match(lines[0]) if lines else None
    if not m:
        raise ObjectError("bad header: %r" % (lines[0] if lines else ""))
    header, isa, ptr_width = lines[0], m.group(1), int(m.group(2))
    order, sections, functions, data, raw = [], {}, [], [], {}
    i, n = 1, len(lines)
    while i < n:
        if lines[i] != "" or i + 1 >= n or not lines[i + 1].startswith("section "):
            raise ObjectError("line %d: expected a blank line then a section, got %r" % (i + 1, lines[i]))
        name = lines[i + 1][len("section "):]
        if name not in SECTIONS:
            raise ObjectError("unknown section %r" % name)
        if name in order:
            raise ObjectError("repeated section %r" % name)
        order.append(name)
        i += 2
        start = i
        while i < n and not (lines[i] == "" and i + 1 < n and lines[i + 1].startswith("section ")):
            i += 1
        body = lines[start:i]
        if name == "functions":
            cur = []
            for line in body:
                if FN_HEAD.match(line) and cur:
                    functions.append(Function(cur))
                    cur = []
                cur.append(line)
            if cur:
                functions.append(Function(cur))
        elif name == "data":
            for line in body:
                parts = line.split(" ")
                if len(parts) != 2:
                    raise ObjectError("bad data line %r" % line)
                data.append((parts[0], parts[1]))
        else:
            if len(body) != 1:
                raise ObjectError("section %s must be one JSON line" % name)
            try:
                sections[name] = json.loads(body[0])
            except ValueError as e:
                raise ObjectError("section %s: %s" % (name, e))
            raw[name] = body[0]
    o = ObjectFile(header, isa, ptr_width, order, sections, functions, data)
    o.raw = raw
    return o
