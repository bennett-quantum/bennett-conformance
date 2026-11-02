"""RFC 8785 (JCS) canonical JSON, subset needed for §9.2/§12.3 hashing.

Keys sort by UTF-16 code units; strings use minimal JSON escapes; integers
render plainly. Floats use Python's shortest round-trip digits laid out per
ECMAScript Number-to-String (positional in [1e-6, 1e21)); non-finite values are
rejected.
"""
import math


def _escape(s):
    out = ['"']
    for ch in s:
        o = ord(ch)
        if ch == '"':
            out.append('\\"')
        elif ch == '\\':
            out.append('\\\\')
        elif ch == '\b':
            out.append('\\b')
        elif ch == '\f':
            out.append('\\f')
        elif ch == '\n':
            out.append('\\n')
        elif ch == '\r':
            out.append('\\r')
        elif ch == '\t':
            out.append('\\t')
        elif o < 0x20:
            out.append('\\u%04x' % o)
        else:
            out.append(ch)
    out.append('"')
    return ''.join(out)


def _number(x):
    if isinstance(x, float):
        if not math.isfinite(x):
            raise ValueError("non-finite number in JCS input")
        if x == 0:
            return "0"
        if x == int(x) and abs(x) < 1e21:
            return str(int(x))
        r = repr(x)
        if 'e' in r and 1e-6 <= abs(x) < 1e21:
            from decimal import Decimal
            return format(Decimal(r), 'f')
        if 'e' in r:
            mant, exp = r.split('e')
            r = mant + 'e' + ('+' if int(exp) >= 0 else '-') + str(abs(int(exp)))
        return r
    return str(x)


def canonical(obj):
    if obj is None:
        s = "null"
    elif obj is True:
        s = "true"
    elif obj is False:
        s = "false"
    elif isinstance(obj, str):
        s = _escape(obj)
    elif isinstance(obj, (int, float)):
        s = _number(obj)
    elif isinstance(obj, list):
        s = "[" + ",".join(canonical(v).decode() for v in obj) + "]"
    elif isinstance(obj, dict):
        keys = sorted(obj.keys(), key=lambda k: k.encode('utf-16-be', 'surrogatepass'))
        s = "{" + ",".join(_escape(k) + ":" + canonical(obj[k]).decode() for k in keys) + "}"
    else:
        raise TypeError(f"not JSON: {type(obj)}")
    return s.encode()
