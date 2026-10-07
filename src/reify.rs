use crate::domain::{Event, Reify};
use crate::list::Nil;
use crate::search::{Dec, Do, Then};
use crate::task::Ident;

impl Reify for Nil {
    fn emit(_: &mut Vec<Event>) {}
}

impl<P: Reify, Q: Reify> Reify for Then<P, Q> {
    fn emit(out: &mut Vec<Event>) {
        P::emit(out);
        Q::emit(out);
    }
}

impl<A: Ident> Reify for Do<A> {
    fn emit(out: &mut Vec<Event>) {
        out.push(Event::Action(A::ID));
    }
}

impl<M: Ident, P: Reify> Reify for Dec<M, P> {
    fn emit(out: &mut Vec<Event>) {
        out.push(Event::Method(M::ID));
        P::emit(out);
    }
}
