//! A totally ordered HTN planner that runs inside the Rust trait solver.
//!
//! The grounded problem is a set of marker types, the state is a type-level bit vector, and
//! depth-first progression search is impl selection. `rustc` does the planning; the binary in
//! this crate only reads the answer back out and formats it.

#![forbid(unsafe_code)]
#![recursion_limit = "4096"]

pub mod bit;
pub mod domain;
pub mod eval;
pub mod gen;
pub mod list;
pub mod plan;
pub mod problem;
pub mod reify;
pub mod sas;
pub mod search;
pub mod state;
pub mod task;

pub use problem::Prob;
