use alloc::boxed::Box;
use alloc::vec::Vec;

use syn::Attribute;
use syn::Ident;
use syn::Token;
use syn::Type;
use syn::braced;
use syn::parse::Parse;
use syn::parse::ParseStream;
use syn::parse::discouraged::Speculative as _;
use syn::parse_quote;
use syn::punctuated::Punctuated;

use crate::Declaration;
use crate::Entry;
use crate::Kind;

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
        let (cfgs, attrs): (Vec<_>, Vec<_>) = input
            .call(Attribute::parse_outer)?
            .into_iter()
            .partition(|attr| attr.path().is_ident("cfg"));
        let name = if input.peek(Ident) && input.peek2(Token![:]) && !input.peek2(Token![::]) {
            let name: Ident = input.parse()?;
            input.parse::<Token![:]>()?;
            Some(name)
        } else {
            None
        };
        let kind = match (input.parse()?, &name) {
            (Kind::Memo { ty, deps, f }, None) => Kind::View { ty, deps, f },
            (Kind::Input { ty, .. }, None) => {
                return Err(syn::Error::new_spanned(
                    ty,
                    "inputs are stored, so they need a field name: `name: Type = initial`",
                ));
            }
            (Kind::Router { .. }, Some(name)) => {
                return Err(syn::Error::new(
                    name.span(),
                    "routers have no field, so they cannot be named",
                ));
            }
            (kind, _) => kind,
        };
        Ok(Self {
            cfgs,
            attrs,
            name,
            kind,
        })
    }
}

impl Parse for Kind {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let ty = input.parse()?;
        if input.peek(Token![=>]) {
            input.parse::<Token![=>]>()?;
            return Ok(Self::Router {
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
            return Ok(Self::Memo {
                ty,
                deps: Box::new(deps),
                f: input.parse()?,
            });
        }
        Ok(Self::Input {
            ty,
            init: input.parse()?,
        })
    }
}
