# twine

A totally ordered HTN planner that runs inside the Rust trait solver. The grounded planning
problem is a set of marker types; search is impl selection; the plan is an associated type.
`rustc` does the planning, and the binary in this crate only reads the answer back out and
prints it.

It plans real benchmarks — the IPC 2023 total-order set — and its plans verify with
`pandaPIparser`.

```sh
cargo weave                                 # print the plan for the compiled-in problem
cargo weave --save plan.txt                 # save it in the IPC plan format
cargo weave --verify plan.txt               # read one back and check it
cargo weave-gen examples/robot_02_001.sas   # point it at another grounded problem
```

Compiling `cargo weave` *is* the search. `cargo weave-gen` only rewrites `src/problem.rs`; the
planning happens on the next build. It type-checks if and only if the search finds a plan, and
the plan is read off the type.

The search needs rustc's next-generation trait solver. `.cargo/config.toml` turns it on
(`-Znext-solver=globally`, with `RUSTC_BOOTSTRAP=1` so a stable toolchain takes the flag); see
[why](#why-the-next-solver).

## The encoding

**Everything is lazy.** Trait resolution has no backtracking of its own — impls may not overlap,
so the choice of impl is a function of the types. Search is therefore written as an ordinary
deterministic evaluator over a call-by-need calculus that the solver normalises. A suspended
computation is a struct that implements `Eval`, and a branch runs only when an impl demands its
`Out`:

```rust
impl<T: Eval, E> Choose<T, E> for B1 { type Out = T::Out; }
impl<T, E: Eval> Choose<T, E> for B0 { type Out = E::Out; }
```

`E` is unbounded in the first impl, so the untaken branch is never normalised.

**The cache is the memo table.** The trait solver remembers what every goal normalised to. So
the search never asks "what should I try next?", only "what are the outcomes of this task from
this state?", and it gets each answer once. `Run<T, S, D>` is the lazy stream of every distinct
state task `T` can end in when started in state `S` with depth budget `D`, each one paired with
the plan that gets there:

```rust
pub struct Hit<State, Plan, More>;   // one outcome, then the rest of the stream, suspended
pub struct Miss;                     // no more
```

Nothing about the caller is in `Run<T, S, D>`. Wherever the search meets the same task in the same
state with the same budget — in another branch, in a later deepening round, at the other end of a
cycle — `rustc` reads the stream it already built. Blind depth-first search becomes dynamic
programming over (task, state, budget), and nobody had to write a hash table.

**Sequences are binds, methods are concatenations.** A method's subtasks are a stream of streams:
each outcome of the first subtask, followed by every outcome of the rest from there (`Chain`,
`Bind`). A task's stream is its methods' streams one after another (`Methods`), each outcome
tagged `Dec<M, _>`. Plans are trees glued with `Then<_, _>`, so composing two is one node.

**Duplicates are dropped.** What follows a task depends only on the state it ends in, so the
first plan to reach a state is as good as any other. `Dedup` filters every task's stream, and
every sequence of two or more, against the states it has already produced, so a stream is as long
as the number of states it can end in, not the number of paths there. Without this, a task like
"drive the cars to the start, in any order" has factorially many outcomes.

**The state is a sparse binary trie.** A state, a precondition, an add list and a delete list are
all the same shape: a complete binary trie over the state bits, with every all-zero subtree
collapsed to `Off`. The collapsed form is canonical, so equality of types is equality of states,
and every operation stops at the first `Off` in its mask. A precondition test or an update costs
the bits it mentions times the height of the trie, and the new state shares every untouched
subtree with the old one.

**The depth bound only where it is needed.** The generator computes the strongly connected
components of the decomposition graph. A task on a cycle (`Comp`) spends a level of the budget
per decomposition. A task that can reach a cycle but is not on one (`Pass`) passes it on. A task
with no cycle below it (`Flat`) is searched at budget zero, so its streams are the same types in
every round of deepening. Budgets are binary numerals, since one sits in the key of every task the
search meets.

**Deepening starts generous.** The schedule is a type-level list of bounds, 64, 256 and 1024 by
default. Depth-first search under a generous bound tends to dive straight down a plan, and the
cache makes a bound that is larger than needed cheap; deepening from 1 would exhaust every
shallow round first. `scripts/ipc.sh` runs a short pass of deepening 1, 2, 4, … before it, because
that is fastest on easy problems.

**Methods are tried cheapest first.** The generator orders each task's methods by the fewest
actions any decomposition of their subtasks can bottom out in, and drops methods that cannot
bottom out at all. On a recursive task this tries the way out before the way further in. It is
the only heuristic, and it is static.

**The plan is the type.** The search reaches `Found<Plan>`. `Reify` walks the plan tree at run
time, in preorder, and turns each marker back into its index in the grounded model;
`plan::from_trace` rebuilds the derivation from method arities alone, and `plan::format` writes
the IPC plan that `pandaPIparser -c` reads.

**No plan does not compile.** Only `Found` implements `Solved`, so naming `Trace<Prob>` for a
problem with no plan within the schedule is a type error:

```
error[E0277]: the trait bound `Stuck: Solved` is not satisfied
```

## What makes the trait solver slow

`rustc` costs about a millisecond per element pulled from a stream, so the encoding is shaped by
what the solver charges for:

- **Every goal walks its types.** Matching an impl copies each argument into a fresh inference
  variable, and that walks the whole argument. Goals stay small: no lists of methods (each task's
  methods are a chain of named cells, `Alts`), no Peano numerals, no long lists in keys.
- **Every impl is one struct.** Each computation has exactly one `Eval` impl, which hands off to a
  trait on the thing it matches. Matching on a parameter instead makes `rustc`, checking the
  generic impls, enumerate candidates for an unknown type and recurse until it overflows.
- **Coherence is pairwise.** Tasks are separate structs, `T0`, `T1`, …, each with one `Task` impl.
  As `T<const N: u32>` the impls share a self type, and coherence compares all of them pairwise: on
  Monroe, with 67 000 tasks, that took 488 s before the search began.
- **Masks are shared.** Every interior node of every mask is emitted once, as a type alias. On
  Monroe that halved the source and the memory rustc needs to parse it.
- **The names are not compiled in.** `weave` reads the `.sas` file back at run time for names and
  to check its own plan, so the generated program carries only what the search needs.

## Why the next solver

The stable solver has no global cache for these goals. On `Blocksworld-HPDDL/pfile_010` it used
12.7 GB and was killed; the next solver finds the plan in 20 s using 600 MB. The search also
recurses deeply inside the solver, so `.cargo/config.toml` gives rustc a 4 GB stack
(`RUST_MIN_STACK`); only the pages it touches are allocated.

## Getting a problem in

twine reads the `.sas` files that [pandaPIgrounder](https://github.com/panda-planner-dev) produces:
run `pandaPIparser domain.hddl problem.hddl out.htn` and then `pandaPIgrounder out.htn out.sas`.
`src/sas.rs` is a small positional reader for that format, cut down to what the generator needs.

`cargo weave-gen <problem.sas> [--schedule 64,256,1024] [--rounds N] [--count-all]` writes
`src/problem.rs`: one marker type per task, a chain of cells per task's methods, the shared masks,
and the `Problem` impl for the type-level search. It refuses partially ordered problems and
conditional effects. `--rounds N` deepens 1, 2, 4, …; `--count-all` makes every abstract task
spend depth, not just those on a cycle.

`cargo weave --verify FILE` checks a plan against the grounded model the way a verifier does:
every decomposition legal, the leaves the actions in the order given, executable from the initial
state, and the goal reached at the end. `scripts/weave.sh domain.hddl problem.hddl` runs the whole
chain and ends in `pandaPIparser -v`.

## The IPC 2027 entry

`twine.def` builds an Apptainer image for the total-order non-optimal track: pandaPIparser and
pandaPIgrounder from source, a pinned Rust 1.99.0, and the twine library prebuilt with the flags
the search uses, so a run compiles only the `weave` binary.

```sh
apptainer build twine.sif twine.def
apptainer run twine.sif domain.hddl problem.hddl plan.txt
```

`scripts/ipc.sh` is the entry point: parse, ground, then the two-phase portfolio — deepening
1, 2, 4, … for 30 s, then the default schedule for the rest of the time, each a fresh `rustc` —
and finally `pandaPIparser -c` into the IPC format.

## What it can do

The first three problems of each domain of the IPC 2023 total-order set, run as the competition
runs them: `scripts/ipc.sh` from HDDL to IPC plan, one problem at a time on one desktop core,
8 GB and 30 minutes. "Slowest" is wall time for the whole pipeline, parse to verified plan. Every
plan was checked by `pandaPIparser -v` against the original HDDL.

| Domain | Solved | Slowest |
|---|---|---|
| Barman-BDI, Blocksworld-GTOHP, Depots, Logistics-Learned, Minecraft-Regular, Robot, Rover-GTOHP, Satellite-GTOHP | 3 / 3 each | 4 s |
| Factories-simple | 3 / 3 | 4 s |
| Multiarm-Blocksworld | 3 / 3 | 67 s |
| Hiking | 3 / 3 | 90 s |
| Blocksworld-HPDDL | 3 / 3 | 191 s |
| AssemblyHierarchical | 2 / 3 | 15 s |
| Snake | 2 / 2 run | 38 s |
| Minecraft-Player | 1 / 3 | 32 s |
| Monroe-Fully-Observable | 1 / 1 run | 30 s |
| Monroe-Partially-Observable | 0 / 1 run | — |
| Freecell-Learned | 0 / 3 | — |

That is 42 plans from the runs that finished. Towers, Transport and Woodworking (all solved in
a few seconds by earlier builds of this engine), Snake `pb-4slots` and three Monroe problems are
still to be run. The failures are memory: Freecell needs tens of millions of search steps even outside
the compiler, and the largest Minecraft-Player problem does not get through the grounder.

The version of twine before this engine — one search node per type, the trace in every type,
iterative deepening over all decompositions — solved 6 of the first 18 of these problems, and ran
out of memory or out of a 5-minute limit on the rest.

Blind search optimises nothing: there is no heuristic beyond the method order, and the plans are
often long, since a generous depth bound lets depth-first search wander before it finds the way
out. If you want a planner that is actually good, this is not it; this is the one where the
compiler does the searching.

`examples/robot_02_001.sas` is the grounder's output for `Robot/pfile_02_001` of the
[IPC 2023](https://github.com/ipc2023-htn) total-order set, checked in so the generator can be run
without the toolchain.
