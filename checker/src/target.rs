//! Closed target metadata schemas from ISA §8. Unknown fields and malformed
//! nested values reject even if a particular certificate never uses them.
use crate::cert::{check_keys, rej, u64_of};
use crate::fault::Reject;
use crate::json::Value;
use crate::rat::{parse_biguint_str, rat_of_json};
use num_traits::Signed;
struct Check<'a> {
    root: &'a Value,
    errors: &'a mut Vec<Reject>,
}
impl Check<'_> {
    fn bad(&mut self, p: &str, m: &str) {
        self.errors.push(rej("E_CERT_TARGET", "§8", p, m));
    }
    fn object(&mut self, v: &Value, fields: &[&str], required: &[&str], p: &str) -> bool {
        let mut errors = vec![];
        let ok = check_keys(v, fields, required, p, &mut errors);
        for mut e in errors {
            e.code = "E_CERT_TARGET";
            e.section = "§8";
            self.errors.push(e);
        }
        ok
    }
    fn string(&mut self, v: Option<&Value>, p: &str) {
        if v.and_then(Value::as_str).is_none() {
            self.bad(p, "expected a string");
        }
    }
    fn number(&mut self, v: Option<&Value>, p: &str, nonnegative: bool) {
        if v.and_then(|v| rat_of_json(v).ok())
            .map(|r| !nonnegative || !r.is_negative())
            != Some(true)
        {
            self.bad(p, "expected an exact finite decimal in the required range");
        }
    }
    fn uint(&mut self, v: Option<&Value>, p: &str, unbounded: bool) {
        if v.map(|v| {
            (unbounded && v.as_str() == Some("unbounded"))
                || u64_of(v).is_some()
                || v.as_str().and_then(parse_biguint_str).is_some()
        }) != Some(true)
        {
            self.bad(
                p,
                "expected a nonnegative integer or an admitted unbounded value",
            );
        }
    }
    fn boolean(&mut self, v: Option<&Value>, p: &str) {
        if !matches!(v, Some(Value::Bool(_))) {
            self.bad(p, "expected a boolean");
        }
    }
    fn array<'a>(&mut self, v: Option<&'a Value>, p: &str) -> Option<&'a [Value]> {
        let a = v.and_then(Value::as_arr);
        if a.is_none() {
            self.bad(p, "expected an array");
        }
        a
    }
    fn map<'a>(
        &mut self,
        v: Option<&'a Value>,
        p: &str,
    ) -> Option<&'a std::collections::BTreeMap<String, Value>> {
        let m = v.and_then(Value::as_obj);
        if m.is_none() {
            self.bad(p, "expected an object");
        }
        m
    }
    fn expression(&mut self, v: &Value, vars: &[&str], p: &str) {
        if matches!(v, Value::Num(_) | Value::Str(_)) {
            self.number(Some(v), p, false);
            return;
        }
        if v.get("var").is_some() {
            if self.object(v, &["var"], &["var"], p)
                && !v
                    .get("var")
                    .and_then(Value::as_str)
                    .map(|s| vars.contains(&s))
                    .unwrap_or(false)
            {
                self.bad(p, "expression variable is not bound in this context");
            }
            return;
        }
        if v.get("table").is_some() {
            if self.object(v, &["table", "index"], &["table", "index"], p) {
                let table = v.get("table").and_then(Value::as_str);
                if table
                    .and_then(|t| self.root.get("tables").and_then(|m| m.get(t)))
                    .is_none()
                {
                    self.bad(p, "expression references an undeclared table");
                }
                match v.get("index").and_then(Value::as_str) {
                    Some("k") if vars.contains(&"k") => {}
                    Some("eps_decade") if vars.contains(&"eps") => {}
                    _ => self.bad(
                        p,
                        "table index must be k or eps_decade in a context binding that variable",
                    ),
                }
            }
            return;
        }
        if !self.object(v, &["op", "args"], &["op", "args"], p) {
            return;
        }
        let op = v.get("op").and_then(Value::as_str);
        let arity = match op {
            Some("+" | "-" | "*" | "/" | "min" | "max") => Some(2),
            Some("ceil") => Some(1),
            Some("log2inv_eps") if vars.contains(&"eps") => Some(0),
            _ => None,
        };
        if let Some(args) = self.array(v.get("args"), p) {
            if Some(args.len()) != arity {
                self.bad(p, "unknown expression operation or incorrect arity");
            }
            for value in args {
                self.expression(value, vars, p);
            }
        }
    }
    fn controls(&mut self, v: &Value, p: &str) {
        if self.object(v, &["min", "max"], &["min", "max"], p) {
            self.uint(v.get("min"), p, false);
            self.uint(v.get("max"), p, true);
        }
    }
    fn costs(&mut self, v: &Value, vars: &[&str], p: &str) {
        if let Some(m) = self.map(Some(v), p) {
            if !m.contains_key("time") {
                self.bad(p, "cost object requires a time expression");
            }
            for (name, e) in m {
                if self
                    .root
                    .get("resources")
                    .and_then(|r| r.get(name))
                    .is_none()
                {
                    self.bad(p, "cost references an undeclared resource");
                }
                self.expression(e, vars, p);
            }
        }
    }
    fn class(&mut self, v: Option<&Value>, p: &str, wildcard: bool) {
        let name = v.and_then(Value::as_str);
        if !name
            .map(|c| {
                (wildcard && c == "*") || self.root.get("classes").and_then(|m| m.get(c)).is_some()
            })
            .unwrap_or(false)
        {
            self.bad(p, "expected a declared class name");
        }
    }
    fn gate(&mut self, v: &Value, p: &str) {
        if !self.object(
            v,
            &["op", "classes", "controls", "params", "epsilon", "cost"],
            &["op", "classes", "controls", "epsilon", "cost"],
            p,
        ) {
            return;
        }
        self.string(v.get("op"), p);
        if let Some(a) = self.array(v.get("classes"), p) {
            for c in a {
                self.class(Some(c), p, true);
            }
        }
        self.controls(v.get("controls").unwrap(), p);
        if let Some(a) = v.get("params") {
            if let Some(a) = self.array(Some(a), p) {
                for param in a {
                    self.string(Some(param), p);
                }
            }
        }
        let exact = v
            .get("epsilon")
            .and_then(|e| rat_of_json(e).ok())
            .map(|r| r == crate::rat::Rat::from_integer(0.into()))
            .unwrap_or(false);
        let vars = if exact {
            &["k", "c"][..]
        } else {
            &["k", "c", "eps"][..]
        };
        self.expression(v.get("epsilon").unwrap(), vars, p);
        self.costs(v.get("cost").unwrap(), vars, p);
    }
    fn scheme(&mut self, v: &Value, p: &str) {
        if !self.object(
            v,
            &["name", "realises", "ancillae", "template"],
            &["name", "realises", "ancillae", "template"],
            p,
        ) {
            return;
        }
        self.string(v.get("name"), p);
        let realises = v.get("realises").unwrap();
        if self.object(
            realises,
            &["op", "controls", "params"],
            &["op", "controls"],
            p,
        ) {
            self.string(realises.get("op"), p);
            self.controls(realises.get("controls").unwrap(), p);
            if let Some(params) = realises.get("params") {
                if let Some(params) = self.array(Some(params), p) {
                    for param in params {
                        self.string(Some(param), p);
                    }
                }
            }
        }
        let a = v.get("ancillae").unwrap();
        let parametric = realises
            .get("params")
            .and_then(Value::as_arr)
            .map(|a| a.len() == 1 && a[0].as_str() == Some("theta"))
            == Some(true);
        let scheme_vars = if parametric {
            &["k", "c", "theta"][..]
        } else {
            &["k", "c"][..]
        };
        if self.object(
            a,
            &["count", "state", "class"],
            &["count", "state", "class"],
            p,
        ) {
            self.expression(a.get("count").unwrap(), scheme_vars, p);
            self.class(a.get("class"), p, false);
            if !matches!(
                a.get("state").and_then(Value::as_str),
                Some("clean" | "dirty")
            ) {
                self.bad(p, "ancilla state must be clean or dirty");
            }
        }
        if let Some(template) = self.array(v.get("template"), p) {
            for leaf in template {
                if !self.object(
                    leaf,
                    &["gate", "wires", "params", "repeat"],
                    &["gate", "wires"],
                    p,
                ) {
                    continue;
                }
                self.string(leaf.get("gate"), p);
                if let Some(wires) = self.array(leaf.get("wires"), p) {
                    for wire in wires {
                        self.string(Some(wire), p);
                    }
                }
                if let Some(repeat) = leaf.get("repeat") {
                    if self.object(repeat, &["var", "from", "to"], &["var", "from", "to"], p) {
                        if repeat.get("var").and_then(Value::as_str) != Some("i") {
                            self.bad(p, "template repeat binds i");
                        }
                        for key in ["from", "to"] {
                            self.expression(repeat.get(key).unwrap(), scheme_vars, p);
                        }
                    }
                }
                if let Some(params) = leaf.get("params") {
                    let mut vars = Vec::new();
                    if parametric {
                        vars.push("theta");
                    }
                    if leaf.get("repeat").is_some() {
                        vars.push("i");
                    }
                    if let Some(params) = self.array(Some(params), p) {
                        for param in params {
                            self.expression(param, &vars, p);
                        }
                    }
                }
            }
        }
    }
    fn all(&mut self) {
        let t = self.root;
        if let Some(widths) =
            self.array(t.get("ptr_widths_supported"), "target.ptr_widths_supported")
        {
            for w in widths {
                self.uint(Some(w), "target.ptr_widths_supported", false);
            }
        }
        if let Some(classes) = self.map(t.get("classes"), "target.classes") {
            for value in classes.values() {
                if self.object(
                    value,
                    &["capacity", "qubit_second", "dirty_rate"],
                    &["capacity", "qubit_second", "dirty_rate"],
                    "target.classes",
                ) {
                    self.uint(value.get("capacity"), "target.classes.capacity", true);
                    for key in ["qubit_second", "dirty_rate"] {
                        self.number(value.get(key), "target.classes", true);
                    }
                }
            }
        }
        if let Some(resources) = self.map(t.get("resources"), "target.resources") {
            for value in resources.values() {
                if self.object(
                    value,
                    &["kind", "unit", "rate"],
                    &["kind", "unit"],
                    "target.resources",
                ) {
                    if !matches!(
                        value.get("kind").and_then(Value::as_str),
                        Some("count" | "time" | "occupancy" | "peak")
                    ) {
                        self.bad("target.resources.kind", "unknown resource kind");
                    }
                    self.string(value.get("unit"), "target.resources.unit");
                    if let Some(rate) = value.get("rate") {
                        if let Some(rate) = self.map(Some(rate), "target.resources.rate") {
                            for (name, value) in rate {
                                self.class(
                                    Some(&Value::Str(name.clone())),
                                    "target.resources.rate",
                                    false,
                                );
                                self.number(Some(value), "target.resources.rate", true);
                            }
                        }
                    }
                }
            }
        }
        let caps = t.get("capabilities").unwrap_or(&Value::Null);
        if self.object(
            caps,
            &[
                "measurement",
                "mbu",
                "max_parallel",
                "max_predicate_depth",
                "classical_step_time",
            ],
            &[
                "measurement",
                "mbu",
                "max_parallel",
                "max_predicate_depth",
                "classical_step_time",
            ],
            "target.capabilities",
        ) {
            self.boolean(caps.get("mbu"), "target.capabilities.mbu");
            for key in ["max_parallel", "max_predicate_depth"] {
                self.uint(caps.get(key), "target.capabilities", true);
            }
            self.number(
                caps.get("classical_step_time"),
                "target.capabilities.classical_step_time",
                true,
            );
            let m = caps.get("measurement").unwrap();
            if self.object(
                m,
                &["mid_circuit", "feedforward_latency"],
                &["mid_circuit", "feedforward_latency"],
                "target.capabilities.measurement",
            ) {
                self.boolean(
                    m.get("mid_circuit"),
                    "target.capabilities.measurement.mid_circuit",
                );
                self.number(
                    m.get("feedforward_latency"),
                    "target.capabilities.measurement.feedforward_latency",
                    true,
                );
            }
        }
        if let Some(tables) = self.map(t.get("tables"), "target.tables") {
            for table in tables.values() {
                if let Some(a) = self.array(Some(table), "target.tables") {
                    for v in a {
                        self.number(Some(v), "target.tables", false);
                    }
                }
            }
        }
        if let Some(gates) = self.array(t.get("gates"), "target.gates") {
            for (i, gate) in gates.iter().enumerate() {
                self.gate(gate, &format!("target.gates[{i}]"));
            }
        }
        if let Some(schemes) = self.array(t.get("schemes"), "target.schemes") {
            for (i, scheme) in schemes.iter().enumerate() {
                self.scheme(scheme, &format!("target.schemes[{i}]"));
            }
        }
        let qram = t.get("qram").unwrap_or(&Value::Null);
        if *qram != Value::Null
            && self.object(
                qram,
                &["load", "store", "exch"],
                &["load", "store", "exch"],
                "target.qram",
            )
        {
            for key in ["load", "store", "exch"] {
                let r = qram.get(key).unwrap();
                if self.object(r, &["cost"], &["cost"], "target.qram") {
                    self.costs(r.get("cost").unwrap(), &["a", "w", "c"], "target.qram.cost");
                }
            }
        }
        if let Some(imports) = self.map(t.get("imports"), "target.imports") {
            for value in imports.values() {
                if self.object(
                    value,
                    &["version", "map"],
                    &["version", "map"],
                    "target.imports",
                ) {
                    self.string(value.get("version"), "target.imports.version");
                    if let Some(map) = self.map(value.get("map"), "target.imports.map") {
                        for row in map.values() {
                            if let Some(a) = self.array(Some(row), "target.imports.map") {
                                if a.len() != 2 {
                                    self.bad("target.imports.map", "expected resource/factor pair");
                                } else {
                                    self.string(Some(&a[0]), "target.imports.map");
                                    self.number(Some(&a[1]), "target.imports.map", false);
                                }
                            }
                        }
                    }
                }
            }
        }
        let objective = t.get("objective").unwrap_or(&Value::Null);
        match objective.as_obj() {
            Some(m) if m.len() == 1 => {
                if let Some(weighted) = m.get("weighted") {
                    if let Some(m) = self.map(Some(weighted), "target.objective.weighted") {
                        for (name, v) in m {
                            if t.get("resources").and_then(|m| m.get(name)).is_none() {
                                self.bad("target.objective", "objective names undeclared resource");
                            }
                            self.number(Some(v), "target.objective.weighted", true);
                        }
                    }
                } else if let Some(lex) = m.get("lexicographic") {
                    if let Some(a) = self.array(Some(lex), "target.objective.lexicographic") {
                        for name in a {
                            if name
                                .as_str()
                                .and_then(|n| t.get("resources").and_then(|r| r.get(n)))
                                .is_none()
                            {
                                self.bad("target.objective", "objective names undeclared resource");
                            }
                        }
                    }
                } else {
                    self.bad(
                        "target.objective",
                        "expected weighted or lexicographic objective",
                    );
                }
            }
            _ => self.bad(
                "target.objective",
                "objective must contain one aggregation choice",
            ),
        }
        if let Some(topo) = t.get("topology") {
            if self.object(
                topo,
                &["vertices", "edges"],
                &["vertices", "edges"],
                "target.topology",
            ) {
                if let Some(vs) = self.array(topo.get("vertices"), "target.topology.vertices") {
                    for vertex in vs {
                        if self.object(vertex, &["class"], &["class"], "target.topology.vertices") {
                            self.class(
                                vertex.get("class"),
                                "target.topology.vertices.class",
                                false,
                            );
                        }
                    }
                }
                self.array(topo.get("edges"), "target.topology.edges");
            }
        }
    }
}
pub(crate) fn validate(v: &Value, errors: &mut Vec<Reject>) {
    Check { root: v, errors }.all();
}
