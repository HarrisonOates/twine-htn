use crate::domain::{Event, Reify};
use crate::list::{Cons, Nil};
use crate::search::{Do, Use};
use crate::task::Ident;

impl Reify for Nil {
    fn emit(_: &mut Vec<Event>) {}
}

impl<H: Reify, T: Reify> Reify for Cons<H, T> {
    fn emit(out: &mut Vec<Event>) {
        H::emit(out);
        T::emit(out);
    }
}

impl<A: Ident> Reify for Do<A> {
    fn emit(out: &mut Vec<Event>) {
        out.push(Event::Action(A::ID));
    }
}

impl<M: Ident> Reify for Use<M> {
    fn emit(out: &mut Vec<Event>) {
        out.push(Event::Method(M::ID));
    }
}
