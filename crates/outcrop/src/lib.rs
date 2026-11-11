//! Static, `no_std`, sans-IO incremental evaluation of context-dependent
//! signals.
//!
//! A [`Formation`] is a dependency graph of signals built at compile time,
//! one stratum at a time. Every signal is one of:
//!
//! - an **input**, fed by the caller;
//! - a **memo**, computed from the signals beneath it and cached, for values
//!   that are expensive to compute;
//! - a **view**, computed from the signals beneath it every time it is read,
//!   for values cheap enough not to be stored.
//!
//! [`Formation::poll`] reports every input fed a different value and every
//! memo whose value changed since the last poll, and every view with a
//! changed dependency. Nothing allocates: the whole graph
//! lives in a single value whose type encodes its shape.
//!
//! ```
//! use core::fmt::Debug;
//! use outcrop::{Formation, Visit};
//!
//! #[derive(Debug, PartialEq, Eq)]
//! struct Temperature(i16);
//! #[derive(Debug, PartialEq, Eq)]
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
//!     .view(|t: &Temperature| AntiFreeze(t.0 < 10));
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

#[cfg(feature = "macros")]
#[doc(hidden)]
#[expect(
    clippy::must_use_candidate,
    reason = "support for the code `strata!` generates, not public API"
)]
pub mod __private;
mod bank;
#[cfg(feature = "macros")]
#[expect(
    unnameable_types,
    reason = "borrowed layers only appear in the chain `strata!` builds"
)]
mod borrowed;
mod compute;
#[expect(
    unnameable_types,
    reason = "change flags sized by the strata, unnameable so they stay an implementation detail"
)]
mod flags;
mod formation;
#[cfg(feature = "macros")]
#[expect(
    unnameable_types,
    reason = "nested structs only appear in inferred formation types"
)]
mod nested;
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

pub use bank::Element;
pub use bank::Slot;
pub use bank::Write;
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
