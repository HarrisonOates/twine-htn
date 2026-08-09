/// A suspended type-level computation. Every recursive step in the planner is a struct that
/// implements `Eval`, so a branch is only normalised when an impl actually demands its `Out`.
pub trait Eval {
    type Out;
}
