//! Strict JSON parser (number lexemes preserved, duplicate keys rejected)
//! plus RFC 8785 (JCS) canonical serialisation. Written from RFC 8259/8785.

use std::collections::BTreeMap;
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    /// The exact input lexeme (validated against the JSON number grammar).
    Num(String),
    Str(String),
    Arr(Vec<Value>),
    /// Insertion order is not retained; lookups are by key. JCS re-sorts.
    Obj(BTreeMap<String, Value>),
}

impl Value {
    pub fn get(&self, key: &str) -> Option<&Value> {
        match self {
            Value::Obj(m) => m.get(key),
            _ => None,
        }
    }
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::Str(s) => Some(s),
            _ => None,
        }
    }
    pub fn as_arr(&self) -> Option<&[Value]> {
        match self {
            Value::Arr(a) => Some(a),
            _ => None,
        }
    }
    pub fn as_obj(&self) -> Option<&BTreeMap<String, Value>> {
        match self {
            Value::Obj(m) => Some(m),
            _ => None,
        }
    }
    /// Exact integer value of a JSON number lexeme, if it denotes an integer
    /// that fits the checked range. Rejects fraction/exponent spellings that
    /// are not integral and values outside i128.
    pub fn as_i128(&self) -> Option<i128> {
        match self {
            Value::Num(lex) => lex.parse::<i128>().ok(),
            _ => None,
        }
    }
    pub fn kind(&self) -> &'static str {
        match self {
            Value::Null => "null",
            Value::Bool(_) => "boolean",
            Value::Num(_) => "number",
            Value::Str(_) => "string",
            Value::Arr(_) => "array",
            Value::Obj(_) => "object",
        }
    }
}

#[derive(Debug)]
pub struct ParseError(pub String);
impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl std::error::Error for ParseError {}

const MAX_DEPTH: usize = 512;

pub fn parse(text: &str) -> Result<Value, ParseError> {
    let bytes = text.as_bytes();
    let mut p = Parser { b: bytes, i: 0 };
    p.ws();
    let v = p.value(0)?;
    p.ws();
    if p.i != bytes.len() {
        return Err(p.err("trailing characters after JSON value"));
    }
    Ok(v)
}

struct Parser<'a> {
    b: &'a [u8],
    i: usize,
}

impl<'a> Parser<'a> {
    fn err(&self, msg: &str) -> ParseError {
        ParseError(format!("JSON parse error at byte {}: {msg}", self.i))
    }
    fn ws(&mut self) {
        while self.i < self.b.len() && matches!(self.b[self.i], b' ' | b'\t' | b'\n' | b'\r') {
            self.i += 1;
        }
    }
    fn peek(&self) -> Option<u8> {
        self.b.get(self.i).copied()
    }
    fn value(&mut self, depth: usize) -> Result<Value, ParseError> {
        if depth > MAX_DEPTH {
            return Err(self.err("nesting deeper than 512"));
        }
        match self.peek() {
            None => Err(self.err("unexpected end of input")),
            Some(b'n') => self.lit("null", Value::Null),
            Some(b't') => self.lit("true", Value::Bool(true)),
            Some(b'f') => self.lit("false", Value::Bool(false)),
            Some(b'"') => Ok(Value::Str(self.string()?)),
            Some(b'[') => {
                self.i += 1;
                let mut out = Vec::new();
                self.ws();
                if self.peek() == Some(b']') {
                    self.i += 1;
                    return Ok(Value::Arr(out));
                }
                loop {
                    self.ws();
                    out.push(self.value(depth + 1)?);
                    self.ws();
                    match self.peek() {
                        Some(b',') => self.i += 1,
                        Some(b']') => {
                            self.i += 1;
                            return Ok(Value::Arr(out));
                        }
                        _ => return Err(self.err("expected ',' or ']' in array")),
                    }
                }
            }
            Some(b'{') => {
                self.i += 1;
                let mut out = BTreeMap::new();
                self.ws();
                if self.peek() == Some(b'}') {
                    self.i += 1;
                    return Ok(Value::Obj(out));
                }
                loop {
                    self.ws();
                    if self.peek() != Some(b'"') {
                        return Err(self.err("expected object key string"));
                    }
                    let key = self.string()?;
                    self.ws();
                    if self.peek() != Some(b':') {
                        return Err(self.err("expected ':' after object key"));
                    }
                    self.i += 1;
                    self.ws();
                    let val = self.value(depth + 1)?;
                    if out.insert(key.clone(), val).is_some() {
                        return Err(self.err(&format!("duplicate object key {key:?}")));
                    }
                    self.ws();
                    match self.peek() {
                        Some(b',') => self.i += 1,
                        Some(b'}') => {
                            self.i += 1;
                            return Ok(Value::Obj(out));
                        }
                        _ => return Err(self.err("expected ',' or '}' in object")),
                    }
                }
            }
            Some(b'-') | Some(b'0'..=b'9') => self.number(),
            Some(c) => Err(self.err(&format!("unexpected character {:?}", c as char))),
        }
    }
    fn lit(&mut self, word: &str, v: Value) -> Result<Value, ParseError> {
        if self.b[self.i..].starts_with(word.as_bytes()) {
            self.i += word.len();
            Ok(v)
        } else {
            Err(self.err(&format!("expected {word}")))
        }
    }
    fn number(&mut self) -> Result<Value, ParseError> {
        let start = self.i;
        if self.peek() == Some(b'-') {
            self.i += 1;
        }
        match self.peek() {
            Some(b'0') => self.i += 1,
            Some(b'1'..=b'9') => {
                while matches!(self.peek(), Some(b'0'..=b'9')) {
                    self.i += 1;
                }
            }
            _ => return Err(self.err("invalid number")),
        }
        if self.peek() == Some(b'.') {
            self.i += 1;
            if !matches!(self.peek(), Some(b'0'..=b'9')) {
                return Err(self.err("invalid fraction"));
            }
            while matches!(self.peek(), Some(b'0'..=b'9')) {
                self.i += 1;
            }
        }
        if matches!(self.peek(), Some(b'e') | Some(b'E')) {
            self.i += 1;
            if matches!(self.peek(), Some(b'+') | Some(b'-')) {
                self.i += 1;
            }
            if !matches!(self.peek(), Some(b'0'..=b'9')) {
                return Err(self.err("invalid exponent"));
            }
            while matches!(self.peek(), Some(b'0'..=b'9')) {
                self.i += 1;
            }
        }
        Ok(Value::Num(
            std::str::from_utf8(&self.b[start..self.i]).unwrap().to_string(),
        ))
    }
    fn string(&mut self) -> Result<String, ParseError> {
        debug_assert_eq!(self.peek(), Some(b'"'));
        self.i += 1;
        let mut out = String::new();
        loop {
            match self.peek() {
                None => return Err(self.err("unterminated string")),
                Some(b'"') => {
                    self.i += 1;
                    return Ok(out);
                }
                Some(b'\\') => {
                    self.i += 1;
                    match self.peek() {
                        Some(b'"') => out.push('"'),
                        Some(b'\\') => out.push('\\'),
                        Some(b'/') => out.push('/'),
                        Some(b'b') => out.push('\u{08}'),
                        Some(b'f') => out.push('\u{0c}'),
                        Some(b'n') => out.push('\n'),
                        Some(b'r') => out.push('\r'),
                        Some(b't') => out.push('\t'),
                        Some(b'u') => {
                            self.i += 1;
                            let hi = self.hex4()?;
                            if (0xd800..0xdc00).contains(&hi) {
                                if self.peek() == Some(b'\\') && self.b.get(self.i + 1) == Some(&b'u') {
                                    self.i += 2;
                                    let lo = self.hex4()?;
                                    if !(0xdc00..0xe000).contains(&lo) {
                                        return Err(self.err("invalid low surrogate"));
                                    }
                                    let cp = 0x10000 + ((hi - 0xd800) << 10) + (lo - 0xdc00);
                                    out.push(char::from_u32(cp).ok_or_else(|| self.err("bad surrogate pair"))?);
                                    continue;
                                }
                                return Err(self.err("lone high surrogate"));
                            }
                            if (0xdc00..0xe000).contains(&hi) {
                                return Err(self.err("lone low surrogate"));
                            }
                            out.push(char::from_u32(hi).ok_or_else(|| self.err("bad \\u escape"))?);
                            continue;
                        }
                        _ => return Err(self.err("invalid escape")),
                    }
                    self.i += 1;
                }
                Some(c) if c < 0x20 => return Err(self.err("unescaped control character in string")),
                Some(_) => {
                    // Copy one UTF-8 char.
                    let rest = std::str::from_utf8(&self.b[self.i..])
                        .map_err(|_| self.err("invalid UTF-8 in string"))?;
                    let ch = rest.chars().next().unwrap();
                    out.push(ch);
                    self.i += ch.len_utf8();
                }
            }
        }
    }
    fn hex4(&mut self) -> Result<u32, ParseError> {
        if self.i + 4 > self.b.len() {
            return Err(self.err("truncated \\u escape"));
        }
        let s = std::str::from_utf8(&self.b[self.i..self.i + 4]).map_err(|_| self.err("bad \\u escape"))?;
        let v = u32::from_str_radix(s, 16).map_err(|_| self.err("bad \\u escape"))?;
        self.i += 4;
        Ok(v)
    }
}

/// RFC 8785 canonical serialisation. Fails on non-finite numbers.
pub fn jcs(v: &Value) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    write_jcs(v, &mut out)?;
    Ok(out)
}

fn write_jcs(v: &Value, out: &mut Vec<u8>) -> Result<(), String> {
    match v {
        Value::Null => out.extend_from_slice(b"null"),
        Value::Bool(true) => out.extend_from_slice(b"true"),
        Value::Bool(false) => out.extend_from_slice(b"false"),
        Value::Num(lex) => {
            let s = jcs_number(lex)?;
            out.extend_from_slice(s.as_bytes());
        }
        Value::Str(s) => write_jcs_string(s, out),
        Value::Arr(items) => {
            out.push(b'[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(b',');
                }
                write_jcs(item, out)?;
            }
            out.push(b']');
        }
        Value::Obj(map) => {
            // RFC 8785: sort keys by their UTF-16 code unit sequences.
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort_by(|a, b| utf16_units(a).cmp(&utf16_units(b)));
            out.push(b'{');
            for (i, k) in keys.iter().enumerate() {
                if i > 0 {
                    out.push(b',');
                }
                write_jcs_string(k, out);
                out.push(b':');
                write_jcs(&map[*k], out)?;
            }
            out.push(b'}');
        }
    }
    Ok(())
}

fn utf16_units(s: &str) -> Vec<u16> {
    s.encode_utf16().collect()
}

fn write_jcs_string(s: &str, out: &mut Vec<u8>) {
    out.push(b'"');
    for ch in s.chars() {
        match ch {
            '"' => out.extend_from_slice(b"\\\""),
            '\\' => out.extend_from_slice(b"\\\\"),
            '\u{08}' => out.extend_from_slice(b"\\b"),
            '\u{09}' => out.extend_from_slice(b"\\t"),
            '\u{0a}' => out.extend_from_slice(b"\\n"),
            '\u{0c}' => out.extend_from_slice(b"\\f"),
            '\u{0d}' => out.extend_from_slice(b"\\r"),
            c if (c as u32) < 0x20 => {
                out.extend_from_slice(format!("\\u{:04x}", c as u32).as_bytes());
            }
            c => {
                let mut buf = [0u8; 4];
                out.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
            }
        }
    }
    out.push(b'"');
}

/// ECMAScript Number-to-String as RFC 8785 section 3.2.2.3 requires,
/// computed from the shortest round-trip digits Rust produces.
pub fn jcs_number(lex: &str) -> Result<String, String> {
    let v: f64 = lex.parse().map_err(|_| format!("invalid JSON number {lex:?}"))?;
    if !v.is_finite() {
        return Err(format!("non-finite JSON number {lex:?} has no JCS form"));
    }
    Ok(ecmascript_number(v))
}

pub fn ecmascript_number(v: f64) -> String {
    if v == 0.0 {
        return "0".to_string();
    }
    let sign = if v < 0.0 { "-" } else { "" };
    let a = v.abs();
    // Rust LowerExp gives shortest digits like "1.2345e20" or "5e-1".
    let sci = format!("{a:e}");
    let (digits, exp) = parse_sci(&sci);
    // ECMAScript: value = digits (as integer s) * 10^(n-k), k = len(digits).
    let k = digits.len() as i32;
    let n = exp + 1; // decimal point position relative to digit string
    let body = if k <= n && n <= 21 {
        let mut s = digits.clone();
        s.push_str(&"0".repeat((n - k) as usize));
        s
    } else if 0 < n && n <= 21 {
        let mut s = String::new();
        s.push_str(&digits[..n as usize]);
        s.push('.');
        s.push_str(&digits[n as usize..]);
        s
    } else if -6 < n && n <= 0 {
        let mut s = "0.".to_string();
        s.push_str(&"0".repeat((-n) as usize));
        s.push_str(&digits);
        s
    } else {
        let mut s = String::new();
        s.push_str(&digits[..1]);
        if k > 1 {
            s.push('.');
            s.push_str(&digits[1..]);
        }
        s.push('e');
        let e = n - 1;
        if e >= 0 {
            s.push('+');
        }
        s.push_str(&e.to_string());
        s
    };
    format!("{sign}{body}")
}

fn parse_sci(sci: &str) -> (String, i32) {
    let (mant, exp) = sci.split_once('e').expect("LowerExp always has e");
    let exp: i32 = exp.parse().unwrap();
    let digits: String = mant.chars().filter(|c| c.is_ascii_digit()).collect();
    // Strip trailing zeros cannot occur in shortest form except "0" mantissa path (excluded).
    (digits, exp)
}

/// Build a string value.
pub fn s(v: &str) -> Value {
    Value::Str(v.to_string())
}
/// Build an integer number value from a canonical lexeme.
pub fn n(v: impl ToString) -> Value {
    Value::Num(v.to_string())
}
