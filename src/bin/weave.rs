//! The search runs while this binary is compiled: naming `Woven` is what makes the trait solver
//! plan. If the problem has no solution within its depth bound, this file does not build.

#![recursion_limit = "4096"]

#[rustfmt::skip]
#[path = "../problem.rs"]
mod problem;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use twine::plan;
use twine::sas::Model;

type Woven = twine::search::Trace<problem::Prob>;

const USAGE: &str = "\
usage: cargo weave [--save FILE] [--verify FILE] [--model]

  (no arguments)   print the plan the trait solver found
  --save FILE      also write it to FILE, in the IPC format pandaPIparser -c reads
  --verify FILE    read a plan back and check it against the compiled-in problem
  --model          print what the compiled-in problem is
";

fn main() -> ExitCode {
    let mut save: Option<PathBuf> = None;
    let mut verify: Option<PathBuf> = None;
    let mut show_model = false;

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        let mut value = || args.next().ok_or_else(|| format!("{arg} needs a path"));
        let parsed = match arg.as_str() {
            "--save" => value().map(|v| save = Some(v.into())),
            "--verify" | "--load" => value().map(|v| verify = Some(v.into())),
            "--model" => {
                show_model = true;
                Ok(())
            }
            "-h" | "--help" => {
                print!("{USAGE}");
                return ExitCode::SUCCESS;
            }
            other => Err(format!("unknown argument {other}")),
        };
        if let Err(e) = parsed {
            eprintln!("weave: {e}\n{USAGE}");
            return ExitCode::FAILURE;
        }
    }

    // The names the plan prints live in the grounded model, not in the compiled-in program.
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join(problem::SOURCE);
    let model = match twine::sas::from_file(&source) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("weave: {}: {e}", source.display());
            return ExitCode::FAILURE;
        }
    };

    if show_model {
        eprintln!(
            "- {} state bits, {} actions, {} tasks, {} methods",
            model.num_state_bits,
            model.num_actions,
            model.num_tasks,
            model.methods.len()
        );
        eprintln!("- The initial task is \"{}\"", model.task_names[model.initial_task as usize]);
        eprintln!("- The initial state contains {} set bits", model.s0.len());
        eprintln!("- State-based goal contains {} bits", model.goal.len());
    }

    if let Some(path) = verify {
        return match load(&model, &path) {
            Ok(length) => {
                eprintln!("- Read a plan of {length} actions from {}", path.display());
                eprintln!("- PLAN ACCEPTED");
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("weave: {}: {e}", path.display());
                eprintln!("- PLAN REJECTED");
                ExitCode::FAILURE
            }
        };
    }

    let events = twine::domain::trace::<Woven>();
    let derivation = match plan::from_trace(&model, &events) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("weave: the compiled-in trace is not a derivation: {e}");
            return ExitCode::FAILURE;
        }
    };
    let length = match plan::check(&model, &derivation) {
        Ok(length) => length,
        Err(e) => {
            eprintln!("weave: the trait solver returned a plan that does not verify: {e}");
            return ExitCode::FAILURE;
        }
    };

    eprintln!("- The search ran in rustc; this binary only formats the answer");
    eprintln!(
        "- Found a solution of {length} actions from {} decompositions",
        derivation.nodes.len() - length
    );

    let text = plan::format(&model, &derivation);
    println!("{text}");

    if let Some(path) = save {
        if let Err(e) = std::fs::write(&path, format!("{text}\n")) {
            eprintln!("weave: {}: {e}", path.display());
            return ExitCode::FAILURE;
        }
        eprintln!("- Saved to {}", path.display());
    }
    ExitCode::SUCCESS
}

fn load(model: &Model, path: &Path) -> Result<usize, String> {
    let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let derivation = plan::parse(model, &text)?;
    plan::check(model, &derivation)
}
