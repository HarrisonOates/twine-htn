# twine

A totally ordered HTN planner that runs inside the Rust trait solver. The grounded planning
problem is a set of marker types; progression search is impl selection; the plan is an associated
type. `rustc` does the planning, and the binary in this crate only reads the answer back out and
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
planning happens on the next build.

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

`E` is unbounded in the first impl, so the untaken branch is never normalised. Every recursive
step in the planner goes through this.

**The state is a type-level bit vector.** `Cons<B1, Cons<B0, ...>>`, one bit per state bit of the
grounded model, and preconditions, add lists and delete lists have the same shape. A precondition
test is a zip (`Holds`), and progression is a zip (`Apply` = `(state \ del) u add`). No type-level
integers and no equality test on types are needed anywhere.

**Tasks and methods are marker types.** `T7` implements `Task`; its `Kind` is `Prim<T7>` when the
task is primitive and `Comp<tlist![M3, M9]>` when it is abstract, so the primitive/abstract test
and the method lookup are both impl selection. A `Prim` carries an `Action` impl with the three
masks; a method carries `type Subtasks`, its subtask markers in execution order.

**The network is a list, and search is a fold over it.** For a totally ordered problem the network
is a chain, so `Solve<P, State, Network, Depth, Trace>` needs no more than the list of remaining
tasks. Progressing an action pops the head; applying a method prepends its subtasks. Backtracking
is one trait:

```rust
impl<Trace, Alt> OrElse<Alt> for Found<Trace> { type Out = Found<Trace>; }
impl<Alt: Eval> OrElse<Alt> for Stuck { type Out = Alt::Out; }
```

`Try` walks a task's methods, and each one's alternative is the rest of the list as an unevaluated
thunk. `Found` short-circuits: the remaining methods are never normalised, which is what makes
this depth-first search and not exhaustive enumeration.

**The bound is a Peano numeral,** decremented on each method application, so it counts
decompositions along a path. `Deepen` restarts at successive bounds, and it is what stops a cyclic
decomposition from following one recursive method to the bottom every time.

**The plan is the trace.** Each step conses `Do<A>` or `Use<M>`, and reaching an empty network
with the goal satisfied reverses the accumulator into `Found<...>`. Because progression works on
the front of a chain, that list is exactly a preorder walk of the decomposition tree, so the tree
rebuilds from method arities alone. `Reify` walks the list at run time and turns each marker back
into its index in the grounded model, `plan::from_trace` rebuilds the tree, and `plan::format`
writes the IPC plan that `pandaPIparser -c` reads.

**Unsolvable does not compile.** Only `Found` implements `Solved`, so naming `Trace<Prob>` for a
problem with no solution inside its depth bound is a type error:

```
error[E0277]: the trait bound `Stuck: Solved` is not satisfied
```

## Getting a problem in

twine reads the `.sas` files that [pandaPIgrounder](https://github.com/panda-planner-dev) produces:
run `pandaPIparser domain.hddl problem.hddl out.htn` and then `pandaPIgrounder out.htn out.sas`.
`src/sas.rs` is a small positional reader for that format, cut down to what the generator needs.

`cargo weave-gen <problem.sas> [--depth N]` writes `src/problem.rs`: one marker type per task and
per method, the masks, the `Problem` impl for the type-level search, and a `Domain` impl carrying
the same model as run-time tables, so a plan can name itself and be checked. It refuses partially
ordered problems and conditional effects.

`cargo weave --verify FILE` checks a plan against the compiled-in problem the way a verifier does:
every decomposition legal, the leaves the actions in the order given, executable from the initial
state, and the goal reached at the end. `scripts/weave.sh domain.hddl problem.hddl` runs the whole
chain and ends in `pandaPIparser -v`.

## What it can do

Times are `cargo weave` from a cold `src/problem.rs`, on one desktop core. Every plan was converted
with `pandaPIparser -c` and verified against the original HDDL.

| Instance | State bits | Tasks | Plan | Depth bound | rustc |
|---|---|---|---|---|---|
| `Robot/pfile_02_001` | 9 | 19 | 10 actions, 13 decompositions | 20 | 1.9 s |
| `Robot/pfile_02_002` | 11 | 20 | 10 actions, 15 decompositions | 18 | 2.1 s |
| `Towers/pfile_01` | 15 | 86 | 4 actions | 10 | 0.8 s |
| `Transport/pfile01` | 11 | 19 | 8 actions | 12 | 0.7 s |
| `Satellite-GTOHP/p01` | 16 | 50 | 21 actions | 14 | 1.2 s |
| `Blocksworld-GTOHP/p01` | 30 | 62 | 39 actions, 20 decompositions | 24 | 2.9 s |
| `Depots/p01` | 37 | 100 | 25 actions | 16 | 3.4 s |
| `Factories-simple/pfile01` | 17 | 47 | 31 actions, 25 decompositions | 28 | 24 s |
| `Barman-BDI/pfile01` | 45 | 165 | — | 40 | OOM at ~2 min |

The depth bound is the cost driver, not the size of the model: iterative deepening repeats the
whole search once per round, and every intermediate state is a live type in the solver's cache.
Barman needs a bound of 40 and `rustc` is killed before it gets there.

Blind depth-first search optimises nothing — there is no heuristic, no duplicate detection, and no
partial order, since the type-level network is a chain by construction. If you want a planner that
is actually good, this is not it; this is the one where the compiler does the searching.

`examples/robot_02_001.sas` is the grounder's output for `Robot/pfile_02_001` of the
[IPC 2023](https://github.com/ipc2023-htn) total-order set, checked in so the generator can be run
without the toolchain.
