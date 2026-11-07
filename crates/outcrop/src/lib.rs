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
//!
//! A formation built this way has a type that cannot be written out. Where a
//! named type is needed, such as in a `static`, the `macros` feature provides
//! `strata!`, which declares one.
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
    reason = "builder machinery behind the sealed `Compute` and `Layered` bounds"
)]
mod plumbing;
mod reading;
mod sink;
mod strata;
#[expect(
    unnameable_types,
    reason = "strata only appear in inferred formation types, so they can change without breaking users"
)]
mod stratum;
mod visit;

pub use formation::Formation;
#[cfg(feature = "macros")]
pub use outcrop_macros::strata;
pub use reading::Reading;
pub use sink::Sink;
pub use strata::Feed;
pub use strata::Get;
pub use strata::Report;
pub use strata::Strata;
pub use stratum::Compute;
pub use stratum::Layered;
pub use visit::Visit;
