use core::marker::PhantomData;

/// Type-level lists: subtask sequences, the states a stream has produced, and binary numerals,
/// least significant bit first.
pub struct Nil;
pub struct Cons<H, T>(PhantomData<(H, T)>);
