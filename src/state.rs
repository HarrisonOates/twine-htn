use core::marker::PhantomData;

use crate::bit::{And, Implies, Not, Or, B1};
use crate::eval::Eval;
use crate::list::{Cons, Nil};

/// A state, a precondition, an add list and a delete list are all the same shape: one bit per
/// state bit of the grounded model, so every operation on them is a zip.
pub struct Holds<Mask, State>(PhantomData<(Mask, State)>);

impl Eval for Holds<Nil, Nil> {
    type Out = B1;
}

impl<M, Ms, S, Ss> Eval for Holds<Cons<M, Ms>, Cons<S, Ss>>
where
    M: Implies<S>,
    Holds<Ms, Ss>: Eval,
    <M as Implies<S>>::Out: And<<Holds<Ms, Ss> as Eval>::Out>,
{
    type Out = <<M as Implies<S>>::Out as And<<Holds<Ms, Ss> as Eval>::Out>>::Out;
}

/// `(state \ del) u add`, matching the engine's order: deletes first, then adds.
pub struct Apply<State, Add, Del>(PhantomData<(State, Add, Del)>);

impl Eval for Apply<Nil, Nil, Nil> {
    type Out = Nil;
}

impl<S, Ss, A, As, D, Ds> Eval for Apply<Cons<S, Ss>, Cons<A, As>, Cons<D, Ds>>
where
    D: Not,
    S: And<<D as Not>::Out>,
    <S as And<<D as Not>::Out>>::Out: Or<A>,
    Apply<Ss, As, Ds>: Eval,
{
    type Out =
        Cons<<<S as And<<D as Not>::Out>>::Out as Or<A>>::Out, <Apply<Ss, As, Ds> as Eval>::Out>;
}
