# twine

**Planning by compilation. By compilation.**

Compilation-based planners translate a planning problem into another formalism (e.g SAT) and hand it to an external solver.
twine takes the terminology rather more seriously: it translates a totally ordered Hierarchical Task Network (HTN) problem into Rust types and asks `rustc` to compile them.

Trait resolution executes the progression search; associated-type normalisation computes the plan. The program cannot compile until its types reduce to a valid plan, leaving the resulting executable with the undemanding task of printing the plan.

If bounded search fails, the planner reports this through the compiler's native diagnostic:

```text
error[E0277]: the trait bound `Stuck: Solved` is not satisfied
```

## Quick Start

Requires Rust and Cargo. twine enables the next-generation trait solver (`-Znext-solver=globally`) in `.cargo/config.toml`.

The repository includes a grounded Robot benchmark ready out of the box:

```bash
cargo weave                                 # Compile (search) and print the plan
cargo weave --save plan.txt                 # Save an IPC-format plan
cargo weave --verify plan.txt               # Validate against the grounded problem
cargo weave-gen examples/robot_02_001.sas   # Regenerate the example's Rust types
```

`cargo weave-gen` generates `src/problem.rs`. The subsequent `cargo weave` performs the search during compilation. Running the binary again simply prints the answer; provided nothing has changed, Cargo's build cache saves us from asking the compiler to solve the same problem twice.

To run custom HDDL problems, ground them using `pandaPIparser` and `pandaPIgrounder`, then point `cargo weave-gen` at the resulting `.sas` file.

## Encoding Search into Types

twine encodes an HTN progression search entirely as type-level computations:

| Planning concept | Rust representation | Mechanism |
|---|---|---|
| **State** | `Br<L, R>`, `On`, `Off` | Canonical sparse binary trie; identical states have identical types |
| **Action** | `Prim<Id<N>, Program>` | Preconditions and effects combined into a trie walk that rewrites state types |
| **Decomposition** | `Alt<MethodId, Subtasks, Next>` | Chains of alternative decomposition methods |
| **Search call** | `Run<Task, State, Depth>` | Lazy stream of outcomes: `Hit<State, Plan, More>` or `Miss` |
| **Plan** | `Do`, `Dec`, `Then` | Type-level derivation tree, reified into IPC output at runtime |

### Deterministic evaluation and compiler memoisation

Rust trait resolution does not backtrack between overlapping implementations, so we cannot simply ask the compiler to choose a method. Instead, twine implements a deterministic, call-by-need evaluator at the type level: branches are suspended computations implementing `Eval`, normalised only when their results are demanded.

Because `Run<Task, State, Depth>` carries no caller context, identical (task, state, budget) queries produce identical types across branches and deepening rounds. Rust's next-generation trait solver [caches goal evaluations globally](https://rustc-dev-guide.rust-lang.org/solve/caching.html), allowing repeated computations to be reused within a compilation.

A more in-depth description of how the translation into types works is in progress.
