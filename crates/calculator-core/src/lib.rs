#![no_std]
#![forbid(unsafe_code)]

extern crate alloc;

mod api;
mod expression;
mod interval;
mod session;
mod syntax;
mod types;

#[cfg(test)]
mod conformance_tests;

pub use api::{calculate, evaluate, parse, present};
pub use session::{apply_calculation_result, reduce_input};
pub use types::*;
