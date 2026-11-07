//! The [`strata!`] macro, re-exported by
//! [`outcrop`](https://docs.rs/outcrop) behind its `macros` feature.

extern crate alloc;

mod expand;
mod parse;

use alloc::boxed::Box;
use alloc::vec::Vec;

use quote::ToTokens as _;
use syn::Attribute;
use syn::Expr;
use syn::Ident;
use syn::Path;
use syn::Type;
use syn::Visibility;

struct Declaration {
    attrs: Vec<Attribute>,
    krate: Path,
    vis: Visibility,
    ident: Ident,
    entries: Vec<Entry>,
}

enum Entry {
    Input { ty: Type, init: Expr },
    Derived { ty: Type, deps: Box<Type>, f: Expr },
    Router { ty: Type, f: Expr },
}

/// Declares a named strata type, for formations whose type must be written
/// out, such as one kept in a `static`.
///
/// Each entry is one of:
///
/// - `Signal = initial`: an input with its initial value.
/// - `Signal = Deps => f`: a signal computed by `f` from `Deps`, a signal
///   declared above or a tuple of them. `f` is a path to a function or a
///   closure that captures nothing, taking each dependency by reference.
/// - `In => f`: a router, called with every fed `In` and an [`outcrop::Sink`](https://docs.rs/outcrop)
///   feeding the inputs and routers declared above it.
///
/// Attributes are kept, so the struct can derive `Clone`, `PartialEq` and
/// other traits its signals implement. `Default` is implemented from the
/// initial values, and wrapping the struct in a `Formation` marks every signal
/// as changed.
///
/// The generated code refers to `::outcrop`. Where the dependency is renamed
/// or re-exported, pass its path with `#[strata(crate = path::to::outcrop)]`.
///
/// ```
/// use core::fmt::Debug;
/// use outcrop::{Formation, Visit};
///
/// #[derive(Debug, Clone, PartialEq)]
/// struct Temperature(i16);
/// #[derive(Debug, Clone, PartialEq)]
/// struct AntiFreeze(bool);
///
/// outcrop::strata! {
///     #[derive(Debug, Clone, PartialEq)]
///     struct Thermostat {
///         Temperature = Temperature(20),
///         AntiFreeze = Temperature => |t| AntiFreeze(t.0 < 10),
///         &[i16] => |samples, sink| {
///             if let Some(last) = samples.last() {
///                 sink.feed(Temperature(*last));
///             }
///         },
///     }
/// }
///
/// struct Print;
///
/// impl<T: Debug> Visit<T> for Print {
///     fn visit(&mut self, value: &T) {
///         println!("{value:?}");
///     }
/// }
///
/// let mut formation = Formation::from(Thermostat::default());
/// formation.poll(&mut Print); // Temperature(20), AntiFreeze(false)
/// formation.feed(&[12, 3][..]);
/// formation.poll(&mut Print); // Temperature(3), AntiFreeze(true)
/// ```
#[proc_macro]
pub fn strata(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    syn::parse_macro_input!(input as Declaration)
        .into_token_stream()
        .into()
}
