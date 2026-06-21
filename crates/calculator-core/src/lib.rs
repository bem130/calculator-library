//! Exact calculator core.
//!
//! The core API keeps exact and approximate/certified outputs separate. A
//! caller supplies a [`CalculationRequest`] and owns the [`EvaluationContext`]
//! used for the calculation.
//!
//! ```
//! use calculator_core::{
//!     calculate, CalculationOutcome, CalculationRequest, CalculatorError,
//!     DomainErrorKind, EnclosureOutputRequest, EvaluationContext, ExactOutput,
//!     ScientificOutputRequest,
//! };
//!
//! let request = CalculationRequest {
//!     scientific_output: ScientificOutputRequest::Omit,
//!     enclosure_output: EnclosureOutputRequest::Omit,
//!     ..CalculationRequest::default()
//! };
//!
//! let mut context = EvaluationContext::default();
//! let outcome = calculate("0.1 + 0.2", &request, &mut context).unwrap();
//! let CalculationOutcome::Complete(calculation) = outcome else {
//!     panic!("rational addition should complete");
//! };
//! let ExactOutput::Included(exact) = calculation.exact else {
//!     panic!("exact output should be included by default");
//! };
//! assert_eq!(exact.plain_text, "3/10");
//!
//! let error = calculate("1 / 0", &request, &mut context).unwrap_err();
//! assert!(matches!(
//!     error,
//!     CalculatorError::Domain(error) if error.kind == DomainErrorKind::DivisionByZero
//! ));
//! ```
#![no_std]
#![forbid(unsafe_code)]

extern crate alloc;

mod api;
mod expression;
mod interval;
mod polynomial;
mod session;
mod syntax;
mod types;

#[cfg(test)]
mod conformance_tests;
#[cfg(test)]
mod fuzz_tests;

pub use api::{calculate, evaluate, parse, present};
pub use session::{apply_calculation_result, reduce_input};
pub use types::*;
