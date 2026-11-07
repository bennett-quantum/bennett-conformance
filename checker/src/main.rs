//! bennett-check <cert.json> --object <object.bisa> [--target <t.json>]
//! [--schedule <s.json>] [--library <cert.json>]... [--oracle-bin <path>]
//! [--max-elements N] [--max-wires N]
//!
//! Prints one deterministic JCS JSON verdict line to stdout.
//! Exit: 0 verified, 1 rejected, 2 unchecked, 3 usage/IO error.
//! With --from-object the positional argument is the object and its
//! embedded certificate section is checked.

use bennett_cert_checker::cert::{check_certificate, Inputs, Limits};
use std::path::PathBuf;
use std::process::ExitCode;

fn usage(code: u8) -> ExitCode {
    eprintln!(
        "usage: bennett-check <cert.json> --object <object.bisa> [--target <t.json>] [--schedule <s.json>] [--library <cert.json>]... [--oracle-bin <path>] [--max-elements N] [--max-wires N]\n       bennett-check --from-object <object.bisa> [--target <t.json>] [--schedule <s.json>] ..."
    );
    ExitCode::from(code)
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut positional: Option<String> = None;
    let mut object_path: Option<String> = None;
    let mut target_path: Option<String> = None;
    let mut schedule_path: Option<String> = None;
    let mut libraries: Vec<String> = Vec::new();
    let mut oracle_bin: Option<PathBuf> = None;
    let mut limits = Limits::default();
    let mut from_object = false;
    let mut i = 0;
    while i < args.len() {
        let a = &args[i];
        if matches!(
            a.as_str(),
            "--object"
                | "--target"
                | "--schedule"
                | "--library"
                | "--oracle-bin"
                | "--max-elements"
                | "--max-wires"
        ) && args.get(i + 1).map(|v| v.starts_with("--")).unwrap_or(true)
        {
            return usage(3);
        }
        let next = |i: &mut usize| -> Option<String> {
            *i += 1;
            args.get(*i).cloned()
        };
        match a.as_str() {
            "--object" => object_path = next(&mut i),
            "--target" => target_path = next(&mut i),
            "--schedule" => schedule_path = next(&mut i),
            "--library" => match next(&mut i) {
                Some(p) => libraries.push(p),
                None => return usage(3),
            },
            "--oracle-bin" => oracle_bin = next(&mut i).map(PathBuf::from),
            "--max-elements" => match next(&mut i).and_then(|v| v.parse().ok()) {
                Some(v) => limits.max_elements = v,
                None => return usage(3),
            },
            "--max-wires" => match next(&mut i).and_then(|v| v.parse().ok()) {
                Some(v) => limits.max_wires = v,
                None => return usage(3),
            },
            "--from-object" => from_object = true,
            "--help" | "-h" => {
                println!(
                    "bennett-check: independent Bennett ISA certificate checker (spec section 12)"
                );
                return usage(0);
            }
            _ if a.starts_with("--") => return usage(3),
            _ => {
                if positional.is_some() {
                    return usage(3);
                }
                positional = Some(a.clone());
            }
        }
        i += 1;
    }
    let read = |path: &str| -> Result<Vec<u8>, ExitCode> {
        std::fs::read(path).map_err(|e| {
            eprintln!("bennett-check: cannot read {path}: {e}");
            ExitCode::from(3)
        })
    };
    let read_text = |path: &str| -> Result<String, ExitCode> {
        let b = read(path)?;
        String::from_utf8(b).map_err(|_| {
            eprintln!("bennett-check: {path} is not UTF-8");
            ExitCode::from(3)
        })
    };
    let (cert_bytes, object_text);
    if from_object {
        let Some(path) = positional else {
            return usage(3);
        };
        object_text = match read_text(&path) {
            Ok(t) => t,
            Err(c) => return c,
        };
        // Extract the embedded certificate section's single JCS line.
        let lines: Vec<&str> = object_text.split('\n').collect();
        let mut cert_line: Option<&str> = None;
        for w in lines.windows(2) {
            if w[0] == "section certificate" {
                cert_line = Some(w[1]);
            }
        }
        match cert_line {
            Some(l) => cert_bytes = l.as_bytes().to_vec(),
            None => {
                eprintln!("bennett-check: {path} has no certificate section");
                return ExitCode::from(3);
            }
        }
    } else {
        let (Some(cert_path), Some(obj_path)) = (positional, object_path) else {
            return usage(3);
        };
        cert_bytes = match read(&cert_path) {
            Ok(b) => b,
            Err(c) => return c,
        };
        object_text = match read_text(&obj_path) {
            Ok(t) => t,
            Err(c) => return c,
        };
    }
    let target_bytes = match &target_path {
        Some(p) => match read(p) {
            Ok(b) => Some(b),
            Err(c) => return c,
        },
        None => None,
    };
    let schedule_bytes = match &schedule_path {
        Some(p) => match read(p) {
            Ok(b) => Some(b),
            Err(c) => return c,
        },
        None => None,
    };
    let mut library_bytes = Vec::new();
    for p in &libraries {
        match read(p) {
            Ok(b) => library_bytes.push(b),
            Err(c) => return c,
        }
    }
    let verdict = check_certificate(&Inputs {
        cert_bytes,
        object_text,
        target_bytes,
        schedule_bytes,
        libraries: library_bytes,
        oracle_bin,
        limits,
    });
    println!("{}", String::from_utf8(verdict.to_json_bytes()).unwrap());
    ExitCode::from(verdict.exit_code() as u8)
}
