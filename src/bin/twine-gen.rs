use std::path::PathBuf;
use std::process::ExitCode;

const USAGE: &str = "\
usage: cargo weave-gen <problem.sas> [--out FILE] [--depth N]

Rewrites twine/src/problem.rs from a grounded, totally ordered problem. The next build of the
crate is the search.
";

fn main() -> ExitCode {
    let mut input: Option<PathBuf> = None;
    let mut out: PathBuf = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/problem.rs");
    let mut depth = 20usize;

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                print!("{USAGE}");
                return ExitCode::SUCCESS;
            }
            "--out" => match args.next() {
                Some(v) => out = v.into(),
                None => return fail("--out needs a path"),
            },
            "--depth" => match args.next().map(|v| v.parse()) {
                Some(Ok(v)) => depth = v,
                _ => return fail("--depth needs a number"),
            },
            other if other.starts_with('-') => return fail(&format!("unknown argument {other}")),
            other => input = Some(other.into()),
        }
    }
    let Some(input) = input else {
        return fail("no input problem");
    };

    let model = match twine::sas::from_file(&input) {
        Ok(m) => m,
        Err(e) => return fail(&format!("{}: {e}", input.display())),
    };

    let source = input.file_name().unwrap_or(input.as_os_str()).to_string_lossy();
    let text = match twine::gen::generate(&model, &source, depth) {
        Ok(t) => t,
        Err(e) => return fail(&format!("{}: {e}", input.display())),
    };
    if let Err(e) = std::fs::write(&out, text) {
        return fail(&format!("{}: {e}", out.display()));
    }

    eprintln!(
        "- Wrote {} from {} ({} state bits, {} actions, {} tasks, {} methods, depth bound {depth})",
        out.display(),
        input.display(),
        model.num_state_bits,
        model.num_actions,
        model.num_tasks,
        model.methods.len()
    );
    eprintln!("- Now run `cargo weave`; the search happens while it builds");
    ExitCode::SUCCESS
}

fn fail(message: &str) -> ExitCode {
    eprintln!("twine-gen: {message}\n{USAGE}");
    ExitCode::FAILURE
}
