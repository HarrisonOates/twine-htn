/// One step of the trace a compile-time plan reifies to: the index its marker has in the
/// grounded model, in the preorder the search reached them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    Action(u32),
    Method(u32),
}

pub trait Reify {
    fn emit(out: &mut Vec<Event>);
}

pub fn trace<T: Reify>() -> Vec<Event> {
    let mut out = Vec::new();
    T::emit(&mut out);
    out
}
