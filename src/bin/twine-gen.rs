use std::path::PathBuf;
use std::process::ExitCode;

const USAGE: &str = "\
usage: cargo weave-gen <problem.sas> [--out FILE] [--rounds N] [--linear] [--schedule B,B,...]
                       [--count-all]

  --schedule     the depth bounds to try, in order (default 64,256,1024)
  --rounds N     instead, N bounds 1, 2, 4, ...
  --linear       with --rounds, bounds 1, 2, 3, ...
  --count-all    every abstract task spends depth, not only those on a cycle

Rewrites twine/src/problem.rs from a grounded, totally ordered problem. The next build of the
crate is the search.
";

fn main() -> ExitCode {
    let mut input: Option<PathBuf> = None;
    let mut out: PathBuf = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/problem.rs");
    let mut options = twine::gen::Options::default();

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
            "--rounds" => match args.next().map(|v| v.parse()) {
                Some(Ok(v)) => {
                    options.rounds = v;
                    options.schedule = None;
                }
                _ => return fail("--rounds needs a number"),
            },
            "--linear" => {
                options.linear = true;
                options.schedule = None;
            }
            "--schedule" => {
                let bounds = args.next().map(|v| {
                    v.split(',').map(|b| b.trim().parse::<u64>()).collect::<Result<Vec<_>, _>>()
                });
                match bounds {
                    Some(Ok(b)) if !b.is_empty() && !b.contains(&0) => options.schedule = Some(b),
                    _ => return fail("--schedule needs a list of positive bounds, like 64,256"),
                }
            }
            "--count-all" => options.count_all = true,
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

    // `weave` reads the model back at run time, from a path relative to this crate when the
    // model is inside it and an absolute one otherwise.
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let absolute = std::fs::canonicalize(&input).unwrap_or_else(|_| input.clone());
    let source = absolute.strip_prefix(manifest).unwrap_or(&absolute).to_string_lossy();
    let text = match twine::gen::generate(&model, &source, &options) {
        Ok(t) => t,
        Err(e) => return fail(&format!("{}: {e}", input.display())),
    };
    if let Err(e) = std::fs::write(&out, text) {
        return fail(&format!("{}: {e}", out.display()));
    }

    eprintln!(
        "- Wrote {} from {} ({} state bits, {} actions, {} tasks, {} methods, bounds {:?})",
        out.display(),
        input.display(),
        model.num_state_bits,
        model.num_actions,
        model.num_tasks,
        model.methods.len(),
        options.bounds()
    );
    eprintln!("- Now run `cargo weave`; the search happens while it builds");
    ExitCode::SUCCESS
}

fn fail(message: &str) -> ExitCode {
    eprintln!("twine-gen: {message}\n{USAGE}");
    ExitCode::FAILURE
}
