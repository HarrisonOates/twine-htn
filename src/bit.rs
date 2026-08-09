use crate::eval::Eval;

pub struct B0;
pub struct B1;

pub trait Not {
    type Out;
}

impl Not for B0 {
    type Out = B1;
}

impl Not for B1 {
    type Out = B0;
}

pub trait And<R> {
    type Out;
}

impl<R> And<R> for B0 {
    type Out = B0;
}

impl<R> And<R> for B1 {
    type Out = R;
}

pub trait Or<R> {
    type Out;
}

impl<R> Or<R> for B0 {
    type Out = R;
}

impl<R> Or<R> for B1 {
    type Out = B1;
}

pub trait Implies<R> {
    type Out;
}

impl<R> Implies<R> for B0 {
    type Out = B1;
}

impl<R> Implies<R> for B1 {
    type Out = R;
}

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

pub struct Zero;
pub struct Succ<N>(core::marker::PhantomData<N>);
