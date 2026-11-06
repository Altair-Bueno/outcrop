//! Static, `no_std`, sans-IO incremental evaluation of context-dependent
//! signals.
//!
//! A [`Formation`] is a dependency graph of signals built at compile time,
//! one stratum at a time. Inputs are fed by the caller, derived signals are
//! computed from the signals beneath them, and [`Formation::poll`] reports
//! every signal whose value changed. Nothing allocates: the whole graph lives
//! in a single value whose type encodes its shape.
//!
//! ```
//! use core::fmt::Debug;
//! use outcrop::{Formation, Visit};
//!
//! #[derive(Debug, PartialEq)]
//! struct Temperature(i16);
//! #[derive(Debug, PartialEq)]
//! struct AntiFreeze(bool);
//!
//! struct Print;
//!
//! impl<T: Debug> Visit<T> for Print {
//!     fn visit(&mut self, value: &T) {
//!         println!("{value:?}");
//!     }
//! }
//!
//! let mut formation = Formation::<8>::new()
//!     .input(Temperature(20))
//!     .with(|t: &Temperature| AntiFreeze(t.0 < 10));
//!
//! formation.poll(&mut Print); // Temperature(20), AntiFreeze(false)
//! formation.feed(Temperature(0));
//! formation.poll(&mut Print); // Temperature(0), AntiFreeze(true)
//! ```
#![no_std]
#![forbid(unsafe_code)]
#![cfg_attr(coverage_nightly, feature(coverage_attribute))]

mod capacity;
mod compute;
mod formation;
#[expect(
    unnameable_types,
    reason = "sealed supertraits of the public bounds, unnameable so they cannot be implemented downstream"
)]
mod plumbing;
mod sink;
pub mod stratum;
mod visit;

pub use capacity::Capacity;
pub use capacity::Supported;
pub use formation::Formation;
pub use sink::Sink;
pub use visit::Visit;
