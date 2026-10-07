use std::collections::HashMap;
use std::fmt::Write;

use crate::domain::Event;
use crate::sas::Model;

/// A decomposition tree. Node indices are assigned in preorder, which for a totally ordered
/// problem means the leaves in index order are the plan in execution order.
#[derive(Debug)]
pub struct Node {
    pub task: u32,
    pub method: Option<u32>,
    pub children: Vec<usize>,
}

#[derive(Debug)]
pub struct Derivation {
    pub nodes: Vec<Node>,
    pub root: usize,
}

impl Derivation {
    pub fn leaves(&self) -> impl Iterator<Item = (usize, u32)> + '_ {
        self.nodes
            .iter()
            .enumerate()
            .filter(|(_, n)| n.method.is_none())
            .map(|(id, n)| (id, n.task))
    }
}

pub fn from_trace(m: &Model, events: &[Event]) -> Result<Derivation, String> {
    let mut nodes = Vec::new();
    let mut cursor = 0;
    let root = grow(m, events, &mut cursor, m.initial_task, &mut nodes)?;
    if cursor != events.len() {
        return Err(format!("{} trailing trace events", events.len() - cursor));
    }
    Ok(Derivation { nodes, root })
}

fn grow(
    m: &Model,
    events: &[Event],
    cursor: &mut usize,
    task: u32,
    nodes: &mut Vec<Node>,
) -> Result<usize, String> {
    let event = *events.get(*cursor).ok_or("the trace ends before the tree is complete")?;
    *cursor += 1;
    let id = nodes.len();
    match event {
        Event::Action(action) => {
            if action != task {
                return Err(format!("expected task {task}, the trace does action {action}"));
            }
            nodes.push(Node { task, method: None, children: Vec::new() });
        }
        Event::Method(method) => {
            let info =
                m.methods.get(method as usize).ok_or_else(|| format!("no method {method}"))?;
            if info.task != task {
                return Err(format!(
                    "method {} decomposes task {}, not {task}",
                    info.name, info.task
                ));
            }
            nodes.push(Node { task, method: Some(method), children: Vec::new() });
            let mut children = Vec::with_capacity(info.subtasks.len());
            for &subtask in &info.subtasks {
                children.push(grow(m, events, cursor, subtask, nodes)?);
            }
            nodes[id].children = children;
        }
    }
    Ok(id)
}

/// The IPC plan format `pandaPIparser -c` reads: the actions in execution order, the root, then
/// one line per decomposition naming its children.
pub fn format(m: &Model, derivation: &Derivation) -> String {
    let mut text = String::from("==>\n");
    for (id, task) in derivation.leaves() {
        writeln!(text, "{id} {}", m.task_names[task as usize]).unwrap();
    }
    writeln!(text, "root {}", derivation.root).unwrap();
    for (id, node) in derivation.nodes.iter().enumerate() {
        let Some(method) = node.method else { continue };
        write!(
            text,
            "{id} {} -> {}",
            m.task_names[node.task as usize], m.methods[method as usize].name
        )
        .unwrap();
        for child in &node.children {
            write!(text, " {child}").unwrap();
        }
        text.push('\n');
    }
    text.push_str("<==");
    text
}

/// A parsed line. The grounder does not give methods unique names, so a line fixes a set of
/// candidates and the tree walk picks the one whose subtasks are the children that are there.
struct Raw {
    task: u32,
    methods: Option<Vec<u32>>,
    children: Vec<u64>,
}

fn resolve(m: &Model, raw: &HashMap<u64, Raw>, id: u64, node: &Raw) -> Result<u32, String> {
    let candidates = node.methods.as_ref().expect("only called on decompositions");
    let mut children = Vec::with_capacity(node.children.len());
    for child in &node.children {
        children.push(raw.get(child).ok_or_else(|| format!("no node with id {child}"))?.task);
    }
    candidates
        .iter()
        .copied()
        .find(|&method| {
            let info = &m.methods[method as usize];
            info.task == node.task && info.subtasks == children.as_slice()
        })
        .ok_or_else(|| {
            format!(
                "node {id} applies {}, which does not decompose {} into those children",
                m.methods[candidates[0] as usize].name, m.task_names[node.task as usize]
            )
        })
}

fn assemble(
    m: &Model,
    raw: &HashMap<u64, Raw>,
    id: u64,
    expected: u32,
    nodes: &mut Vec<Node>,
    leaves: &mut Vec<u64>,
) -> Result<usize, String> {
    let node = raw.get(&id).ok_or_else(|| format!("no node with id {id}"))?;
    if node.task != expected {
        return Err(format!(
            "node {id} is {}, but its parent expects {}",
            m.task_names[node.task as usize], m.task_names[expected as usize]
        ));
    }
    let at = nodes.len();
    if node.methods.is_none() {
        nodes.push(Node { task: node.task, method: None, children: Vec::new() });
        leaves.push(id);
        return Ok(at);
    }

    let method = resolve(m, raw, id, node)?;
    let info = &m.methods[method as usize];
    nodes.push(Node { task: node.task, method: Some(method), children: Vec::new() });
    let mut children = Vec::with_capacity(node.children.len());
    for (&child, &subtask) in node.children.iter().zip(&info.subtasks) {
        children.push(assemble(m, raw, child, subtask, nodes, leaves)?);
    }
    nodes[at].children = children;
    Ok(at)
}

pub fn parse(m: &Model, text: &str) -> Result<Derivation, String> {
    let by_name: HashMap<&str, u32> =
        m.task_names.iter().enumerate().map(|(id, name)| (name.as_str(), id as u32)).collect();
    let mut methods: HashMap<&str, Vec<u32>> = HashMap::new();
    for (id, method) in m.methods.iter().enumerate() {
        methods.entry(method.name.as_str()).or_default().push(id as u32);
    }

    let mut lines = text.lines().map(str::trim).filter(|l| !l.is_empty());
    if lines.next() != Some("==>") {
        return Err("the plan does not start with ==>".into());
    }

    let mut raw: HashMap<u64, Raw> = HashMap::new();
    let mut order: Vec<u64> = Vec::new();
    let mut root = None;

    for line in lines {
        if line == "<==" {
            break;
        }
        if let Some(rest) = line.strip_prefix("root ") {
            root = Some(rest.trim().parse::<u64>().map_err(|e| format!("bad root id: {e}"))?);
            continue;
        }
        let (id, rest) = line.split_once(' ').ok_or_else(|| format!("bad line: {line}"))?;
        let id: u64 = id.parse().map_err(|e| format!("bad id in {line:?}: {e}"))?;
        match rest.split_once(" -> ") {
            None => {
                let task = *by_name.get(rest).ok_or_else(|| format!("unknown action {rest:?}"))?;
                if !m.is_primitive(task) {
                    return Err(format!("{rest} is not a primitive task"));
                }
                raw.insert(id, Raw { task, methods: None, children: Vec::new() });
                order.push(id);
            }
            Some((task, tail)) => {
                let task = *by_name.get(task).ok_or_else(|| format!("unknown task {task:?}"))?;
                let mut fields = tail.split_whitespace();
                let name = fields.next().ok_or_else(|| format!("no method in {line:?}"))?;
                let candidates =
                    methods.get(name).ok_or_else(|| format!("unknown method {name:?}"))?;
                let children = fields
                    .map(|f| f.parse::<u64>().map_err(|e| format!("bad child id: {e}")))
                    .collect::<Result<Vec<_>, _>>()?;
                raw.insert(id, Raw { task, methods: Some(candidates.clone()), children });
            }
        }
    }

    let root = root.ok_or("the plan has no root line")?;
    let mut nodes = Vec::new();
    let mut leaves = Vec::new();
    let root = assemble(m, &raw, root, m.initial_task, &mut nodes, &mut leaves)?;
    if leaves != order {
        return Err("the listed actions are not the leaves in execution order".into());
    }
    Ok(Derivation { nodes, root })
}

/// Checks the derivation against the model: every decomposition legal, and the actions it
/// yields executable from the initial state and reaching the goal.
pub fn check(m: &Model, derivation: &Derivation) -> Result<usize, String> {
    if derivation.nodes[derivation.root].task != m.initial_task {
        return Err(format!(
            "the root is {}, not the initial task {}",
            m.task_names[derivation.nodes[derivation.root].task as usize],
            m.task_names[m.initial_task as usize]
        ));
    }
    for (id, node) in derivation.nodes.iter().enumerate() {
        let Some(method) = node.method else {
            if !m.is_primitive(node.task) {
                return Err(format!(
                    "node {id} is the abstract task {} with no method",
                    m.task_names[node.task as usize]
                ));
            }
            continue;
        };
        let info = &m.methods[method as usize];
        if info.task != node.task || info.subtasks.len() != node.children.len() {
            return Err(format!("node {id} misapplies {}", info.name));
        }
        for (&child, &subtask) in node.children.iter().zip(&info.subtasks) {
            if derivation.nodes[child].task != subtask {
                return Err(format!("node {id} has the wrong child {child}"));
            }
        }
    }

    let mut state = vec![false; m.num_state_bits];
    for &fact in &m.s0 {
        state[fact as usize] = true;
    }
    let mut length = 0;
    for (id, task) in derivation.leaves() {
        let action = &m.actions[task as usize];
        let name = &m.task_names[task as usize];
        if let Some(&fact) = action.prec.iter().find(|&&f| !state[f as usize]) {
            return Err(format!(
                "action {id} {} needs {}, which does not hold",
                name, m.fact_names[fact as usize]
            ));
        }
        for &fact in &action.del {
            state[fact as usize] = false;
        }
        for &fact in &action.add {
            state[fact as usize] = true;
        }
        length += 1;
    }
    if let Some(&fact) = m.goal.iter().find(|&&f| !state[f as usize]) {
        return Err(format!(
            "the goal fact {} does not hold at the end",
            m.fact_names[fact as usize]
        ));
    }
    Ok(length)
}
