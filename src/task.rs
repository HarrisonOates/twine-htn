use core::marker::PhantomData;

/// Every task of the grounded model is a marker type, and its `Kind` is all the search needs to
/// know about it: impl selection on the kind is the primitive/abstract test, a primitive kind
/// carries its masks, and an abstract kind names the first of its methods, a chain of
/// `search::Alts` cells.
pub trait Task {
    type Kind;
}

/// A primitive task: its index in the grounded model, then its precondition, add and delete
/// masks.
pub struct Prim<I, Prec, Add, Del>(PhantomData<(I, Prec, Add, Del)>);
/// An abstract task on a cycle of the decomposition graph: applying one of its methods spends a
/// level of the depth bound, which is what stops a recursive decomposition going on for ever.
pub struct Comp<Ms>(PhantomData<Ms>);
/// An abstract task that is on no cycle but can reach one: it spends nothing and passes the
/// bound down unchanged.
pub struct Pass<Ms>(PhantomData<Ms>);
/// An abstract task with no cycle anywhere below it. Its decompositions are finite on their
/// own, so it is searched with the bound at zero whatever it was, and its streams are the same
/// types in every deepening round.
pub struct Flat<Ms>(PhantomData<Ms>);

/// An index in the grounded model, so a compile-time plan can name itself at run time.
///
/// Tasks are still one struct each: every task's `Task` impl has its own self type, so coherence
/// has nothing to compare. Were the tasks `T<N>` too, it would check every pair of the impls for
/// overlap, which on a model with tens of thousands of tasks takes longer than the search.
pub struct Id<const N: u32>;

pub trait Ident {
    const ID: u32;
}

impl<const N: u32> Ident for Id<N> {
    const ID: u32 = N;
}
