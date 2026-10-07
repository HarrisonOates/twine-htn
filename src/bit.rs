use crate::eval::Eval;

pub struct B0;
pub struct B1;

/// The lazy conditional the whole search is built on: the taken branch is bounded by `Eval`
/// and normalised, the other is an unconstrained parameter and is never looked at.
pub trait Choose<T, E> {
    type Out;
}

impl<T: Eval, E> Choose<T, E> for B1 {
    type Out = T::Out;
}

impl<T, E: Eval> Choose<T, E> for B0 {
    type Out = E::Out;
}
