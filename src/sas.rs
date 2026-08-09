//! Reader for pandaPIgrounder's `.sas` format, cut down to what the generator needs.
//!
//! Parsing is strictly positional: section headers and blank lines are discarded by position,
//! never matched, and comments are not tolerated anywhere. Integer lists are `-1`-terminated,
//! and any other negative `x` stands for `x + 1`. Effect lists are blocks of
//! `<n_conditions> <condition>* <effect>` terminated by a negative number.
//!
//! Everything the search does not need is consumed and dropped: mutex groups, invariants,
//! action costs, and the orderings of a method beyond the sequence they put its subtasks in.

use std::fmt;
use std::path::Path;

#[derive(Debug)]
pub struct ReadError {
    pub line: usize,
    pub message: String,
}

impl fmt::Display for ReadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}: {}", self.line, self.message)
    }
}

impl std::error::Error for ReadError {}

impl From<std::io::Error> for ReadError {
    fn from(e: std::io::Error) -> ReadError {
        ReadError { line: 0, message: e.to_string() }
    }
}

#[derive(Debug, Default)]
pub struct Action {
    pub prec: Vec<u32>,
    pub add: Vec<u32>,
    pub del: Vec<u32>,
}

#[derive(Debug)]
pub struct Method {
    pub name: String,
    pub task: u32,
    /// Subtasks in execution order, empty when this method's orderings are not a chain.
    pub subtasks: Vec<u32>,
}

#[derive(Debug, Default)]
pub struct Model {
    pub num_state_bits: usize,
    pub fact_names: Vec<String>,
    pub num_actions: usize,
    pub actions: Vec<Action>,
    pub has_conditional_effects: bool,
    pub s0: Vec<u32>,
    pub goal: Vec<u32>,
    pub num_tasks: usize,
    pub task_names: Vec<String>,
    pub initial_task: u32,
    pub methods: Vec<Method>,
    pub methods_for: Vec<Vec<u32>>,
    pub is_htn: bool,
    pub is_totally_ordered: bool,
}

impl Model {
    pub fn is_primitive(&self, task: u32) -> bool {
        (task as usize) < self.num_actions
    }
}

struct Cursor<'a> {
    lines: std::str::Lines<'a>,
    line_no: usize,
    eof: bool,
}

impl<'a> Cursor<'a> {
    fn new(text: &'a str) -> Self {
        Cursor { lines: text.lines(), line_no: 0, eof: false }
    }

    fn err<T>(&self, message: impl Into<String>) -> Result<T, ReadError> {
        Err(ReadError { line: self.line_no, message: message.into() })
    }

    /// The grounder's reader reads past the end freely: `getline` yields an empty line and
    /// sets eof, and a classical problem simply stops before the hierarchical sections.
    fn line(&mut self) -> &'a str {
        match self.lines.next() {
            Some(l) => {
                self.line_no += 1;
                l
            }
            None => {
                self.eof = true;
                ""
            }
        }
    }

    fn require_line(&mut self, what: &str) -> Result<&'a str, ReadError> {
        let line = self.line();
        if self.eof {
            return self.err(format!("unexpected end of file while reading {what}"));
        }
        Ok(line)
    }

    fn skip(&mut self, n: usize) {
        for _ in 0..n {
            self.line();
        }
    }

    fn count(&mut self, what: &str) -> Result<usize, ReadError> {
        let line = self.require_line(what)?;
        match first_int(line) {
            Some(n) if n >= 0 => Ok(n as usize),
            _ => self.err(format!("expected a count for {what}, found {line:?}")),
        }
    }
}

fn ints(line: &str) -> impl Iterator<Item = Option<i64>> + '_ {
    line.split_ascii_whitespace().map(|t| t.parse::<i64>().ok())
}

fn first_int(line: &str) -> Option<i64> {
    line.split_ascii_whitespace().next().and_then(|t| t.parse::<i64>().ok())
}

fn read_int_list(cursor: &Cursor, line: &str, out: &mut Vec<i32>) -> Result<(), ReadError> {
    out.clear();
    for token in ints(line) {
        let Some(x) = token else {
            return cursor.err(format!("malformed integer list: {line:?}"));
        };
        if x == -1 {
            return Ok(());
        }
        out.push(if x < 0 { (x + 1) as i32 } else { x as i32 });
    }
    Ok(())
}

/// Unconditional effects land in `plain`; a block with conditions is only counted, since the
/// generator refuses a model that has any.
fn read_effects(
    cursor: &Cursor,
    line: &str,
    plain: &mut Vec<u32>,
    conditional: &mut bool,
) -> Result<(), ReadError> {
    let mut it = ints(line);
    loop {
        let Some(header) = it.next() else {
            return Ok(());
        };
        let Some(n) = header else {
            return cursor.err(format!("malformed conditional effect list: {line:?}"));
        };
        if n < 0 {
            return Ok(());
        }
        for _ in 0..n {
            match it.next() {
                Some(Some(c)) if c >= 0 => *conditional = true,
                _ => return cursor.err(format!("malformed effect condition: {line:?}")),
            }
        }
        let effect = match it.next() {
            Some(Some(e)) if e >= 0 => e as u32,
            _ => return cursor.err(format!("malformed effect atom: {line:?}")),
        };
        if n == 0 {
            plain.push(effect);
        }
    }
}

pub fn from_file(path: impl AsRef<Path>) -> Result<Model, ReadError> {
    from_str(&std::fs::read_to_string(path)?)
}

pub fn from_str(text: &str) -> Result<Model, ReadError> {
    let mut cursor = Cursor::new(text);
    let mut model = Model::default();
    cursor.skip(1);
    read_classical(&mut cursor, &mut model)?;
    read_hierarchical(&mut cursor, &mut model)?;
    Ok(model)
}

fn read_classical(cursor: &mut Cursor, m: &mut Model) -> Result<(), ReadError> {
    let mut scratch: Vec<i32> = Vec::new();

    m.num_state_bits = cursor.count("state features")?;
    for _ in 0..m.num_state_bits {
        m.fact_names.push(cursor.require_line("fact names")?.to_string());
    }

    cursor.skip(2);
    let num_vars = cursor.count("mutex groups")?;
    cursor.skip(num_vars + 2);

    for what in ["strict mutex groups", "non-strict mutex groups", "invariants"] {
        let n = cursor.count(what)?;
        cursor.skip(n + 2);
    }

    m.num_actions = cursor.count("actions")?;
    for a in 0..m.num_actions {
        let mut action = Action::default();
        cursor.require_line("action cost")?;

        let line = cursor.require_line("action preconditions")?;
        read_int_list(cursor, line, &mut scratch)?;
        for &p in &scratch {
            if p < 0 || p as usize >= m.num_state_bits {
                return cursor.err(format!("precondition {p} of action {a} out of bounds"));
            }
            action.prec.push(p as u32);
        }
        action.prec.sort_unstable();
        action.prec.dedup();

        let line = cursor.require_line("action add effects")?;
        read_effects(cursor, line, &mut action.add, &mut m.has_conditional_effects)?;
        let line = cursor.require_line("action delete effects")?;
        read_effects(cursor, line, &mut action.del, &mut m.has_conditional_effects)?;

        for &f in action.add.iter().chain(&action.del) {
            if f as usize >= m.num_state_bits {
                return cursor.err(format!("effect {f} of action {a} out of bounds"));
            }
        }
        m.actions.push(action);
    }

    cursor.skip(2);
    let line = cursor.require_line("initial state")?;
    read_int_list(cursor, line, &mut scratch)?;
    m.s0 = scratch.iter().map(|&x| x as u32).collect();

    cursor.skip(2);
    let line = cursor.require_line("goal")?;
    read_int_list(cursor, line, &mut scratch)?;
    m.goal = scratch.iter().map(|&x| x as u32).collect();

    for (what, facts) in [("initial state", &m.s0), ("goal", &m.goal)] {
        if let Some(&bad) = facts.iter().find(|&&x| x as usize >= m.num_state_bits) {
            return cursor.err(format!("fact {bad} in {what} out of bounds"));
        }
    }

    cursor.skip(2);
    m.num_tasks = cursor.count("tasks")?;
    m.is_htn = true;
    for i in 0..m.num_tasks {
        let line = cursor.require_line("task names")?;
        let mut tok = line.split_ascii_whitespace();
        let Some(Ok(abstract_flag)) = tok.next().map(|t| t.parse::<i64>()) else {
            return cursor.err(format!("malformed task line: {line:?}"));
        };
        if (abstract_flag == 0) != (i < m.num_actions) {
            return cursor.err(format!(
                "task {i} is {}, but primitive tasks must occupy 0..{}",
                if abstract_flag == 0 { "primitive" } else { "abstract" },
                m.num_actions
            ));
        }
        m.task_names.push(tok.next().unwrap_or("").to_string());
    }
    Ok(())
}

fn read_hierarchical(cursor: &mut Cursor, m: &mut Model) -> Result<(), ReadError> {
    let mut scratch: Vec<i32> = Vec::new();

    let excess = cursor.line();
    if !cursor.eof && !excess.is_empty() {
        return cursor.err("excess task (list of tasks is longer than expected)");
    }
    cursor.line();
    let line = cursor.line();
    if cursor.eof {
        m.is_htn = false;
        return Ok(());
    }
    let Some(initial) = first_int(line) else {
        return cursor.err(format!("malformed initial task: {line:?}"));
    };
    if initial < 0 || initial as usize >= m.num_tasks {
        return cursor.err(format!("initial task {initial} out of bounds"));
    }
    m.initial_task = initial as u32;

    cursor.skip(2);
    let num_methods = cursor.count("methods")?;
    m.is_totally_ordered = true;
    m.methods_for = vec![Vec::new(); m.num_tasks];

    for i in 0..num_methods {
        let name = cursor.require_line("method name")?.to_string();

        let line = cursor.require_line("decomposed task")?;
        let Some(task) = first_int(line) else {
            return cursor.err(format!("malformed decomposed task for method {i}: {line:?}"));
        };
        if task < m.num_actions as i64 || task as usize >= m.num_tasks {
            return cursor
                .err(format!("method {i} decomposes {task}, which is not an abstract task"));
        }

        let line = cursor.require_line("method subtasks")?;
        read_int_list(cursor, line, &mut scratch)?;
        if scratch.is_empty() {
            return cursor.err(format!("method {name:?} has no subtasks"));
        }
        let mut subtasks = Vec::with_capacity(scratch.len());
        for &st in &scratch {
            if st < 0 || st as usize >= m.num_tasks {
                return cursor.err(format!("subtask {st} of method {i} out of bounds"));
            }
            subtasks.push(st as u32);
        }

        let line = cursor.require_line("method orderings")?;
        read_int_list(cursor, line, &mut scratch)?;
        if !scratch.len().is_multiple_of(2) {
            return cursor.err(format!("method {i} has an odd number of ordering entries"));
        }
        let mut pairs = Vec::with_capacity(scratch.len() / 2);
        for pair in scratch.chunks_exact(2) {
            let (before, after) = (pair[0], pair[1]);
            if before < 0
                || after < 0
                || before as usize >= subtasks.len()
                || after as usize >= subtasks.len()
            {
                return cursor.err(format!("ordering entry of method {i} out of bounds"));
            }
            pairs.push((before as usize, after as usize));
        }

        let order = match chain(subtasks.len(), &pairs) {
            Some(order) => order,
            None => {
                m.is_totally_ordered = false;
                Vec::new()
            }
        };
        m.methods_for[task as usize].push(i as u32);
        m.methods.push(Method {
            name,
            task: task as u32,
            subtasks: order.iter().map(|&pos| subtasks[pos]).collect(),
        });
    }
    Ok(())
}

/// The subtask positions in execution order, or `None` when the orderings do not put them in
/// one: a topological sort is the execution order exactly when it is the only one there is, so
/// every round must have exactly one position with no unsatisfied predecessor.
fn chain(n: usize, pairs: &[(usize, usize)]) -> Option<Vec<usize>> {
    let mut edges: Vec<(usize, usize)> = pairs.to_vec();
    edges.sort_unstable();
    edges.dedup();

    let mut in_degree = vec![0usize; n];
    for &(_, after) in &edges {
        in_degree[after] += 1;
    }

    let mut order = Vec::with_capacity(n);
    let mut done = vec![false; n];
    for _ in 0..n {
        let mut ready = (0..n).filter(|&x| !done[x] && in_degree[x] == 0);
        let next = ready.next()?;
        if ready.next().is_some() {
            return None;
        }
        done[next] = true;
        for &(before, after) in &edges {
            if before == next {
                in_degree[after] -= 1;
            }
        }
        order.push(next);
    }
    Some(order)
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = r#";; #state features
4
f0
f1
f2
f3

;; Mutex Groups
4
0 0 f0
1 1 f1
2 2 f2
3 3 f3

;; further strict Mutex Groups
1
0 1 -1

;; further non strict Mutex Groups
0

;; known invariants
1
-2 3 -1

;; Actions
3
1
0 -1
0 2 1 1 3 -1
0 0 -1
5
-1
0 1 -1
-1
0
2 3 2 -1
-1
-1

;; initial state
0 -1

;; goal
3 -1

;; tasks (primitive and abstract)
5
0 a0
0 a1
0 a2
1 A
1 B

;; initial abstract task
4

;; methods
2
m-B
4
3 0 -1
0 1 -1
m-A
3
1 2 0 -1
0 1 1 2 0 2 -1
"#;

    fn fixture() -> Model {
        from_str(FIXTURE).expect("fixture should parse")
    }

    #[test]
    fn facts_actions_and_goal() {
        let m = fixture();
        assert_eq!(m.num_state_bits, 4);
        assert_eq!(m.fact_names, ["f0", "f1", "f2", "f3"]);
        assert_eq!(m.s0, [0]);
        assert_eq!(m.goal, [3]);
        assert_eq!(m.num_actions, 3);
        assert_eq!(m.actions[0].prec, [0]);
        assert_eq!(m.actions[0].add, [2]);
        assert_eq!(m.actions[0].del, [0]);
        assert_eq!(m.actions[2].prec, [2, 3]);
        assert!(m.has_conditional_effects);
    }

    #[test]
    fn tasks_and_methods() {
        let m = fixture();
        assert!(m.is_htn);
        assert_eq!(m.num_tasks, 5);
        assert_eq!(m.task_names, ["a0", "a1", "a2", "A", "B"]);
        assert_eq!(m.initial_task, 4);
        assert_eq!(m.methods[0].name, "m-B");
        assert_eq!(m.methods[0].task, 4);
        assert_eq!(m.methods_for[4], [0]);
        assert_eq!(m.methods_for[3], [1]);
    }

    #[test]
    fn a_transitive_ordering_edge_still_gives_the_sequence() {
        let m = fixture();
        assert!(m.is_totally_ordered);
        assert_eq!(m.methods[0].subtasks, [3, 0]);
        assert_eq!(m.methods[1].subtasks, [1, 2, 0]);
    }

    #[test]
    fn an_unordered_method_is_not_a_chain() {
        let m = from_str(&FIXTURE.replace("0 1 1 2 0 2 -1", "-1")).unwrap();
        assert!(!m.is_totally_ordered);
        assert!(m.methods[1].subtasks.is_empty());
    }

    #[test]
    fn a_cyclic_ordering_is_not_a_chain() {
        let m = from_str(&FIXTURE.replace("0 1 1 2 0 2 -1", "0 1 1 2 2 0 -1")).unwrap();
        assert!(!m.is_totally_ordered);
    }

    #[test]
    fn an_empty_method_is_rejected() {
        let e = from_str(&FIXTURE.replace("1 2 0 -1\n0 1 1 2 0 2 -1", "-1\n-1")).unwrap_err();
        assert!(e.message.contains("no subtasks"), "{e}");
    }

    #[test]
    fn an_excess_task_is_detected() {
        let e = from_str(&FIXTURE.replace("1 B\n", "1 B\n1 C\n")).unwrap_err();
        assert!(e.message.contains("excess task"), "{e}");
    }

    #[test]
    fn a_classical_problem_has_no_hierarchy() {
        let cut = FIXTURE.split(";; initial abstract task").next().unwrap();
        let m = from_str(cut).unwrap();
        assert!(!m.is_htn);
        assert_eq!(m.num_actions, 3);
        assert!(m.methods.is_empty());
    }
}
