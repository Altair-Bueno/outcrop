use alloc::boxed::Box;
use alloc::vec::Vec;

use proc_macro2::Span;
use syn::Attribute;
use syn::Expr;
use syn::Ident;
use syn::Meta;
use syn::Token;
use syn::Type;
use syn::braced;
use syn::parenthesized;
use syn::parse::Parse;
use syn::parse::ParseStream;
use syn::parse::discouraged::Speculative as _;
use syn::parse_quote;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned as _;

use crate::Declaration;
use crate::Entry;
use crate::Field;
use crate::Function;
use crate::Input;
use crate::Kind;
use crate::Memo;
use crate::Router;
use crate::View;

impl Parse for Declaration {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let mut krate = None;
        let mut attrs = input.call(Attribute::parse_outer)?;
        for attr in attrs.iter().filter(|attr| attr.path().is_ident("strata")) {
            attr.parse_nested_meta(|meta| {
                if !meta.path.is_ident("crate") {
                    return Err(meta.error("expected `crate = path::to::outcrop`"));
                }
                if krate.is_some() {
                    return Err(meta.error("`crate` is given more than once"));
                }
                krate = Some(meta.value()?.parse()?);
                Ok(())
            })?;
        }
        attrs.retain(|attr| !attr.path().is_ident("strata"));
        let krate = krate.unwrap_or_else(|| parse_quote!(::outcrop));
        let vis = input.parse()?;
        input.parse::<Token![struct]>()?;
        let ident = input.parse()?;
        let content;
        braced!(content in input);
        let entries = Punctuated::<Entry, Token![,]>::parse_terminated(&content)?;
        Ok(Self {
            attrs,
            krate,
            vis,
            ident,
            entries: entries.into_iter().collect(),
        })
    }
}

/// The attributes of an entry, with the `#[strata(..)]` options and the doc
/// comments taken out.
#[derive(Default)]
struct Options {
    attrs: Vec<Attribute>,
    docs: Vec<Meta>,
    field_attrs: Vec<Meta>,
    fn_attrs: Vec<Meta>,
    first_field: Option<Span>,
    first_fn: Option<Span>,
    bank: Option<Span>,
}

impl Parse for Options {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let mut options = Self::default();
        for attr in input.call(Attribute::parse_outer)? {
            if attr.path().is_ident("doc") {
                options.docs.push(attr.meta);
                continue;
            }
            if !attr.path().is_ident("strata") {
                options.attrs.push(attr);
                continue;
            }
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("bank") {
                    options.bank.get_or_insert(meta.path.span());
                    return Ok(());
                }
                let target = if meta.path.is_ident("fn") {
                    options.first_fn.get_or_insert(meta.path.span());
                    &mut options.fn_attrs
                } else if meta.path.is_ident("field") {
                    options.first_field.get_or_insert(meta.path.span());
                    &mut options.field_attrs
                } else {
                    return Err(meta.error("expected `bank`, `fn(..)` or `field(..)`"));
                };
                let content;
                parenthesized!(content in meta.input);
                target.extend(content.parse_terminated(Meta::parse, Token![,])?);
                Ok(())
            })?;
        }
        Ok(options)
    }
}

impl Parse for Entry {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let Options {
            attrs,
            docs,
            mut field_attrs,
            mut fn_attrs,
            first_field,
            first_fn,
            bank,
        } = input.parse()?;
        let name = if input.peek(Ident) && input.peek2(Token![:]) && !input.peek2(Token![::]) {
            let name: Ident = input.parse()?;
            input.parse::<Token![:]>()?;
            Some(name)
        } else {
            None
        };
        if name.is_some() {
            field_attrs.splice(0..0, docs);
        } else {
            fn_attrs.splice(0..0, docs);
        }
        let field = |name| Field {
            name,
            attrs: field_attrs,
        };
        let function = |f| Function { attrs: fn_attrs, f };
        let shape = input.parse()?;
        if let Some(span) = bank
            && !matches!(shape, Shape::Stored { .. })
        {
            return Err(syn::Error::new(
                span,
                "only inputs can be banks: `#[strata(bank)] name: [T; N] = initial`",
            ));
        }
        let kind = match (shape, name) {
            (Shape::Stored { ty, .. }, None) => {
                return Err(syn::Error::new_spanned(
                    ty,
                    "inputs are stored, so they need a field name: `name: Type = initial`",
                ));
            }
            (Shape::Routed { .. }, Some(name)) => {
                return Err(syn::Error::new(
                    name.span(),
                    "routers have no field, so they cannot be named",
                ));
            }
            (Shape::Stored { ty, init }, Some(name)) => {
                if let Some(span) = first_fn {
                    return Err(syn::Error::new(
                        span,
                        "inputs have no function, so `fn(..)` does not apply to them",
                    ));
                }
                let input = Input {
                    field: field(name),
                    ty,
                    init,
                };
                if bank.is_some() {
                    Kind::Bank(input)
                } else {
                    Kind::Input(input)
                }
            }
            (Shape::Derived { ty, deps, f }, Some(name)) => Kind::Memo(Memo {
                field: field(name),
                function: function(f),
                ty,
                deps: *deps,
            }),
            (Shape::Derived { ty, deps, f }, None) => {
                fieldless("views", first_field)?;
                Kind::View(View {
                    function: function(f),
                    ty,
                    deps: *deps,
                })
            }
            (Shape::Routed { ty, f }, None) => {
                fieldless("routers", first_field)?;
                Kind::Router(Router {
                    function: function(f),
                    ty,
                })
            }
        };
        Ok(Self { attrs, kind })
    }
}

fn fieldless(kind: &str, first_field: Option<Span>) -> syn::Result<()> {
    match first_field {
        Some(span) => Err(syn::Error::new(
            span,
            format!("{kind} have no field, so `field(..)` does not apply to them"),
        )),
        None => Ok(()),
    }
}

enum Shape {
    Stored { ty: Type, init: Expr },
    Derived { ty: Type, deps: Box<Type>, f: Expr },
    Routed { ty: Type, f: Expr },
}

impl Parse for Shape {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let ty = input.parse()?;
        if input.peek(Token![=>]) {
            input.parse::<Token![=>]>()?;
            return Ok(Self::Routed {
                ty,
                f: input.parse()?,
            });
        }
        input.parse::<Token![=]>()?;
        let fork = input.fork();
        if let Ok(deps) = fork.parse::<Type>()
            && fork.peek(Token![=>])
        {
            input.advance_to(&fork);
            input.parse::<Token![=>]>()?;
            return Ok(Self::Derived {
                ty,
                deps: Box::new(deps),
                f: input.parse()?,
            });
        }
        Ok(Self::Stored {
            ty,
            init: input.parse()?,
        })
    }
}

#[cfg(test)]
mod tests {
    use alloc::string::String;
    use alloc::string::ToString as _;

    use proc_macro2::TokenStream;
    use quote::quote;

    use crate::Declaration;

    fn error(input: TokenStream) -> Option<String> {
        syn::parse2::<Declaration>(input)
            .err()
            .map(|error| error.to_string())
    }

    #[test]
    fn rejects_unnamed_inputs() {
        let error = error(quote!(struct S { A = A(0) }));
        assert_eq!(
            error.as_deref(),
            Some("inputs are stored, so they need a field name: `name: Type = initial`")
        );
    }

    #[test]
    fn rejects_field_attributes_on_routers() {
        let error = error(quote!(struct S { #[strata(field(serde(skip)))] [u8] => f }));
        assert_eq!(
            error.as_deref(),
            Some("routers have no field, so `field(..)` does not apply to them")
        );
    }

    #[test]
    fn rejects_fn_attributes_on_inputs() {
        let error = error(quote!(
            struct S { #[strata(fn(inline))] a: A = A(0) }
        ));
        assert_eq!(
            error.as_deref(),
            Some("inputs have no function, so `fn(..)` does not apply to them")
        );
    }

    #[test]
    fn rejects_unknown_attribute_targets() {
        let error = error(quote!(
            struct S { #[strata(impl(inline))] a: A = A(0) }
        ));
        assert_eq!(
            error.as_deref(),
            Some("expected `bank`, `fn(..)` or `field(..)`")
        );
    }

    #[test]
    fn rejects_banks_that_are_not_inputs() {
        for input in [
            quote!(struct S { #[strata(bank)] a: A = B => f }),
            quote!(struct S { #[strata(bank)] A = B => f }),
            quote!(struct S { #[strata(bank)] [u8] => f }),
        ] {
            assert_eq!(
                error(input).as_deref(),
                Some("only inputs can be banks: `#[strata(bank)] name: [T; N] = initial`")
            );
        }
    }

    #[test]
    fn rejects_named_routers() {
        let error = error(quote!(struct S { frame: [u8] => f }));
        assert_eq!(
            error.as_deref(),
            Some("routers have no field, so they cannot be named")
        );
    }

    #[test]
    fn rejects_repeated_crate_options() {
        let error = error(quote!(
            #[strata(crate = a)]
            #[strata(crate = b)]
            struct S {}
        ));
        assert_eq!(error.as_deref(), Some("`crate` is given more than once"));
    }

    #[test]
    fn rejects_unknown_options() {
        let error = error(quote!(
            #[strata(krate = layers)]
            struct S {}
        ));
        assert_eq!(
            error.as_deref(),
            Some("expected `crate = path::to::outcrop`")
        );
    }
}
