use alloc::boxed::Box;

use syn::Attribute;
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
            return Ok(Self::Derived {
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
