use core::marker::PhantomData;

use crate::bit::{Choose, B0, B1};
use crate::eval::Eval;
use crate::list::{Cons, Nil};

/// A state, a precondition, an add list and a delete list are all the same shape: a complete
/// binary trie over the state bits of the grounded model, with every all-zero subtree collapsed
/// to `Off`. The collapsed form is canonical, so a type names exactly one set of bits, and every
/// operation stops at the first `Off` it meets in a mask: a test or an update costs the number
/// of bits the mask mentions times the height of the trie, not the width of the state.
pub struct Off;
/// A set bit, at the bottom of the trie.
pub struct On;
/// An interior node with at least one set bit below it.
pub struct Br<L, R>(PhantomData<(L, R)>);

/// Both halves of a subtree, reading `Off` as two `Off` halves.
pub trait Split {
    type L;
    type R;
}

impl Split for Off {
    type L = Off;
    type R = Off;
}

impl<L, R> Split for Br<L, R> {
    type L = L;
    type R = R;
}

/// The canonical node over two halves: `Off` when both are.
pub trait Join<R> {
    type Out;
}

impl Join<Off> for Off {
    type Out = Off;
}

impl Join<On> for Off {
    type Out = Br<Off, On>;
}

impl<A, B> Join<Br<A, B>> for Off {
    type Out = Br<Off, Br<A, B>>;
}

impl<R> Join<R> for On {
    type Out = Br<On, R>;
}

impl<A, B, R> Join<R> for Br<A, B> {
    type Out = Br<Br<A, B>, R>;
}

// Every operation below dispatches on `Self`, never on a parameter. When rustc checks the
// generic impls in `search` it meets these with unknown masks, and an unknown `Self` makes it
// stop at "ambiguous"; dispatching on a parameter instead has it enumerate the impls and recurse
// into `Br<_, Br<_, ...>>` until it overflows.

/// `Mask ⊆ State`, failing at the first missing bit.
pub struct Holds<Mask, State>(PhantomData<(Mask, State)>);
pub struct Fails;

impl Eval for Fails {
    type Out = B0;
}

impl<M: Within<S>, S> Eval for Holds<M, S> {
    type Out = M::Out;
}

pub trait Within<S> {
    type Out;
}

impl<S> Within<S> for Off {
    type Out = B1;
}

impl Within<On> for On {
    type Out = B1;
}

impl Within<Off> for On {
    type Out = B0;
}

impl<A, B> Within<Off> for Br<A, B> {
    type Out = B0;
}

impl<A, B, C, D> Within<Br<C, D>> for Br<A, B>
where
    A: Within<C>,
    A::Out: Choose<Holds<B, D>, Fails>,
{
    type Out = <A::Out as Choose<Holds<B, D>, Fails>>::Out;
}

/// `(state \ del) u add`, matching the engine's order: deletes first, then adds. Only the
/// subtrees the masks touch are rebuilt; everything else is the old state's subtree, shared.
pub struct Apply<State, Add, Del>(PhantomData<(State, Add, Del)>);

impl<S, A: Effect<S, D>, D> Eval for Apply<S, A, D> {
    type Out = A::Out;
}

/// An add mask applied, with its delete mask, to a state.
pub trait Effect<S, D> {
    type Out;
}

impl<S, D: Clear<S>> Effect<S, D> for Off {
    type Out = D::Out;
}

impl<S, D> Effect<S, D> for On {
    type Out = On;
}

impl<S: Split, D: Split, AL, AR> Effect<S, D> for Br<AL, AR>
where
    AL: Effect<S::L, D::L>,
    AR: Effect<S::R, D::R>,
    AL::Out: Join<AR::Out>,
{
    type Out = <AL::Out as Join<AR::Out>>::Out;
}

/// A delete mask applied to a state.
pub trait Clear<S> {
    type Out;
}

impl<S> Clear<S> for Off {
    type Out = S;
}

impl<S> Clear<S> for On {
    type Out = Off;
}

impl<S: Split, DL, DR> Clear<S> for Br<DL, DR>
where
    DL: Clear<S::L>,
    DR: Clear<S::R>,
    DL::Out: Join<DR::Out>,
{
    type Out = <DL::Out as Join<DR::Out>>::Out;
}

/// Structural equality on canonical tries, which is equality of the bit sets they name.
pub struct Same<A, B>(PhantomData<(A, B)>);
pub struct Yes;

impl Eval for Yes {
    type Out = B1;
}

impl<A: SameAs<B>, B> Eval for Same<A, B> {
    type Out = A::Out;
}

pub trait SameAs<B> {
    type Out;
}

impl SameAs<Off> for Off {
    type Out = B1;
}

impl SameAs<On> for On {
    type Out = B1;
}

impl SameAs<On> for Off {
    type Out = B0;
}

impl SameAs<Off> for On {
    type Out = B0;
}

impl<A, B> SameAs<Br<A, B>> for Off {
    type Out = B0;
}

impl<A, B> SameAs<Off> for Br<A, B> {
    type Out = B0;
}

impl<A, B, C, D> SameAs<Br<C, D>> for Br<A, B>
where
    A: SameAs<C>,
    A::Out: Choose<Same<B, D>, Fails>,
{
    type Out = <A::Out as Choose<Same<B, D>, Fails>>::Out;
}

/// Whether a state is in a list of states.
pub struct Member<S, List>(PhantomData<(S, List)>);

impl<S, L: Contains<S>> Eval for Member<S, L> {
    type Out = L::Out;
}

pub trait Contains<S> {
    type Out;
}

impl<S> Contains<S> for Nil {
    type Out = B0;
}

impl<S, H, T> Contains<S> for Cons<H, T>
where
    S: SameAs<H>,
    S::Out: Choose<Yes, Member<S, T>>,
{
    type Out = <S::Out as Choose<Yes, Member<S, T>>>::Out;
}

// Set operations
// --------------
//
// For projecting a state onto the facts a task can touch and merging the task's outcomes back:
// each result is canonical, and a subtree one side has as `Off` is not walked.

/// `Self ∩ B`
pub trait Meet<B> {
    type Out;
}

impl<B> Meet<B> for Off {
    type Out = Off;
}

impl Meet<On> for On {
    type Out = On;
}

impl Meet<Off> for On {
    type Out = Off;
}

impl<A, B> Meet<Off> for Br<A, B> {
    type Out = Off;
}

impl<A: Meet<C>, B: Meet<D>, C, D> Meet<Br<C, D>> for Br<A, B>
where
    A::Out: Join<B::Out>,
{
    type Out = <A::Out as Join<B::Out>>::Out;
}

/// `Self ∖ B`
pub trait Less<B> {
    type Out;
}

impl<B> Less<B> for Off {
    type Out = Off;
}

impl Less<On> for On {
    type Out = Off;
}

impl Less<Off> for On {
    type Out = On;
}

impl<A, B> Less<Off> for Br<A, B> {
    type Out = Br<A, B>;
}

impl<A: Less<C>, B: Less<D>, C, D> Less<Br<C, D>> for Br<A, B>
where
    A::Out: Join<B::Out>,
{
    type Out = <A::Out as Join<B::Out>>::Out;
}

/// `Self ∪ B`
pub trait Unite<B> {
    type Out;
}

impl<B> Unite<B> for Off {
    type Out = B;
}

impl<B> Unite<B> for On {
    type Out = On;
}

impl<A, B> Unite<Off> for Br<A, B> {
    type Out = Br<A, B>;
}

impl<A: Unite<C>, B: Unite<D>, C, D> Unite<Br<C, D>> for Br<A, B> {
    type Out = Br<A::Out, B::Out>;
}

// Firing an action
// ----------------
//
// An action's precondition, delete list and add list, merged by the generator into one trie whose
// leaves say what happens to each fact it mentions. One walk over the state tests the
// precondition and writes the effects together, so firing costs one goal per trie node the action
// touches rather than a test and then an update, each with its own bookkeeping: the trait solver
// keeps every goal it proves, and goals, not types, are what fill its memory.

/// The fact must hold and is left as it is.
pub struct Need;
/// The fact must hold and is deleted.
pub struct NeedDel;
/// The fact is added (whether or not it held, or is also needed: deletes come first, then adds).
pub struct Add;
/// The fact must hold, and stays (deleted and added again, or only needed and added).
pub struct NeedAdd;
/// The fact is deleted.
pub struct Del;

/// The state after an action, or `Blocked` if its precondition fails.
pub struct Fired<S>(PhantomData<S>);
pub struct Blocked;

/// The new state, if `Self`, an action's merged trie, can fire in state `S`.
pub trait Fire<S> {
    type Out;
}

impl<S> Fire<S> for Off {
    type Out = Fired<S>;
}

impl Fire<On> for Need {
    type Out = Fired<On>;
}

impl Fire<Off> for Need {
    type Out = Blocked;
}

impl Fire<On> for NeedDel {
    type Out = Fired<Off>;
}

impl Fire<Off> for NeedDel {
    type Out = Blocked;
}

impl Fire<On> for NeedAdd {
    type Out = Fired<On>;
}

impl Fire<Off> for NeedAdd {
    type Out = Blocked;
}

impl<S> Fire<S> for Add {
    type Out = Fired<On>;
}

impl<S> Fire<S> for Del {
    type Out = Fired<Off>;
}

impl<L: Fire<Off>, R> Fire<Off> for Br<L, R>
where
    L::Out: FireRight<R, Off>,
{
    type Out = <L::Out as FireRight<R, Off>>::Out;
}

impl<L: Fire<A>, R, A, B> Fire<Br<A, B>> for Br<L, R>
where
    L::Out: FireRight<R, B>,
{
    type Out = <L::Out as FireRight<R, B>>::Out;
}

/// With the left half fired, the right half: `R` on state half `B`.
pub trait FireRight<R, B> {
    type Out;
}

impl<R, B> FireRight<R, B> for Blocked {
    type Out = Blocked;
}

impl<NL, R: Fire<B>, B> FireRight<R, B> for Fired<NL>
where
    R::Out: Rejoin<NL>,
{
    type Out = <R::Out as Rejoin<NL>>::Out;
}

/// Both halves fired: the canonical node over them.
pub trait Rejoin<NL> {
    type Out;
}

impl<NL> Rejoin<NL> for Blocked {
    type Out = Blocked;
}

impl<NL: Join<NR>, NR> Rejoin<NL> for Fired<NR> {
    type Out = Fired<NL::Out>;
}
