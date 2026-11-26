//! Model-work admission and accounting for the shared budget, layered over the durable core ledger.
pub use sw_core::budget::*;
pub mod admission;
pub mod helpers;
pub mod runtime;
