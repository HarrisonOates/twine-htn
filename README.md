# twine

**Planning by compilation. By compilation.**

Compilation-based planning translates a planning problem into another formalism and hands it
to a solver. Often, that solver is not even a compiler. A disappointing commitment to the bit.

twine takes the name literally: compile a totally ordered hierarchical task network (HTN)
problem into Rust types, then ask Rust to compile them. Type checking evaluates the search;
the resulting plan is an associated type. The compiler has to find a plan before it can build
the executable that prints it. The executable's job is to explain why it was allowed to exist.

It plans real benchmarks — the IPC 2023 total-order set — and its plans verify with
`pandaPIparser`. This has gone somewhat further than the joke required.

## Make the compiler do it

With Rust and Cargo installed, run these from the repository root. The checked-in Robot
example is ready to go; the parser and grounder are only needed to bring your own HDDL.

```sh
cargo weave                                 # print the plan for the compiled-in problem
cargo weave --save plan.txt                 # save it in the IPC plan format
cargo weave --verify plan.txt               # read one back and check it
cargo weave-gen examples/robot_02_001.sas    # regenerate the included problem's types
```

Compiling `cargo weave` *is* the search. `cargo weave-gen` only rewrites `src/problem.rs`; the
planning happens on the next build. A successful build contains the plan; running it reads the
answer back out, checks it against the grounded model, and prints it. Subsequent runs do not
repeat the search. Cargo's build cache is, technically, also a plan cache.

The search needs rustc's next-generation trait solver. `.cargo/config.toml` turns it on
(`-Znext-solver=globally`, with `RUSTC_BOOTSTRAP=1` so a stable toolchain takes the flag); see
[why](#why-the-next-solver).

## How to put a planning problem in a type

An HTN problem describes actions and ways to break larger tasks into smaller ones. twine
generates marker types for those pieces; trait implementations describe what they do.
The types carry the planning data in their structure, without constructing values at run time:

| Planning ingredient | Rust encoding |
|---|---|
| A grounded task | A marker struct such as `T17`, with a `Task::Kind` associated type. |
| A state | A binary trie of `Br<Left, Right>`, `On` and `Off`. For two facts, `Br<On, Off>` means the first is true and the second is false. |
| An action | `Prim<Id<N>, Program>`, where `Program` is a trie encoding the preconditions and effects. Applying it computes a new state type. |
| A compound task's methods | A chain of named cells containing `Alt<MethodId, Subtasks, Next>`. Subtasks are type-level lists. |
| A search call | `Run<Task, State, Depth>`, whose `Eval::Out` is a lazy stream of outcomes. The depth budget is a binary numeral made of types, too. |
| A plan | A tree of `Do<Action>`, `Dec<Method, Plan>` and `Then<Left, Right>`. |

The binary asks for the answer like this:

```rust
type Woven = twine::search::Trace<problem::Prob>;
let events = twine::domain::trace::<Woven>();
```

Resolving `Woven` forces associated-type normalisation through the search until it reaches
`Found<Plan>` or exhausts the configured depth schedule as `Stuck`. Only `Found` implements
`Solved`, so bounded failure arrives in the compiler's native planning-report format:

```text
error[E0277]: the trait bound `Stuck: Solved` is not satisfied
```

That means no plan was found within the schedule. A timeout, memory limit or compiler recursion
overflow says nothing about whether a plan exists. Sometimes the fix for a type error really is
to give the robot more time to think.

## Why the next solver

The next-generation trait solver has a [global cache of goal evaluations](https://rustc-dev-guide.rust-lang.org/solve/caching.html).
twine arranges its search around repeated task/state/budget computations, so the compiler can
reuse their results within a compilation. That cache supplies the planner's memo table.
We have outsourced both the search and the hash table to the build tool.

This matters in practice: on `Blocksworld-HPDDL/pfile_010`, the old solver used 12.7 GB and was
killed; the next solver found the plan in 20 s using 600 MB. `.cargo/config.toml` also gives
rustc a 4 GB stack (`RUST_MIN_STACK`); only the pages it touches are allocated. The search runs
deep inside the solver, occasionally in several senses of the word.

<details>
<summary>The implementation details, for anyone still on board</summary>

### Teaching trait resolution to search

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

**The cache is the memo table.** The search asks "what are the outcomes of this task from
this state?", so repeated calls can reuse the same normalisation results. `Run<T, S, D>` is the lazy stream of every distinct
state task `T` can end in when started in state `S` with depth budget `D`, each one paired with
the plan that gets there:

```rust
pub struct Hit<State, Plan, More>;   // one outcome, then the rest of the stream, suspended
pub struct Miss;                     // no more
```

Nothing about the caller is in `Run<T, S, D>`. Wherever the search meets the same task in the same
state with the same budget — in another branch, in a later deepening round, at the other end of a
cycle — `rustc` can reuse the stream it already built. Blind depth-first search becomes dynamic
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
and every operation stops at the first `Off` in its mask. An action's precondition, delete list and
add list are merged into one trie whose leaves say what happens to each fact (`Need`, `NeedDel`,
`NeedAdd`, `Add`, `Del`), so firing it is a single walk that tests and rewrites together: it costs
the bits the action mentions times the height of the trie, and the new state shares every
untouched subtree with the old one.

**Depth bounds only where they are needed.** The generator computes the strongly connected
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

**Inferred preconditions prune.** A fact that some later task of a sequence needs, and that
nothing earlier in the sequence can add, must already hold when the sequence starts. The generator
computes this for every compound task (over all its refinements, as a fixpoint) and for every tail
of every method's subtasks, PANDAdealer-style. The search tests a task's (`Guard`) before looking
inside it, and the rest of a method's (`Sub`) before searching it: a single mask test that can
skip a whole subtree.

**Tasks are cached under the facts they touch.** A compound task's refinements read and write only
the facts its reachable actions mention, so its outcomes depend on nothing else. `Proj` caches its
stream under the state restricted to those facts and merges each outcome back, so states that
differ only elsewhere share one stream. On Factories `pfile05` this and the pruning together are
the difference between running out of memory after 953 s and a plan in 119 s.

**Methods are tried cheapest first.** The generator orders each task's methods by the fewest
actions any decomposition of their subtasks can bottom out in, and drops methods that cannot
bottom out at all. On a recursive task this tries the way out before the way further in. It is
the only heuristic, and it is static.

**The plan is the type.** The search reaches `Found<Plan>`. `Reify` walks the plan tree at run
time, in preorder, and turns each marker back into its index in the grounded model;
`plan::from_trace` rebuilds the derivation from method arities alone, and `plan::format` writes
the IPC plan that `pandaPIparser -c` reads.

### What makes the trait solver slow

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

</details>

## Getting a problem in

twine reads the `.sas` files that [pandaPIgrounder](https://github.com/panda-planner-dev) produces:
run `pandaPIparser domain.hddl problem.hddl out.htn` and then `pandaPIgrounder out.htn out.sas`.
`src/sas.rs` is a small positional reader for that format, cut down to what the generator needs.

`cargo weave-gen <problem.sas> [--schedule 64,256,1024] [--rounds N] [--count-all] [--no-prune]
[--no-project]` writes
`src/problem.rs`: one marker type per task, a chain of cells per task's methods, the shared masks,
and the `Problem` impl for the type-level search. It refuses partially ordered problems and
conditional effects. `--rounds N` deepens 1, 2, 4, …; `--count-all` makes every abstract task
spend depth, not just those on a cycle; `--no-prune` and `--no-project` turn off the inferred
preconditions and the projected cache.

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
Our search portfolio is two attempts to get the same binary to compile.

## What it can do

The first three problems of each domain of the IPC 2023 total-order set, run as the competition
runs them: `scripts/ipc.sh` from HDDL to IPC plan, one problem at a time on one desktop core,
8 GB and 30 minutes. "Slowest" is wall time for the whole pipeline, parse to verified plan. Every
plan was checked by `pandaPIparser -v` against the original HDDL. These numbers predate the
inferred-precondition pruning and the projected cache.

| Domain | Solved | Slowest |
|---|---|---|
| Barman-BDI, Blocksworld-GTOHP, Depots, Logistics-Learned, Minecraft-Regular, Robot, Rover-GTOHP, Satellite-GTOHP, Towers, Transport, Woodworking | 3 / 3 each | 4 s |
| Factories-simple | 3 / 3 | 4 s |
| Multiarm-Blocksworld | 3 / 3 | 66 s |
| Monroe-Fully-Observable | 3 / 3 | 72 s |
| Hiking | 3 / 3 | 90 s |
| Blocksworld-HPDDL | 3 / 3 | 191 s |
| Snake | 3 / 3 | 564 s |
| AssemblyHierarchical | 2 / 3 | 15 s |
| Minecraft-Player | 1 / 3 | 32 s |
| Monroe-Partially-Observable | 1 / 3 | 306 s |
| Freecell-Learned | 0 / 3 | — |

That is 55 of 63. The failures are memory: Freecell needs tens of millions of search steps even
outside the compiler, the largest Minecraft-Player problem does not get through the grounder, and
the rest run out of their 8 GB inside the trait solver. (Lamps and SharpSAT are not in this set.)

The version of twine before this engine — one search node per type, the trace in every type,
iterative deepening over all decompositions — solved 6 of the first 18 of these problems, and ran
out of memory or out of a 5-minute limit on the rest.

Blind search optimises nothing: there is no heuristic beyond the method order, and the plans are
often long, since a generous depth bound lets depth-first search wander before it finds the way
out. Plan quality is not a concern of the track, which is fortunate. twine's main contribution
to planning is a much more literal interpretation of the word "compilation".

`examples/robot_02_001.sas` is the grounder's output for `Robot/pfile_02_001` of the
[IPC 2023](https://github.com/ipc2023-htn) total-order set, checked in so the generator can be run
without the toolchain.
