use core::marker::PhantomData;

use crate::eval::Eval;

pub struct Nil;
pub struct Cons<H, T>(PhantomData<(H, T)>);

#[macro_export]
macro_rules! tlist {
    () => { $crate::list::Nil };
    ($h:ty $(, $t:ty)* $(,)?) => { $crate::list::Cons<$h, $crate::tlist!($($t),*)> };
}

pub struct Append<A, B>(PhantomData<(A, B)>);

impl<B> Eval for Append<Nil, B> {
    type Out = B;
}

impl<H, T, B> Eval for Append<Cons<H, T>, B>
where
    Append<T, B>: Eval,
{
    type Out = Cons<H, <Append<T, B> as Eval>::Out>;
}

pub struct Rev<A, Acc>(PhantomData<(A, Acc)>);

impl<Acc> Eval for Rev<Nil, Acc> {
    type Out = Acc;
}

impl<H, T, Acc> Eval for Rev<Cons<H, T>, Acc>
where
    Rev<T, Cons<H, Acc>>: Eval,
{
    type Out = <Rev<T, Cons<H, Acc>> as Eval>::Out;
}
