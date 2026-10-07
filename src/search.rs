use core::marker::PhantomData;

use crate::bit::{Choose, B0, B1};
use crate::eval::Eval;
use crate::list::{Cons, Nil};
use crate::state::{Blocked, Fire, Fired, Holds, Less, Meet, Member, Unite};
use crate::task::{Comp, Flat, Guard, Pass, Prim, Proj, Sub, Task};

/// A grounded totally ordered HTN problem, ready for the trait solver.
///
/// `Schedule` is the list of depth bounds iterative deepening tries, in order. A bound is a binary
/// numeral, least significant bit first and with no trailing `B0`, so `Nil` is zero: it sits in
/// the key of every task the search meets, and a Peano numeral would cost a node per level there.
pub trait Problem {
    type Init;
    type Goal;
    type Root;
    type Schedule;
}

/// A lazy stream of outcomes. `Hit` is one way to carry out a piece of the network, the state it
/// ends in and the plan that gets there, followed by the rest of the stream as a suspended
/// computation that nothing normalises until an impl asks for its `Out`.
pub struct Hit<St, Plan, More>(PhantomData<(St, Plan, More)>);
pub struct Miss;

/// Plans are trees, so gluing two together is one node. `Reify` walks them in preorder, which
/// for a totally ordered network is the order progression reached each derivation node.
pub struct Do<A>(PhantomData<A>);
pub struct Dec<M, Plan>(PhantomData<(M, Plan)>);
pub struct Then<P, Q>(PhantomData<(P, Q)>);

pub struct Dead;

impl Eval for Dead {
    type Out = Miss;
}

/// Every way to carry out task `T` from state `St` decomposing at most `D` levels deep, each end
/// state once. Nothing about the caller is in this type, which is the point: the trait solver
/// caches what a type normalises to, so wherever the search meets the same task in the same
/// state with the same budget — in another branch, in a later deepening round, at the other end
/// of a cycle — it reads the stream it already built instead of searching again.
pub struct Run<T, St, D>(PhantomData<(T, St, D)>);

// Every suspended computation is a struct with exactly one `Eval` impl, which hands off to a
// trait on the thing it matches. Matching on a parameter would have rustc, checking these
// generic impls, enumerate the candidates for an unknown type and recurse until it overflows;
// an unknown `Self` it leaves alone.

impl<T: Task, St, D> Eval for Run<T, St, D>
where
    T::Kind: Expand<St, D>,
{
    type Out = <T::Kind as Expand<St, D>>::Out;
}

/// The primitive/abstract test, and how an abstract task spends the depth bound.
pub trait Expand<St, D> {
    type Out;
}

impl<I, Prog: Fire<St>, St, D> Expand<St, D> for Prim<I, Prog>
where
    Prog::Out: Outcome<I>,
{
    type Out = <Prog::Out as Outcome<I>>::Out;
}

/// An action's stream: its one outcome, if its precondition held.
pub trait Outcome<I> {
    type Out;
}

impl<I> Outcome<I> for Blocked {
    type Out = Miss;
}

impl<I, S> Outcome<I> for Fired<S> {
    type Out = Hit<S, Do<I>, Dead>;
}

impl<Ms, St, D: Budget<Ms, St>> Expand<St, D> for Comp<Ms> {
    type Out = D::Out;
}

impl<Ms, St, D> Expand<St, D> for Pass<Ms>
where
    Dedup<Methods<Ms, St, D>, Nil>: Eval,
{
    type Out = <Dedup<Methods<Ms, St, D>, Nil> as Eval>::Out;
}

impl<Ms, St, D> Expand<St, D> for Flat<Ms>
where
    Dedup<Methods<Ms, St, Nil>, Nil>: Eval,
{
    type Out = <Dedup<Methods<Ms, St, Nil>, Nil> as Eval>::Out;
}

impl<P, K, St, D> Expand<St, D> for Guard<P, K>
where
    Holds<P, St>: Eval,
    <Holds<P, St> as Eval>::Out: Choose<Inner<K, St, D>, Dead>,
{
    type Out = <<Holds<P, St> as Eval>::Out as Choose<Inner<K, St, D>, Dead>>::Out;
}

/// A task kind's stream in a state, as a value of its own: the cache key a projected task shares.
pub struct Inner<K, St, D>(PhantomData<(K, St, D)>);

impl<K: Expand<St, D>, St, D> Eval for Inner<K, St, D> {
    type Out = K::Out;
}

impl<R, K, St: Meet<R>, D> Expand<St, D> for Proj<R, K>
where
    Inner<K, St::Out, D>: Eval,
    <Inner<K, St::Out, D> as Eval>::Out: Remerge<St, R>,
{
    type Out = <<Inner<K, St::Out, D> as Eval>::Out as Remerge<St, R>>::Out;
}

/// Puts each outcome of a projected stream back into the full state it started from: the facts
/// outside `R` as they were, the facts inside as the outcome has them.
pub trait Remerge<St, R> {
    type Out;
}

impl<St, R> Remerge<St, R> for Miss {
    type Out = Miss;
}

impl<S, P, More, St: Less<R>, R> Remerge<St, R> for Hit<S, P, More>
where
    St::Out: Unite<S>,
{
    type Out = Hit<<St::Out as Unite<S>>::Out, P, RemergeMore<More, St, R>>;
}

pub struct RemergeMore<More, St, R>(PhantomData<(More, St, R)>);

impl<More: Eval, St, R> Eval for RemergeMore<More, St, R>
where
    More::Out: Remerge<St, R>,
{
    type Out = <More::Out as Remerge<St, R>>::Out;
}

/// A level of the bound, spent on decomposing a task that is on a cycle.
pub trait Budget<Ms, St> {
    type Out;
}

impl<Ms, St> Budget<Ms, St> for Nil {
    type Out = Miss;
}

impl<Ms, St, B, N> Budget<Ms, St> for Cons<B, N>
where
    Cons<B, N>: Pred,
    Dedup<Methods<Ms, St, <Cons<B, N> as Pred>::Out>, Nil>: Eval,
{
    type Out = <Dedup<Methods<Ms, St, <Cons<B, N> as Pred>::Out>, Nil> as Eval>::Out;
}

/// The predecessor of a nonzero binary numeral.
pub trait Pred {
    type Out;
}

impl Pred for Cons<B1, Nil> {
    type Out = Nil;
}

impl<H, R> Pred for Cons<B1, Cons<H, R>> {
    type Out = Cons<B0, Cons<H, R>>;
}

impl<N: Pred> Pred for Cons<B0, N> {
    type Out = Cons<B1, N::Out>;
}

/// An abstract task's methods are a chain of named cells, one marker type per method, so the
/// rest of a task's methods is one small type however many there are.
pub trait Alts {
    type Cell;
}

/// A method: its index, its subtasks in order, and the marker of the next method to try.
pub struct Alt<M, Subtasks, Next>(PhantomData<(M, Subtasks, Next)>);
pub struct NoAlt;
/// The end of every method chain.
pub struct Fin;

impl Alts for Fin {
    type Cell = NoAlt;
}

/// The streams of the methods chained from `Ms`, one after another, each outcome tagged with the
/// method it came from.
pub struct Methods<Ms, St, D>(PhantomData<(Ms, St, D)>);

impl<Ms: Alts, St, D> Eval for Methods<Ms, St, D>
where
    Ms::Cell: Alternatives<St, D>,
{
    type Out = <Ms::Cell as Alternatives<St, D>>::Out;
}

pub trait Alternatives<St, D> {
    type Out;
}

impl<St, D> Alternatives<St, D> for NoAlt {
    type Out = Miss;
}

impl<M, Sub, Next, St, D> Alternatives<St, D> for Alt<M, Sub, Next>
where
    RunSeq<Sub, St, D>: Eval,
    <RunSeq<Sub, St, D> as Eval>::Out: Lift<M, Methods<Next, St, D>>,
{
    type Out = <<RunSeq<Sub, St, D> as Eval>::Out as Lift<M, Methods<Next, St, D>>>::Out;
}

/// Tags a stream's plans with `Dec<M, _>`, then carries on with `Alt` once it runs dry.
pub trait Lift<M, Alt> {
    type Out;
}

impl<M, Alt: Eval> Lift<M, Alt> for Miss {
    type Out = Alt::Out;
}

impl<St, Pl, More, M, Alt> Lift<M, Alt> for Hit<St, Pl, More> {
    type Out = Hit<St, Dec<M, Pl>, LiftMore<M, More, Alt>>;
}

pub struct LiftMore<M, More, Alt>(PhantomData<(M, More, Alt)>);

impl<M, More: Eval, Alt> Eval for LiftMore<M, More, Alt>
where
    More::Out: Lift<M, Alt>,
{
    type Out = <More::Out as Lift<M, Alt>>::Out;
}

/// Every way to carry out a sequence of tasks: each outcome of the head, followed by every
/// outcome of the tail from there. Sequences of two or more are deduplicated, which keeps a
/// method's stream as long as the number of states it can end in, not the number of paths.
pub struct RunSeq<Ts, St, D>(PhantomData<(Ts, St, D)>);
pub struct Chain<T, Rest, St, D>(PhantomData<(T, Rest, St, D)>);

impl<Ts: Sequence<St, D>, St, D> Eval for RunSeq<Ts, St, D> {
    type Out = Ts::Out;
}

pub trait Sequence<St, D> {
    type Out;
}

impl<St, D> Sequence<St, D> for Nil {
    type Out = Hit<St, Nil, Dead>;
}

impl<T, M, Rest, St, D> Sequence<St, D> for Cons<Sub<T, M>, Rest>
where
    Holds<M, St>: Eval,
    <Holds<M, St> as Eval>::Out: Choose<Tails<T, Rest, St, D>, Dead>,
{
    type Out = <<Holds<M, St> as Eval>::Out as Choose<Tails<T, Rest, St, D>, Dead>>::Out;
}

pub struct Tails<T, Rest, St, D>(PhantomData<(T, Rest, St, D)>);

impl<T, Rest: Tail<T, St, D>, St, D> Eval for Tails<T, Rest, St, D> {
    type Out = Rest::Out;
}

/// What follows the head of a sequence decides whether its stream is deduplicated.
pub trait Tail<T, St, D> {
    type Out;
}

impl<T, St, D> Tail<T, St, D> for Nil
where
    Chain<T, Nil, St, D>: Eval,
{
    type Out = <Chain<T, Nil, St, D> as Eval>::Out;
}

impl<T, U, Us, St, D> Tail<T, St, D> for Cons<U, Us>
where
    Dedup<Chain<T, Cons<U, Us>, St, D>, Nil>: Eval,
{
    type Out = <Dedup<Chain<T, Cons<U, Us>, St, D>, Nil> as Eval>::Out;
}

impl<T, Rest, St, D> Eval for Chain<T, Rest, St, D>
where
    Run<T, St, D>: Eval,
    <Run<T, St, D> as Eval>::Out: Bind<Rest, D>,
{
    type Out = <<Run<T, St, D> as Eval>::Out as Bind<Rest, D>>::Out;
}

pub trait Bind<Rest, D> {
    type Out;
}

impl<Rest, D> Bind<Rest, D> for Miss {
    type Out = Miss;
}

impl<S, P, More, Rest, D> Bind<Rest, D> for Hit<S, P, More>
where
    RunSeq<Rest, S, D>: Eval,
    <RunSeq<Rest, S, D> as Eval>::Out: Glue<P, BindMore<More, Rest, D>>,
{
    type Out = <<RunSeq<Rest, S, D> as Eval>::Out as Glue<P, BindMore<More, Rest, D>>>::Out;
}

pub struct BindMore<More, Rest, D>(PhantomData<(More, Rest, D)>);

impl<More: Eval, Rest, D> Eval for BindMore<More, Rest, D>
where
    More::Out: Bind<Rest, D>,
{
    type Out = <More::Out as Bind<Rest, D>>::Out;
}

/// Prefixes a stream's plans with `P`, then carries on with `Alt` once it runs dry.
pub trait Glue<P, Alt> {
    type Out;
}

impl<P, Alt: Eval> Glue<P, Alt> for Miss {
    type Out = Alt::Out;
}

impl<S, Q, More, P, Alt> Glue<P, Alt> for Hit<S, Q, More> {
    type Out = Hit<S, Then<P, Q>, GlueMore<P, More, Alt>>;
}

pub struct GlueMore<P, More, Alt>(PhantomData<(P, More, Alt)>);

impl<P, More: Eval, Alt> Eval for GlueMore<P, More, Alt>
where
    More::Out: Glue<P, Alt>,
{
    type Out = <More::Out as Glue<P, Alt>>::Out;
}

/// Drops every outcome whose end state has already come out of this stream: whatever follows
/// depends on the state alone, so the first plan to reach a state is as good as any other.
pub struct Dedup<Src, Seen>(PhantomData<(Src, Seen)>);

impl<Src: Eval, Seen> Eval for Dedup<Src, Seen>
where
    Src::Out: Fresh<Seen>,
{
    type Out = <Src::Out as Fresh<Seen>>::Out;
}

pub trait Fresh<Seen> {
    type Out;
}

impl<Seen> Fresh<Seen> for Miss {
    type Out = Miss;
}

impl<S, P, More, Seen> Fresh<Seen> for Hit<S, P, More>
where
    Member<S, Seen>: Eval,
    <Member<S, Seen> as Eval>::Out: Choose<Dedup<More, Seen>, Keep<S, P, More, Seen>>,
{
    type Out =
        <<Member<S, Seen> as Eval>::Out as Choose<Dedup<More, Seen>, Keep<S, P, More, Seen>>>::Out;
}

pub struct Keep<S, P, More, Seen>(PhantomData<(S, P, More, Seen)>);

impl<S, P, More, Seen> Eval for Keep<S, P, More, Seen> {
    type Out = Hit<S, P, Dedup<More, Cons<S, Seen>>>;
}

pub struct Found<Plan>(PhantomData<Plan>);
pub struct Stuck;

/// The first outcome of a stream that satisfies the goal, or `Alt` if there is none.
pub struct Seek<Goal, Src, Alt>(PhantomData<(Goal, Src, Alt)>);

impl<Goal, Src: Eval, Alt> Eval for Seek<Goal, Src, Alt>
where
    Src::Out: Accept<Goal, Alt>,
{
    type Out = <Src::Out as Accept<Goal, Alt>>::Out;
}

pub trait Accept<Goal, Alt> {
    type Out;
}

impl<Goal, Alt: Eval> Accept<Goal, Alt> for Miss {
    type Out = Alt::Out;
}

impl<Goal, S, P, More, Alt> Accept<Goal, Alt> for Hit<S, P, More>
where
    Holds<Goal, S>: Eval,
    <Holds<Goal, S> as Eval>::Out: Choose<Done<P>, Seek<Goal, More, Alt>>,
{
    type Out = <<Holds<Goal, S> as Eval>::Out as Choose<Done<P>, Seek<Goal, More, Alt>>>::Out;
}

pub struct Done<P>(PhantomData<P>);

impl<P> Eval for Done<P> {
    type Out = Found<P>;
}

/// Iterative deepening: the root's stream under each bound of the schedule in turn. A bound that
/// is too generous costs little, since a task met again in a state it was already searched from
/// is read from the cache, and everything below the top of a round that an earlier round built
/// is read from the cache too.
pub struct Deepen<P, Schedule>(PhantomData<(P, Schedule)>);

impl<P, L: Rounds<P>> Eval for Deepen<P, L> {
    type Out = L::Out;
}

pub trait Rounds<P> {
    type Out;
}

impl<P> Rounds<P> for Nil {
    type Out = Stuck;
}

impl<P: Problem, D, L> Rounds<P> for Cons<D, L>
where
    Seek<P::Goal, Run<P::Root, P::Init, D>, Deepen<P, L>>: Eval,
{
    type Out = <Seek<P::Goal, Run<P::Root, P::Init, D>, Deepen<P, L>> as Eval>::Out;
}

pub type Search<P> = <Deepen<P, <P as Problem>::Schedule> as Eval>::Out;

/// Only `Found` implements this, so naming `Trace<P>` for a problem with no plan within its
/// schedule is a type error: `the trait bound `Stuck: Solved` is not satisfied`.
pub trait Solved {
    type Trace;
}

impl<Tr> Solved for Found<Tr> {
    type Trace = Tr;
}

pub type Trace<P> = <Search<P> as Solved>::Trace;
