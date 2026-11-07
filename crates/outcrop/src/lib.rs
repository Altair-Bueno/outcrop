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
//! let mut formation = Formation::new()
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

mod compute;
#[expect(
    unnameable_types,
    reason = "change flags sized by the strata, unnameable so they stay an implementation detail"
)]
mod flags;
mod formation;
#[expect(
    unnameable_types,
    reason = "sealed supertraits of the public bounds, unnameable so they cannot be implemented downstream"
)]
mod plumbing;
mod sink;
#[expect(
    unnameable_types,
    reason = "strata only appear in inferred formation types, so they can change without breaking users"
)]
mod stratum;
mod visit;

pub use formation::Formation;
pub use sink::Sink;
pub use stratum::Compute;
pub use stratum::Feed;
pub use stratum::Report;
pub use stratum::Strata;
pub use visit::Visit;
