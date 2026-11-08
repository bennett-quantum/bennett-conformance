//! §3.3 syntactic view ownership over all CFG paths; other aliases are dynamic.
use super::*;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum State {
    Waiting,
    Consumed,
}
struct Node {
    row: Row,
    next: Vec<usize>,
}

pub(super) fn validate(function: &Function, scope: &Scalar) -> Result<(), String> {
    let graph = graph(function)?;
    for node in &graph {
        if node.row.code != Code::RegisterSlice {
            continue;
        }
        let view = &node.row.results[0];
        let parent = &node.row.args[0];
        let mut pending: Vec<_> = node.next.iter().map(|i| (*i, State::Waiting)).collect();
        let mut seen = BTreeSet::new();
        while let Some((index, state)) = pending.pop() {
            if !seen.insert((index, state)) {
                continue;
            }
            let node = &graph[index];
            let used = register_uses(&node.row, scope)?;
            let next_state = step(&node.row, view, parent, state, &used)?;
            if state == State::Consumed && node.row.results.contains(view) {
                continue;
            }
            require(
                !node.next.is_empty()
                    || node.row.code == Code::Fault
                    || next_state == State::Consumed,
                "§3.3",
                "slice view reaches function exit unconsumed",
            )?;
            pending.extend(node.next.iter().map(|i| (*i, next_state)));
        }
        require(
            !node.next.is_empty(),
            "§3.3",
            "slice view reaches function exit unconsumed",
        )?;
        // Every slice starts its own analysis, including an unreachable block
        // and a redefinition reached after a previous instance was consumed.
    }
    Ok(())
}
fn step(
    row: &Row,
    view: &str,
    parent: &str,
    state: State,
    used: &BTreeSet<String>,
) -> Result<State, String> {
    if state == State::Waiting {
        require(
            !used.contains(parent),
            "§3.3",
            "slice parent used before its view's region call",
        )?;
        require(
            !row.results.iter().any(|n| n == view),
            "§3.3",
            "slice view redefined before its region call",
        )?;
    }
    if !used.contains(view) {
        return Ok(state);
    }
    require(
        state == State::Waiting,
        "§3.3",
        "slice view used after its one region call",
    )?;
    require(
        matches!(row.code, Code::ActionCall | Code::OracleCall),
        "§3.3",
        "slice view used outside its one region call",
    )?;
    Ok(State::Consumed)
}
fn graph(function: &Function) -> Result<Vec<Node>, String> {
    let mut starts = BTreeMap::new();
    let mut size = 0;
    for block in &function.blocks {
        starts.insert(block.name.as_str(), size);
        size += block.instructions.len();
    }
    let mut graph = vec![];
    for block in &function.blocks {
        for (i, line) in block.instructions.iter().enumerate() {
            let row = Row::parse(line, &function.kind)?;
            let next = match row.code {
                Code::Br => {
                    let labels = if row.args.len() == 1 {
                        &row.args[..]
                    } else {
                        &row.args[1..]
                    };
                    labels
                        .iter()
                        .map(|text| {
                            let name = text
                                .trim_start_matches('^')
                                .split('(')
                                .next()
                                .expect("label has a name");
                            starts
                                .get(name)
                                .copied()
                                .ok_or_else(|| sec_err("§3.4", "unknown branch block"))
                        })
                        .collect::<Result<Vec<_>, _>>()?
                }
                Code::Ret | Code::Trap | Code::Fault => vec![],
                _ => {
                    require(
                        i + 1 < block.instructions.len(),
                        "§3.4",
                        "block has no terminator",
                    )?;
                    vec![graph.len() + 1]
                }
            };
            graph.push(Node { row, next });
        }
    }
    Ok(graph)
}
fn register_uses(row: &Row, scope: &Scalar) -> Result<BTreeSet<String>, String> {
    let mut names = BTreeSet::new();
    for text in row.args.iter().chain(&row.controls) {
        let tokens = if text.starts_with('(') {
            list(text)?
        } else if let Some((_, tail)) = text.strip_prefix('^').and_then(|s| s.split_once('(')) {
            list(&format!("({tail}"))?
        } else {
            vec![text.clone()]
        };
        for name in tokens {
            if matches!(scope.values.get(&name), Some(Type::Register(_))) {
                names.insert(name);
            }
        }
    }
    Ok(names)
}
