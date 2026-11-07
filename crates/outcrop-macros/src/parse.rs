use alloc::boxed::Box;

use syn::Attribute;
use syn::Token;
use syn::Type;
use syn::braced;
use syn::parse::Parse;
use syn::parse::ParseStream;
use syn::parse::discouraged::Speculative as _;
use syn::punctuated::Punctuated;

use crate::Declaration;
use crate::Entry;

impl Parse for Declaration {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let attrs = input.call(Attribute::parse_outer)?;
        let vis = input.parse()?;
        input.parse::<Token![struct]>()?;
        let ident = input.parse()?;
        let content;
        braced!(content in input);
        let entries = Punctuated::<Entry, Token![,]>::parse_terminated(&content)?;
        Ok(Self {
            attrs,
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
