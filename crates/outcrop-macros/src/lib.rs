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

struct Entry {
    cfgs: Vec<Attribute>,
    attrs: Vec<Attribute>,
    name: Option<Ident>,
    kind: Kind,
}

enum Kind {
    Input { ty: Type, init: Expr },
    Memo { ty: Type, deps: Box<Type>, f: Expr },
    View { ty: Type, deps: Box<Type>, f: Expr },
    Router { ty: Type, f: Expr },
}

/// Declares a named strata type, for formations whose type must be written
/// out, such as one kept in a `static`.
///
/// Each entry is one of:
///
/// - `name: Signal = initial`: an input with its initial value, stored in the
///   field `name`.
/// - `name: Signal = Deps => f`: a memo, computed by `f` from `Deps` and
///   stored in the field `name`. `Deps` is a signal declared above or a tuple
///   of them, and `f` is a path to a function or a closure that captures
///   nothing, taking each dependency by reference.
/// - `Signal = Deps => f`: a view, computed by `f` from `Deps` every time it
///   is read. It has no name because it has no field: it takes no memory.
/// - `In => f`: a router, called with every fed `In` and an [`outcrop::Sink`](https://docs.rs/outcrop)
///   feeding the inputs and routers declared above it.
///
/// So the struct's fields are exactly the stored signals. The attributes of
/// inputs and memos are forwarded to their field. Views and routers have no
/// field, so their attributes, such as `#[expect]`, go on the function
/// generated from `f`. `cfg` attributes apply to everything an entry
/// generates.
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
///         temperature: Temperature = Temperature(20),
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
