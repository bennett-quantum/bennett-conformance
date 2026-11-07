//! ISA §16.1/§16.2 token spelling and punctuation, before source hashing.
use super::sec_err;
type Lexical = Result<(), &'static str>;

#[derive(Clone, Copy, PartialEq)]
enum Form {
    Header,
    Declaration,
    Registers,
    Bounds,
    Instruction,
}

#[derive(Clone, Copy)]
enum Atom {
    Word,
    Numeric,
    String,
    Punctuation,
    Star,
    Header,
}

struct Token<'a> {
    atom: Atom,
    text: &'a str,
    start: usize,
    end: usize,
}

pub(super) fn validate(lines: &[&str]) -> Result<(), String> {
    let mut section = "";
    for (i, line) in lines.iter().enumerate() {
        let result = match *line {
            "" => Ok(()),
            _ if line.contains('\r') => Err("expected LF line endings"),
            _ if line.starts_with("section ") => {
                section = &line["section ".len()..];
                validate_line(line)
            }
            _ => match section {
                "symbols" | "imports" | "devices" | "layout" | "schedule" | "certificate"
                | "coverage" => Ok(()),
                "data" => validate_data(line),
                _ => validate_line(line),
            },
        };
        result.map_err(|e| sec_err("§16.1", format!("line {}: {e}", i + 1)))?;
    }
    Ok(())
}

fn name(atom: &str) -> bool {
    let mut chars = atom.bytes();
    matches!(chars.next(), Some(b'a'..=b'z' | b'A'..=b'Z' | b'_'))
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, b'_' | b'.'))
}

fn unsigned(atom: &str) -> Lexical {
    match crate::rat::is_canonical_uint_str(atom) {
        true => Ok(()),
        false => Err("expected a canonical unsigned INT"),
    }
}

fn validate_data(line: &str) -> Lexical {
    let (label, hex) = line.split_once(' ').ok_or("data requires NAME SP HEX")?;
    let canonical = name(label)
        && hex.len().is_multiple_of(2)
        && hex.bytes().all(|c| b"0123456789abcdef".contains(&c));
    canonical
        .then_some(())
        .ok_or("data requires NAME and lowercase even-length HEX")
}

fn quoted_end(line: &str, start: usize) -> Result<usize, &'static str> {
    let mut i = start + 1;
    loop {
        match line.as_bytes().get(i) {
            None => return Err("unterminated STRING"),
            Some(b'"') => return Ok(i + 1),
            Some(b'\\') => {
                match line.as_bytes().get(i + 1) {
                    Some(b'"' | b'\\' | b'n') => (),
                    _ => return Err("STRING escapes are quote, backslash and n"),
                }
                i += 2;
            }
            Some(_) => i += 1,
        }
    }
}

fn tokenize(line: &str) -> Result<Vec<Token<'_>>, &'static str> {
    let mut tokens = vec![];
    let mut i = 0;
    while i < line.len() {
        let start = i;
        match line.as_bytes()[i] {
            b' ' => {
                i += 1;
                continue;
            }
            b'"' => i = quoted_end(line, i)?,
            c if b"()[]{},:=@^".contains(&c) => i += 1,
            _ => {
                i = line.as_bytes()[i..]
                    .iter()
                    .position(|c| b" ()[]{},:=@^\"".contains(c))
                    .map_or(line.len(), |n| i + n);
            }
        }
        tokens.push(Token {
            atom: atom_kind(&line[start..i])?,
            text: &line[start..i],
            start,
            end: i,
        });
    }
    Ok(tokens)
}

fn form(tokens: &[Token<'_>]) -> Form {
    match tokens[0].text {
        "bennett-object" => Form::Header,
        "regs" => Form::Registers,
        "fn" | "oracle" | "action" | "perm_action" | "lproc" | "section" | "^" | "}" => {
            Form::Declaration
        }
        _ => Form::Instruction,
    }
}

fn gap(tokens: &[Token<'_>], i: usize, mode: Form, brackets: usize) -> &'static str {
    let branch_group = tokens[..i - 1].last().is_some_and(|t| t.text == "^");
    match (
        mode,
        tokens[i - 1].text,
        tokens[i].text,
        brackets > 0,
        branch_group,
    ) {
        (Form::Registers, "{", "}", _, _) => "  ",
        (Form::Registers, "{", _, _, _) | (Form::Registers, _, "}", _, _) => " ",
        (_, "{", _, _, _) | (_, _, "}", _, _) => "",
        (_, "@" | "^" | ":" | "(" | "[", _, _, _) | (_, _, ":" | ")" | "[" | "]" | ",", _, _) => "",
        (_, ",", _, true, _) => "",
        (Form::Header, "=", _, _, _) | (Form::Header, _, "=", _, _) => "",
        (_, _, "(", _, true) => "",
        _ => " ",
    }
}

fn validate_type(tokens: &[Token<'_>]) -> Lexical {
    // §16.2 region[INT,INT] is the longest type: six tokens plus its boundary.
    const MAX_TYPE_TOKENS: usize = 6;
    let words: Vec<_> = tokens
        .iter()
        .take(MAX_TYPE_TOKENS + 1)
        .map(|t| t.text)
        .collect();
    let end = match words.as_slice() {
        ["region", "[", a, ",", w, "]", ..] => {
            unsigned(a)?;
            unsigned(w)?;
            6
        }
        ["qreg", "[", "*", "]", ..] => 4,
        ["wire" | "reg" | "qreg", "[", n, "]", ..] => {
            unsigned(n)?;
            4
        }
        ["wire" | "ptr" | "f64" | "token", ..] => 1,
        [word, ..] if word.starts_with('i') => {
            unsigned(&word[1..])?;
            1
        }
        _ => return Err("type does not match the §16.2 type grammar"),
    };
    match words.get(end) {
        None | Some(&"," | &")" | &"}" | &"in" | &"mut" | &"token" | &"classical") => Ok(()),
        _ => Err("unexpected token after type"),
    }
}

pub(super) fn float(atom: &str) -> Lexical {
    let value = atom
        .parse::<f64>()
        .ok()
        .filter(|v| v.is_finite())
        .ok_or("invalid or nonfinite FLOAT")?;
    let rendered = crate::json::ecmascript_number(value);
    let expected = match (
        value == 0.0,
        value.is_sign_negative(),
        rendered.contains(['.', 'e']),
    ) {
        (true, true, _) => "-0.0".into(),
        (_, _, false) => format!("{rendered}.0"),
        _ => rendered,
    };
    match atom == expected {
        true => Ok(()),
        false => Err("noncanonical FLOAT"),
    }
}

fn opcode(tokens: &[Token<'_>]) -> usize {
    let mut i = tokens
        .iter()
        .position(|t| t.text == "=")
        .map_or(0, |i| i + 1);
    loop {
        match tokens.get(i).map(|t| t.text) {
            Some("adj") => i += 1,
            Some("ctl") => {
                i += 2;
                if tokens.get(i).is_some_and(|t| t.text == "[") {
                    i += 3;
                }
            }
            _ => return i,
        }
    }
}

fn opcode_widths(word: &str) -> Lexical {
    word.split('.')
        .try_for_each(|part| match (name(part), part.strip_prefix('i')) {
            (false, _) => Err("noncanonical dotted opcode"),
            (true, Some(width)) if !width.starts_with(|c: char| c.is_ascii_alphabetic()) => {
                unsigned(width)
            }
            _ => Ok(()),
        })
}

fn numeric(atom: &str, mode: Form, brackets: usize, angle: bool) -> Lexical {
    match (mode, brackets, angle) {
        (Form::Bounds, _, _) | (_, 1.., _) => unsigned(atom),
        (_, _, true) => float(atom),
        _ => unsigned(atom.strip_prefix('-').unwrap_or(atom)),
    }
}

#[derive(Clone, Copy)]
enum Number {
    Integer,
    Unsigned,
    Angle,
}

fn number_kind(op: &str) -> Number {
    match op {
        "rx" | "ry" | "rz" => Number::Angle,
        "qloop" | "qalloc" | "qborrow" | "qregion" | "qreg.new" | "qreg.slice" | "fault" => {
            Number::Unsigned
        }
        _ => Number::Integer,
    }
}

fn atom_kind(word: &str) -> Result<Atom, &'static str> {
    match word.as_bytes()[0] {
        b'"' => Ok(Atom::String),
        b'*' if word == "*" => Ok(Atom::Star),
        b'0'..=b'9' | b'+' | b'-' | b'.' => Ok(Atom::Numeric),
        c if word.len() == 1 && b"()[]{},:=@^".contains(&c) => Ok(Atom::Punctuation),
        _ if word == "bennett-object" => Ok(Atom::Header),
        _ if name(word) => Ok(Atom::Word),
        _ => Err("invalid NAME or numeric atom"),
    }
}

fn header_number(tokens: &[Token<'_>], i: usize) -> Lexical {
    match tokens.get(i - 2).map(|t| t.text) {
        Some("ptr_width") => unsigned(tokens[i].text),
        _ => {
            let (major, minor) = tokens[i]
                .text
                .split_once('.')
                .ok_or("version requires INT.INT")?;
            unsigned(major)?;
            unsigned(minor)
        }
    }
}

fn validate_atom(
    tokens: &[Token<'_>],
    i: usize,
    mode: Form,
    brackets: usize,
    kind: Number,
) -> Lexical {
    let token = &tokens[i];
    let previous = tokens[..i].last().map(|t| t.text);
    match (token.atom, previous, mode) {
        (_, Some(":"), _) => validate_type(&tokens[i..]),
        (_, Some("="), Form::Header) => header_number(tokens, i),
        (Atom::Numeric, _, _) => match kind {
            Number::Unsigned => unsigned(token.text),
            Number::Angle => numeric(token.text, mode, brackets, true),
            Number::Integer => numeric(token.text, mode, brackets, false),
        },
        (Atom::Star, Some("["), _) => match tokens[..i - 1].last().map(|t| t.text) {
            Some("qreg") => Ok(()),
            _ => Err("* is only a qreg width"),
        },
        (Atom::Star, _, _) => Err("* is only a qreg width"),
        (Atom::Header, _, Form::Header) => Ok(()),
        (Atom::Header, _, _) => Err("unexpected object header token"),
        _ => Ok(()),
    }
}

struct Context {
    mode: Form,
    brackets: usize,
    op: usize,
    number: Number,
}

impl Context {
    fn accept(&mut self, line: &str, tokens: &[Token<'_>], i: usize) -> Lexical {
        if i > 0
            && &line[tokens[i - 1].end..tokens[i].start] != gap(tokens, i, self.mode, self.brackets)
        {
            return Err("noncanonical punctuation spacing");
        }
        match (tokens[i].text, self.mode) {
            ("bounds", Form::Registers) => self.mode = Form::Bounds,
            ("[", _) => self.brackets += 1,
            ("]", _) => self.brackets = self.brackets.checked_sub(1).ok_or("unmatched ]")?,
            _ => {
                let number = match (self.number, i == self.op + 1) {
                    (Number::Angle, false) => Number::Integer,
                    (kind, _) => kind,
                };
                validate_atom(tokens, i, self.mode, self.brackets, number)?;
            }
        }
        Ok(())
    }
}

fn validate_words(line: &str, tokens: &[Token<'_>], mode: Form) -> Lexical {
    let op = opcode(tokens);
    let instruction = tokens.get(op).ok_or("missing opcode")?;
    if mode == Form::Instruction {
        match instruction.text {
            "@" => (),
            word => opcode_widths(word)?,
        }

        let gap = tokens.get(op + 1).map(|t| t.start - instruction.end);
        let required_gap = usize::from(instruction.text != "@");
        if matches!(gap, Some(n) if n != required_gap) || tokens[0].text == "=" {
            return Err("opcode requires a separate operand and an assignment requires a lhs");
        }
    }
    let mut context = Context {
        mode,
        brackets: 0,
        op,
        number: number_kind(instruction.text),
    };
    for i in 0..tokens.len() {
        context.accept(line, tokens, i)?;
    }
    match context.brackets {
        0 => Ok(()),
        _ => Err("unclosed ["),
    }
}

fn validate_line(line: &str) -> Lexical {
    let tokens = tokenize(line)?;
    let first = tokens.first().ok_or("empty token line")?;
    let last = tokens.last().ok_or("empty token line")?;
    if first.start != 0 || last.end != line.len() {
        return Err("leading or trailing whitespace");
    }
    let mode = form(&tokens);
    if mode == Form::Registers && last.text != "}" {
        return Err("regs and bounds must close on their declaration line");
    }
    validate_words(line, &tokens, mode)
}
