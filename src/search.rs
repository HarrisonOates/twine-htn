use core::marker::PhantomData;

use crate::bit::{Choose, Succ, Zero};
use crate::eval::Eval;
use crate::list::{Append, Cons, Nil, Rev};
use crate::state::{Apply, Holds};
use crate::task::{Action, Comp, Method, Prim, Task};

/// A grounded totally ordered HTN problem, ready for the trait solver.
///
/// `MaxDepth` bounds the number of method applications on a path, and doubles as the number of
/// iterative-deepening rounds.
pub trait Problem {
    type Init;
    type Goal;
    type Root;
    type MaxDepth;
}

pub struct Found<Trace>(PhantomData<Trace>);
pub struct Stuck;

/// The one trace event per derivation node, in the order progression reaches them, which for a
/// totally ordered network is exactly a preorder walk of the decomposition tree.
pub struct Do<A>(PhantomData<A>);
pub struct Use<M>(PhantomData<M>);

pub trait OrElse<Alt> {
    type Out;
}

impl<Trace, Alt> OrElse<Alt> for Found<Trace> {
    type Out = Found<Trace>;
}

impl<Alt: Eval> OrElse<Alt> for Stuck {
    type Out = Alt::Out;
}

pub struct Solve<P, St, Net, D, Tr>(PhantomData<(P, St, Net, D, Tr)>);
pub struct Expand<P, K, St, Rest, D, Tr>(PhantomData<(P, K, St, Rest, D, Tr)>);
pub struct Advance<P, A, St, Rest, D, Tr>(PhantomData<(P, A, St, Rest, D, Tr)>);
pub struct Try<P, Ms, St, Rest, D, Tr>(PhantomData<(P, Ms, St, Rest, D, Tr)>);
pub struct Reached<Tr>(PhantomData<Tr>);
pub struct Dead;

impl<Tr> Eval for Reached<Tr>
where
    Rev<Tr, Nil>: Eval,
{
    type Out = Found<<Rev<Tr, Nil> as Eval>::Out>;
}

impl Eval for Dead {
    type Out = Stuck;
}

impl<P: Problem, St, D, Tr> Eval for Solve<P, St, Nil, D, Tr>
where
    Holds<P::Goal, St>: Eval,
    <Holds<P::Goal, St> as Eval>::Out: Choose<Reached<Tr>, Dead>,
{
    type Out = <<Holds<P::Goal, St> as Eval>::Out as Choose<Reached<Tr>, Dead>>::Out;
}

impl<P, St, T: Task, Rest, D, Tr> Eval for Solve<P, St, Cons<T, Rest>, D, Tr>
where
    Expand<P, T::Kind, St, Rest, D, Tr>: Eval,
{
    type Out = <Expand<P, T::Kind, St, Rest, D, Tr> as Eval>::Out;
}

impl<P, A: Action, St, Rest, D, Tr> Eval for Expand<P, Prim<A>, St, Rest, D, Tr>
where
    Holds<A::Prec, St>: Eval,
    <Holds<A::Prec, St> as Eval>::Out: Choose<Advance<P, A, St, Rest, D, Tr>, Dead>,
{
    type Out =
        <<Holds<A::Prec, St> as Eval>::Out as Choose<Advance<P, A, St, Rest, D, Tr>, Dead>>::Out;
}

impl<P, A: Action, St, Rest, D, Tr> Eval for Advance<P, A, St, Rest, D, Tr>
where
    Apply<St, A::Add, A::Del>: Eval,
    Solve<P, <Apply<St, A::Add, A::Del> as Eval>::Out, Rest, D, Cons<Do<A>, Tr>>: Eval,
{
    type Out =
        <Solve<P, <Apply<St, A::Add, A::Del> as Eval>::Out, Rest, D, Cons<Do<A>, Tr>> as Eval>::Out;
}

impl<P, Ms, St, Rest, D, Tr> Eval for Expand<P, Comp<Ms>, St, Rest, D, Tr>
where
    Try<P, Ms, St, Rest, D, Tr>: Eval,
{
    type Out = <Try<P, Ms, St, Rest, D, Tr> as Eval>::Out;
}

impl<P, St, Rest, D, Tr> Eval for Try<P, Nil, St, Rest, D, Tr> {
    type Out = Stuck;
}

impl<P, M, More, St, Rest, Tr> Eval for Try<P, Cons<M, More>, St, Rest, Zero, Tr> {
    type Out = Stuck;
}

impl<P, M: Method, More, St, Rest, D, Tr> Eval for Try<P, Cons<M, More>, St, Rest, Succ<D>, Tr>
where
    Append<M::Subtasks, Rest>: Eval,
    Solve<P, St, <Append<M::Subtasks, Rest> as Eval>::Out, D, Cons<Use<M>, Tr>>: Eval,
    <Solve<P, St, <Append<M::Subtasks, Rest> as Eval>::Out, D, Cons<Use<M>, Tr>> as Eval>::Out:
        OrElse<Try<P, More, St, Rest, Succ<D>, Tr>>,
{
    type Out = <<Solve<
        P,
        St,
        <Append<M::Subtasks, Rest> as Eval>::Out,
        D,
        Cons<Use<M>, Tr>,
    > as Eval>::Out as OrElse<Try<P, More, St, Rest, Succ<D>, Tr>>>::Out;
}

/// Iterative deepening. Without it a cyclic decomposition sends depth-first search down one
/// recursive method until the bound runs out, every time.
pub struct Deepen<P, D, Left>(PhantomData<(P, D, Left)>);

impl<P, D> Eval for Deepen<P, D, Zero> {
    type Out = Stuck;
}

impl<P: Problem, D, L> Eval for Deepen<P, D, Succ<L>>
where
    Solve<P, P::Init, Cons<P::Root, Nil>, D, Nil>: Eval,
    <Solve<P, P::Init, Cons<P::Root, Nil>, D, Nil> as Eval>::Out: OrElse<Deepen<P, Succ<D>, L>>,
{
    type Out = <<Solve<P, P::Init, Cons<P::Root, Nil>, D, Nil> as Eval>::Out as OrElse<
        Deepen<P, Succ<D>, L>,
    >>::Out;
}

pub type Search<P> = <Deepen<P, Zero, <P as Problem>::MaxDepth> as Eval>::Out;

/// Only `Found` implements this, so naming `Trace<P>` for an unsolvable problem is a type error:
/// `the trait bound `Stuck: Solved` is not satisfied`.
pub trait Solved {
    type Trace;
}

impl<Tr> Solved for Found<Tr> {
    type Trace = Tr;
}

pub type Trace<P> = <Search<P> as Solved>::Trace;
