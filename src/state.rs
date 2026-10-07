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
