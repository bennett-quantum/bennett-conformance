use super::*;
#[derive(Clone, Copy, PartialEq)]
enum Body {
    Loop,
    Then,
    Else,
}
struct Frame {
    body: Body,
    nonempty: bool,
}

pub(super) fn validate(function: &Function) -> Result<(), String> {
    declarations(function)?;
    let mut frames: Vec<Frame> = vec![];
    let mut block = false;
    let mut nonempty = false;
    let mut regs = false;
    for (i, line) in function.bytes.lines().enumerate().skip(1) {
        match line {
            _ if line.starts_with("regs ") => {
                require(
                    i == 1 && !regs,
                    "§16.2",
                    "regs must occur once before blocks",
                )?;
                regs = true;
                if let Some(bounds) = line.split_once(" bounds {").map(|(_, s)| s) {
                    require(bounds != "}", "§16.2", "bounds requires at least one INT")?;
                }
            }
            _ if line.starts_with('^') => {
                require(
                    frames.is_empty() && (!block || nonempty),
                    "§16.2",
                    "unclosed or empty block",
                )?;
                require(
                    line.ends_with("):"),
                    "§16.2",
                    "block header must consume its complete line",
                )?;
                block = true;
                nonempty = false;
            }
            "}" | "} {" => {
                let frame = frames
                    .last_mut()
                    .ok_or_else(|| error("§16.2", "unmatched closing brace"))?;
                require(frame.nonempty, "§16.2", "empty structured body")?;
                match (line, frame.body) {
                    ("} {", Body::Then) => {
                        frame.body = Body::Else;
                        frame.nonempty = false;
                    }
                    ("} {", _) => return Err(error("§16.2", "else belongs only to qif")),
                    _ => {
                        frames.pop();
                    }
                }
            }
            _ => {
                require(block, "§16.2", "instruction must belong to a block")?;
                nonempty = true;
                if let Some(parent) = frames.last_mut() {
                    parent.nonempty = true;
                }
                let open = line.ends_with(" {");
                let row = Row::parse(line.strip_suffix(" {").unwrap_or(line), &function.kind)?;
                match row.code {
                    Code::Loop => {
                        require(open, "§16.2", "qloop requires a body")?;
                        frames.push(Frame {
                            body: Body::Loop,
                            nonempty: false,
                        });
                    }
                    Code::If => {
                        require(open, "§16.2", "qif requires a body")?;
                        frames.push(Frame {
                            body: Body::Then,
                            nonempty: false,
                        });
                    }
                    _ => require(!open, "§16.2", "only qloop/qif introduce bodies")?,
                }
            }
        }
    }
    require(
        block && nonempty && frames.is_empty(),
        "§16.2",
        "empty or unterminated function/structured body",
    )
}

fn declarations(function: &Function) -> Result<(), String> {
    let mut names = BTreeSet::new();
    for (group, params) in function.groups.iter().enumerate() {
        for param in params {
            require(
                wires::valid_name(&param.name),
                "§16.2",
                "invalid parameter NAME",
            )?;
            let ty = Type::parse(&param.ty)?;
            if !(matches!(function.kind.as_str(), "cc" | "r") && group == 1) {
                require(
                    names.insert(param.name.clone()),
                    "§3.3",
                    "repeated header name",
                )?;
            }
            match function.kind.as_str() {
                "cc" | "r" => require(
                    group < 2
                        && !matches!(ty, Type::Location(_) | Type::Wire(_) | Type::Region(_, _))
                        && param.suffix.is_none(),
                    "§16.2",
                    "invalid classical header group/type/role",
                )?,
                "lproc" => {
                    let valid = match group {
                        0 | 1 => matches!(ty, Type::Location(_)) && param.suffix.is_none(),
                        2 => {
                            ty == Type::Token
                                && param.suffix.as_deref().is_none_or(|s| s == "token")
                        }
                        3 => {
                            matches!(ty, Type::Integer(_) | Type::Pointer | Type::Float)
                                && param.suffix.as_deref().is_none_or(|s| s == "classical")
                        }
                        4 => {
                            matches!(ty, Type::Region(_, _))
                                && param.suffix.as_deref() == Some("in")
                        }
                        _ => false,
                    };
                    require(valid, "§3.7/§16.2", "invalid lproc header type/role")?;
                }
                _ => (),
            }
        }
    }
    if function.kind == "lproc" {
        require(
            function.groups[1].len() == 1,
            "§3.7",
            "lproc requires exactly one result location",
        )?;
    }
    for line in function.bytes.lines().filter(|l| l.starts_with("regs ")) {
        let rest = line
            .strip_prefix("regs { ")
            .ok_or_else(|| error("§16.2", "malformed regs"))?;
        let (entries, tail) = rest
            .split_once(" }")
            .ok_or_else(|| error("§16.2", "malformed regs closing brace"))?;
        if !entries.is_empty() {
            require(
                matches!(function.kind.as_str(), "oracle" | "lproc"),
                "§3.7",
                "only oracle/lproc may declare registers",
            )?;
            for p in super::super::parse_plist(entries)? {
                require(
                    wires::valid_name(&p.name) && names.insert(p.name),
                    "§3.3",
                    "invalid/duplicate register name",
                )?;
                require(
                    matches!(Type::parse(&p.ty)?, Type::Location(_)) && p.suffix.is_none(),
                    "§16.2",
                    "register declarations require reg[N] without a role",
                )?;
            }
        }
        if !tail.is_empty() {
            require(
                matches!(function.kind.as_str(), "action" | "perm_action"),
                "§16.2",
                "bounds belong to actions",
            )?;
            let values = tail
                .strip_prefix(" bounds {")
                .and_then(|s| s.strip_suffix('}'))
                .ok_or_else(|| error("§16.2", "unexpected trailing regs tokens"))?;
            for n in values.split(", ") {
                uint(n)?;
            }
        }
    }
    let mut labels = BTreeSet::new();
    for block in &function.blocks {
        require(
            wires::valid_name(&block.name) && labels.insert(&block.name),
            "§16.2",
            "invalid/duplicate block name",
        )?;
        require(
            matches!(function.kind.as_str(), "cc" | "r") || block.params.is_empty(),
            "§16.2",
            "quantum blocks take no parameters",
        )?;
        for p in &block.params {
            require(
                wires::valid_name(&p.name) && p.suffix.is_none(),
                "§16.2",
                "invalid block parameter",
            )?;
            Type::parse(&p.ty)?;
        }
    }
    Ok(())
}
