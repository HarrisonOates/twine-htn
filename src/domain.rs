/// The run-time shadow of the type-level model: the same grounded problem as tables, so a plan
/// found by the trait solver can name itself, and a plan read back from disk can be checked.
pub struct ActionInfo {
    pub name: &'static str,
    pub prec: &'static [u32],
    pub add: &'static [u32],
    pub del: &'static [u32],
}

pub struct MethodInfo {
    pub name: &'static str,
    pub task: u32,
    pub subtasks: &'static [u32],
}

pub trait Domain {
    const NUM_FACTS: usize;
    const FACTS: &'static [&'static str];
    const TASKS: &'static [&'static str];
    const ACTIONS: &'static [ActionInfo];
    const METHODS: &'static [MethodInfo];
    const INIT: &'static [u32];
    const GOAL: &'static [u32];
    const ROOT: u32;

    fn is_action(task: u32) -> bool {
        (task as usize) < Self::ACTIONS.len()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    Action(u32),
    Method(u32),
}

pub trait Reify {
    fn emit(out: &mut Vec<Event>);
}

pub fn trace<T: Reify>() -> Vec<Event> {
    let mut out = Vec::new();
    T::emit(&mut out);
    out
}
