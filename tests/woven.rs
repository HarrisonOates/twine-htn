#![recursion_limit = "4096"]

use twine::domain::{trace, Domain};
use twine::search::Trace;
use twine::{plan, Prob};

type Woven = Trace<Prob>;

fn derivation() -> plan::Derivation {
    plan::from_trace::<Prob>(&trace::<Woven>()).expect("the trace is a derivation")
}

#[test]
fn the_trait_solver_returns_an_executable_plan() {
    let length = plan::check::<Prob>(&derivation()).expect("the compile-time plan verifies");
    assert!(length > 0);
}

#[test]
fn a_saved_plan_reads_back_as_the_same_derivation() {
    let woven = derivation();
    let text = plan::format::<Prob>(&woven);
    let read = plan::parse::<Prob>(&text).expect("the plan parses");

    assert_eq!(read.root, woven.root);
    assert_eq!(read.nodes.len(), woven.nodes.len());
    let tasks = |d: &plan::Derivation| d.leaves().map(|(_, t)| t).collect::<Vec<_>>();
    assert_eq!(tasks(&read), tasks(&woven));
    assert_eq!(plan::format::<Prob>(&read), text);
    plan::check::<Prob>(&read).expect("the plan that was read back verifies");
}

#[test]
fn a_plan_whose_actions_are_reordered_is_rejected() {
    let text = plan::format::<Prob>(&derivation());
    let mut lines: Vec<&str> = text.lines().collect();
    let last = lines.iter().position(|l| l.starts_with("root ")).unwrap() - 1;
    lines.swap(1, last);
    let error = plan::parse::<Prob>(&lines.join("\n")).unwrap_err();
    assert!(error.contains("execution order"), "{error}");
}

#[test]
fn a_plan_with_a_method_that_does_not_fit_is_rejected() {
    let text = plan::format::<Prob>(&derivation());
    let other = Prob::METHODS
        .iter()
        .find(|m| m.name != Prob::METHODS[0].name && m.task != Prob::ROOT)
        .expect("the model has more than one method");
    let broken =
        text.replace(&format!("-> {}", Prob::METHODS[0].name), &format!("-> {}", other.name));
    assert!(plan::parse::<Prob>(&broken).is_err());
}
