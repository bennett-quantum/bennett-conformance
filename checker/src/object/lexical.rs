//! ISA §16.1 lexical checks run before hashing or interpreting function bodies.
use super::sec_err;

pub(super) fn validate(lines: &[&str]) -> Result<(), String> {
    let mut section = "";
    for (i, line) in lines.iter().enumerate() {
        if line.is_empty() {
            continue;
        }
        let fail = |message: &str| sec_err("§16.1", format!("line {}: {message}", i + 1));
        if line.contains('\r') {
            return Err(fail("expected LF line endings"));
        }
        if let Some(name) = line.strip_prefix("section ") {
            section = name;
        } else if matches!(
            section,
            "symbols" | "imports" | "devices" | "layout" | "schedule" | "certificate" | "coverage"
        ) {
            // JSON strings have their own spacing and lexical grammar.
            continue;
        } else if section == "data" {
            // Empty HEX has one mandatory trailing separator space.
            let (name, hex) = line
                .split_once(' ')
                .ok_or_else(|| fail("data requires NAME SP HEX"))?;
            if name.is_empty()
                || !hex.len().is_multiple_of(2)
                || !hex
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            {
                return Err(fail("data requires a name and lowercase even-length HEX"));
            }
            continue;
        }
        let empty_regs = line.starts_with("regs {  }");
        let spacing = if empty_regs {
            line.replacen("regs {  }", "regs { }", 1)
        } else {
            line.to_string()
        };
        if line.starts_with("regs { }") {
            return Err(fail("empty regs is spelled regs {  }"));
        }
        let mut quoted = false;
        let mut escaped = false;
        let mut previous_space = false;
        let mut atom = String::new();
        for c in spacing.chars().chain(std::iter::once(' ')) {
            if quoted {
                if escaped {
                    escaped = false;
                } else if c == '\\' {
                    escaped = true;
                } else if c == '"' {
                    quoted = false;
                }
                continue;
            }
            if c == '"' {
                quoted = true;
                previous_space = false;
                continue;
            }
            if c.is_whitespace() || "()[]{},:=@^".contains(c) {
                check_atom(&atom).map_err(|e| fail(e))?;
                atom.clear();
            } else {
                atom.push(c);
            }
            if c.is_whitespace() && c != ' ' {
                return Err(fail("tokens require ASCII spaces"));
            }
            if c == ' ' && previous_space {
                return Err(fail("multiple or trailing spaces between tokens"));
            }
            previous_space = c == ' ';
            if matches!(c, '#' | ';') {
                return Err(fail("object text has no comment syntax"));
            }
        }
        if quoted {
            return Err(fail("unterminated string"));
        }
        if spacing.starts_with(' ') {
            return Err(fail("indented line"));
        }
    }
    Ok(())
}

fn check_atom(atom: &str) -> Result<(), &'static str> {
    if atom.starts_with(|c: char| c.is_ascii_digit() || c == '+' || c == '-') {
        let digits = atom.strip_prefix('-').unwrap_or(atom);
        if !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()) {
            if digits.len() > 1 && digits.starts_with('0') {
                return Err("noncanonical INT");
            }
        } else {
            let value: f64 = atom.parse().map_err(|_| "invalid numeric token")?;
            if !value.is_finite() {
                return Err("nonfinite FLOAT");
            }
            let mut expected = if value == 0.0 && value.is_sign_negative() {
                "-0.0".into()
            } else {
                crate::json::ecmascript_number(value)
            };
            if !expected.contains(['.', 'e']) {
                expected.push_str(".0");
            }
            if atom != expected {
                return Err("noncanonical FLOAT or INT");
            }
        }
    }
    Ok(())
}
