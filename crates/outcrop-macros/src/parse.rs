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

impl Parse for Entry {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let mut cfgs = Vec::new();
        let mut field_attrs = Vec::new();
        let mut fn_attrs = Vec::new();
        let mut first_field = None;
        let mut first_fn = None;
        for attr in input.call(Attribute::parse_outer)? {
            if attr.path().is_ident("cfg") {
                cfgs.push(attr);
            } else if attr.path().is_ident("strata") {
                attr.parse_nested_meta(|meta| {
                    let target = if meta.path.is_ident("fn") {
                        first_fn.get_or_insert(meta.path.span());
                        &mut fn_attrs
                    } else if meta.path.is_ident("field") {
                        first_field.get_or_insert((meta.path.span(), false));
                        &mut field_attrs
                    } else {
                        return Err(meta.error("expected `fn(..)` or `field(..)`"));
                    };
                    let content;
                    parenthesized!(content in meta.input);
                    target.extend(content.parse_terminated(Meta::parse, Token![,])?);
                    Ok(())
                })?;
            } else {
                first_field.get_or_insert((attr.span(), attr.path().is_ident("doc")));
                field_attrs.push(attr.meta);
            }
        }
        let name = if input.peek(Ident) && input.peek2(Token![:]) && !input.peek2(Token![::]) {
            let name: Ident = input.parse()?;
            input.parse::<Token![:]>()?;
            Some(name)
        } else {
            None
        };
        let field = |name| Field {
            name,
            attrs: field_attrs,
        };
        let function = |f| Function { attrs: fn_attrs, f };
        let kind = match (input.parse()?, name) {
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
                Kind::Input(Input {
                    field: field(name),
                    ty,
                    init,
                })
            }
            (Shape::Derived { ty, deps, f }, Some(name)) => Kind::Memo(Memo {
                field: field(name),
                function: function(f),
                ty,
                deps,
            }),
            (Shape::Derived { ty, deps, f }, None) => {
                fieldless("views", first_field)?;
                Kind::View(View {
                    function: function(f),
                    ty,
                    deps,
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
        Ok(Self { cfgs, kind })
    }
}

fn fieldless(kind: &str, first_field: Option<(Span, bool)>) -> syn::Result<()> {
    let Some((span, doc)) = first_field else {
        return Ok(());
    };
    let message = if doc {
        format!("{kind} have no field to document; use a `//` comment instead")
    } else {
        format!("{kind} have no field; put attributes on their function with `#[strata(fn(..))]`")
    };
    Err(syn::Error::new(span, message))
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
