#![recursion_limit = "4096"]

#[rustfmt::skip]
#[path = "../src/problem.rs"]
mod problem;

use std::path::Path;

use twine::domain::trace;
use twine::plan;
use twine::sas::Model;
use twine::search::Trace;

type Woven = Trace<problem::Prob>;

fn model() -> Model {
    twine::sas::from_file(Path::new(env!("CARGO_MANIFEST_DIR")).join(problem::SOURCE))
        .expect("the grounded model the problem was generated from reads back")
}

fn derivation(m: &Model) -> plan::Derivation {
    plan::from_trace(m, &trace::<Woven>()).expect("the trace is a derivation")
}

#[test]
fn the_trait_solver_returns_an_executable_plan() {
    let m = model();
    let length = plan::check(&m, &derivation(&m)).expect("the compile-time plan verifies");
    assert!(length > 0);
}

#[test]
fn a_saved_plan_reads_back_as_the_same_derivation() {
    let m = model();
    let woven = derivation(&m);
    let text = plan::format(&m, &woven);
    let read = plan::parse(&m, &text).expect("the plan parses");

    assert_eq!(read.root, woven.root);
    assert_eq!(read.nodes.len(), woven.nodes.len());
    let tasks = |d: &plan::Derivation| d.leaves().map(|(_, t)| t).collect::<Vec<_>>();
    assert_eq!(tasks(&read), tasks(&woven));
    assert_eq!(plan::format(&m, &read), text);
    plan::check(&m, &read).expect("the plan that was read back verifies");
}

#[test]
fn a_plan_whose_actions_are_reordered_is_rejected() {
    let m = model();
    let text = plan::format(&m, &derivation(&m));
    let mut lines: Vec<&str> = text.lines().collect();
    let last = lines.iter().position(|l| l.starts_with("root ")).unwrap() - 1;
    lines.swap(1, last);
    let error = plan::parse(&m, &lines.join("\n")).unwrap_err();
    assert!(error.contains("execution order"), "{error}");
}

#[test]
fn a_plan_with_a_method_that_does_not_fit_is_rejected() {
    let m = model();
    let text = plan::format(&m, &derivation(&m));
    let first = &m.methods[0];
    let other = m
        .methods
        .iter()
        .find(|o| o.name != first.name && o.task != m.initial_task)
        .expect("the model has more than one method");
    let broken = text.replace(&format!("-> {}", first.name), &format!("-> {}", other.name));
    assert!(plan::parse(&m, &broken).is_err());
}
