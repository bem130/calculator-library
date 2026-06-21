#![no_std]
#![forbid(unsafe_code)]

extern crate alloc;

mod api;
mod expression;
mod session;
mod syntax;
mod types;

pub use api::{calculate, evaluate, parse, present};
pub use session::{apply_calculation_result, reduce_input};
pub use types::*;
