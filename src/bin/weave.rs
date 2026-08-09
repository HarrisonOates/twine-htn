//! The search runs while this binary is compiled: naming `Woven` is what makes the trait solver
//! plan. If the problem has no solution within its depth bound, this file does not build.

#![recursion_limit = "4096"]

use std::path::PathBuf;
use std::process::ExitCode;

use twine::domain::Domain;
use twine::{plan, Prob};

type Woven = twine::search::Trace<Prob>;

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

    if show_model {
        eprintln!(
            "- {} state bits, {} actions, {} tasks, {} methods",
            Prob::NUM_FACTS,
            Prob::ACTIONS.len(),
            Prob::TASKS.len(),
            Prob::METHODS.len()
        );
        eprintln!("- The initial task is \"{}\"", Prob::TASKS[Prob::ROOT as usize]);
        eprintln!("- The initial state contains {} set bits", Prob::INIT.len());
        eprintln!("- State-based goal contains {} bits", Prob::GOAL.len());
    }

    if let Some(path) = verify {
        return match load(&path) {
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
    let derivation = match plan::from_trace::<Prob>(&events) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("weave: the compiled-in trace is not a derivation: {e}");
            return ExitCode::FAILURE;
        }
    };
    let length = match plan::check::<Prob>(&derivation) {
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

    let text = plan::format::<Prob>(&derivation);
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

fn load(path: &PathBuf) -> Result<usize, String> {
    let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let derivation = plan::parse::<Prob>(&text)?;
    plan::check::<Prob>(&derivation)
}
