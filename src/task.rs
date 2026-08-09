use core::marker::PhantomData;

/// Every task of the grounded model is a marker type. `Kind` is what the search dispatches on:
/// impl selection on `Prim`/`Comp` is the primitive/abstract test.
pub trait Task {
    type Kind;
}

pub struct Prim<A>(PhantomData<A>);
pub struct Comp<Ms>(PhantomData<Ms>);

pub trait Action {
    type Prec;
    type Add;
    type Del;
}

pub trait Method {
    type Subtasks;
}

/// The index the marker type had in the grounded model, so a compile-time plan can name itself
/// at run time.
pub trait Ident {
    const ID: u32;
}
