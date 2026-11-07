use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

use proc_macro2::Literal;
use proc_macro2::TokenStream;
use quote::ToTokens;
use quote::format_ident;
use quote::quote;
use quote::quote_spanned;
use syn::Attribute;
use syn::Error;
use syn::Expr;
use syn::Ident;
use syn::Pat;
use syn::Type;
use syn::spanned::Spanned as _;

use crate::Declaration;
use crate::Entry;

struct Signal {
    key: String,
    field: Ident,
    byte: Literal,
    mask: Literal,
}

struct Fed {
    key: String,
    ty: Type,
}

struct Expansion<'d> {
    ident: &'d Ident,
    flags: TokenStream,
    signals: Vec<Signal>,
    fed: Vec<Fed>,
    fields: Vec<TokenStream>,
    defaults: Vec<TokenStream>,
    helpers: Vec<TokenStream>,
    evaluations: Vec<TokenStream>,
    impls: Vec<TokenStream>,
    reports: Vec<TokenStream>,
    visits: Vec<TokenStream>,
    errors: Option<Error>,
}

impl ToTokens for Declaration {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(self.expand());
    }
}

impl Declaration {
    fn expand(&self) -> TokenStream {
        let count = self
            .entries
            .iter()
            .filter(|entry| !matches!(entry, Entry::Router { .. }))
            .count();
        let bytes = count.div_ceil(8);
        let length = Literal::usize_unsuffixed(bytes);
        let mut expansion = Expansion {
            ident: &self.ident,
            flags: quote!([u8; #length]),
            signals: Vec::new(),
            fed: Vec::new(),
            fields: Vec::new(),
            defaults: Vec::new(),
            helpers: Vec::new(),
            evaluations: Vec::new(),
            impls: Vec::new(),
            reports: Vec::new(),
            visits: Vec::new(),
            errors: reject_default(&self.attrs),
        };
        for (position, entry) in self.entries.iter().enumerate() {
            match entry {
                Entry::Input { ty, init } => expansion.input(ty, init),
                Entry::Derived { ty, deps, f } => expansion.derived(ty, deps, f),
                Entry::Router { ty, f } => expansion.router(position, ty, f),
            }
        }
        if let Some(errors) = expansion.errors {
            return errors.to_compile_error();
        }
        let changed = (0..bytes).map(|byte| {
            let bits = (count - 8 * byte).min(8);
            Literal::u8_unsuffixed(u8::MAX >> (8 - bits))
        });
        let Self { attrs, vis, .. } = self;
        let Expansion {
            ident,
            flags,
            fields,
            defaults,
            helpers,
            evaluations,
            impls,
            reports,
            visits,
            signals,
            ..
        } = expansion;
        let names = signals.iter().map(|signal| &signal.field);
        let changed_flags = if evaluations.is_empty() {
            quote!(_)
        } else {
            quote!(changed)
        };
        quote! {
            #(#attrs)*
            #vis struct #ident {
                #(#fields,)*
            }

            impl #ident {
                #(#helpers)*
            }

            impl ::core::default::Default for #ident {
                fn default() -> Self {
                    #(#defaults)*
                    Self { #(#names,)* }
                }
            }

            impl ::outcrop::Strata for #ident {
                type Flags = #flags;

                const CHANGED: #flags = [#(#changed),*];

                fn evaluate(&mut self, #changed_flags: &mut #flags) {
                    #(#evaluations)*
                }
            }

            impl<V> ::outcrop::Report<V> for #ident
            where
                V: #(#visits)+*,
            {
                fn report(&self, changed: &#flags, visitor: &mut V) {
                    #(#reports)*
                }
            }

            #(#impls)*
        }
    }
}

impl Expansion<'_> {
    fn error(&mut self, error: Error) {
        match &mut self.errors {
            Some(errors) => errors.combine(error),
            None => self.errors = Some(error),
        }
    }

    fn signal(&mut self, ty: &Type) -> Option<usize> {
        let key = key(ty);
        if self.signals.iter().any(|signal| signal.key == key) {
            self.error(Error::new(
                ty.span(),
                format_args!("`{key}` is already a signal of this formation"),
            ));
            return None;
        }
        let index = self.signals.len();
        let field = format_ident!("s{index}");
        let byte = Literal::usize_unsuffixed(index / 8);
        let mask = Literal::u8_unsuffixed(1 << (index % 8));
        self.fields.push(quote!(#field: #ty));
        self.visits.push(quote!(::outcrop::Visit<#ty>));
        self.reports.push(quote! {
            if changed[#byte] & #mask != 0 {
                <V as ::outcrop::Visit<#ty>>::visit(visitor, &self.#field);
            }
        });
        self.signals.push(Signal {
            key,
            field,
            byte,
            mask,
        });
        Some(index)
    }

    fn fed(&mut self, ty: &Type) -> Option<Fed> {
        let key = key(ty);
        if self.fed.iter().any(|fed| fed.key == key) {
            self.error(Error::new(
                ty.span(),
                format_args!("`{key}` is already fed to this formation"),
            ));
            return None;
        }
        Some(Fed {
            key,
            ty: ty.clone(),
        })
    }

    fn input(&mut self, ty: &Type, init: &Expr) {
        let (Some(index), Some(fed)) = (self.signal(ty), self.fed(ty)) else {
            return;
        };
        let Signal {
            field, byte, mask, ..
        } = &self.signals[index];
        let (field, byte, mask) = (field.clone(), byte.clone(), mask.clone());
        let (ident, flags) = (self.ident, &self.flags);
        let fed_ty = &fed.ty;
        let differs = quote_spanned!(ty.span()=> self.#field != value);
        let implementation = quote! {
            impl ::outcrop::Feed<#fed_ty, ()> for #ident {
                fn feed(&mut self, changed: &mut #flags, value: #fed_ty) {
                    if #differs {
                        self.#field = value;
                        changed[#byte] |= #mask;
                    }
                }
            }
        };
        self.impls.push(implementation);
        self.defaults.push(quote!(let #field: #ty = #init;));
        self.fed.push(fed);
    }

    fn derived(&mut self, ty: &Type, deps: &Type, f: &Expr) {
        let deps = dependencies(deps);
        let mut masks = BTreeMap::<usize, u8>::new();
        let mut arguments = Vec::new();
        let mut values = Vec::new();
        for dep in &deps {
            let key = key(dep);
            if let Some(index) = self.signals.iter().position(|signal| signal.key == key) {
                *masks.entry(index / 8).or_default() |= 1 << (index % 8);
                let field = self.signals[index].field.clone();
                arguments.push(quote!(&self.#field));
                values.push(quote!(&#field));
            } else {
                self.error(Error::new(
                    dep.span(),
                    format_args!("`{key}` is not a signal declared above"),
                ));
            }
        }
        if deps.is_empty() {
            self.error(Error::new(
                f.span(),
                "a derived signal needs at least one dependency",
            ));
        }
        let Some(index) = self.signal(ty) else {
            return;
        };
        let helper = format_ident!("__outcrop_derive_{index}");
        let parameters = deps.iter().map(|dep| quote!(&#dep)).collect::<Vec<_>>();
        match function(&helper, &parameters, &quote!(-> #ty), f) {
            Ok(function) => self.helpers.push(function),
            Err(error) => self.error(error),
        }
        let Signal {
            field, byte, mask, ..
        } = &self.signals[index];
        let (field, byte, mask) = (field.clone(), byte.clone(), mask.clone());
        let dirty = masks.iter().map(|(byte, mask)| {
            let byte = Literal::usize_unsuffixed(*byte);
            let mask = Literal::u8_unsuffixed(*mask);
            quote!(changed[#byte] & #mask != 0)
        });
        self.defaults
            .push(quote!(let #field: #ty = Self::#helper(#(#values),*);));
        let differs = quote_spanned!(ty.span()=> value != self.#field);
        self.evaluations.push(quote! {
            if #(#dirty)||* {
                let value = Self::#helper(#(#arguments),*);
                if #differs {
                    self.#field = value;
                    changed[#byte] |= #mask;
                }
            }
        });
    }

    fn router(&mut self, position: usize, ty: &Type, f: &Expr) {
        let Some(fed) = self.fed(ty) else {
            return;
        };
        let (ident, flags) = (self.ident, self.flags.clone());
        let view = format_ident!("{ident}Below{position}");
        let helper = format_ident!("__outcrop_route_{position}");
        let sink = quote!(&mut ::outcrop::Sink<'_, #view<'_>>);
        match function(&helper, &[quote!(#ty), sink], &TokenStream::new(), f) {
            Ok(function) => self.helpers.push(function),
            Err(error) => self.error(error),
        }
        let below = self.fed.iter().map(|Fed { ty, .. }| {
            quote! {
                impl ::outcrop::Feed<#ty, ()> for #view<'_> {
                    fn feed(&mut self, changed: &mut #flags, value: #ty) {
                        <#ident as ::outcrop::Feed<#ty, ()>>::feed(self.0, changed, value);
                    }
                }
            }
        });
        let fed_ty = &fed.ty;
        self.impls.push(quote! {
            struct #view<'s>(&'s mut #ident);

            impl ::outcrop::Strata for #view<'_> {
                type Flags = #flags;

                const CHANGED: #flags = <#ident as ::outcrop::Strata>::CHANGED;

                fn evaluate(&mut self, _: &mut #flags) {}
            }

            #(#below)*

            impl ::outcrop::Feed<#fed_ty, ()> for #ident {
                fn feed(&mut self, changed: &mut #flags, value: #fed_ty) {
                    Self::#helper(value, &mut ::outcrop::Sink::new(&mut #view(self), changed));
                }
            }
        });
        self.fed.push(fed);
    }
}

fn key(ty: &Type) -> String {
    ty.to_token_stream().to_string()
}

fn dependencies(deps: &Type) -> Vec<Type> {
    if let Type::Tuple(tuple) = deps {
        tuple.elems.iter().cloned().collect()
    } else if let Type::Paren(paren) = deps {
        Vec::from([(*paren.elem).clone()])
    } else {
        Vec::from([deps.clone()])
    }
}

fn function(
    name: &Ident,
    parameters: &[TokenStream],
    output: &TokenStream,
    f: &Expr,
) -> syn::Result<TokenStream> {
    if let Expr::Closure(closure) = f {
        if closure.inputs.len() != parameters.len() {
            return Err(Error::new(
                closure.inputs.span(),
                format_args!(
                    "expected {} closure parameters, found {}",
                    parameters.len(),
                    closure.inputs.len()
                ),
            ));
        }
        let inputs = closure.inputs.iter().zip(parameters).map(|(pat, ty)| {
            if let Pat::Type(typed) = pat {
                typed.to_token_stream()
            } else {
                quote!(#pat: #ty)
            }
        });
        let body = &closure.body;
        return Ok(quote! {
            fn #name(#(#inputs),*) #output {
                #body
            }
        });
    }
    let arguments = (0..parameters.len())
        .map(|index| format_ident!("a{index}"))
        .collect::<Vec<_>>();
    Ok(quote! {
        fn #name(#(#arguments: #parameters),*) #output {
            (#f)(#(#arguments),*)
        }
    })
}

fn reject_default(attrs: &[Attribute]) -> Option<Error> {
    attrs
        .iter()
        .filter(|attr| attr.path().is_ident("derive"))
        .find_map(|attr| {
            attr.parse_nested_meta(|meta| {
                if meta
                    .path
                    .segments
                    .last()
                    .is_some_and(|segment| segment.ident == "Default")
                {
                    return Err(meta
                        .error("`strata!` implements `Default` from the declared initial values"));
                }
                Ok(())
            })
            .err()
        })
}

#[cfg(test)]
mod tests {
    use alloc::string::String;
    use alloc::string::ToString as _;

    use proc_macro2::TokenStream;
    use quote::ToTokens as _;
    use quote::quote;

    use crate::Declaration;

    fn expand(input: TokenStream) -> syn::Result<String> {
        Ok(syn::parse2::<Declaration>(input)?
            .into_token_stream()
            .to_string())
    }

    #[test]
    fn rejects_duplicate_signals() -> syn::Result<()> {
        let output = expand(quote!(struct S { A = A(0), A = A(1), A = A => f }))?;
        assert!(
            output.contains("`A` is already a signal of this formation"),
            "{output}"
        );
        Ok(())
    }

    #[test]
    fn rejects_types_fed_twice() -> syn::Result<()> {
        let output = expand(quote!(struct S { &[u8] => f, &[u8] => g }))?;
        assert!(
            output.contains("`& [u8]` is already fed to this formation"),
            "{output}"
        );
        Ok(())
    }

    #[test]
    fn rejects_dependencies_not_declared_above() -> syn::Result<()> {
        let output = expand(quote!(struct S { B = A => f, A = A(0) }))?;
        assert!(
            output.contains("`A` is not a signal declared above"),
            "{output}"
        );
        Ok(())
    }

    #[test]
    fn rejects_derived_signals_without_dependencies() -> syn::Result<()> {
        let output = expand(quote!(struct S { B = () => f }))?;
        assert!(
            output.contains("a derived signal needs at least one dependency"),
            "{output}"
        );
        Ok(())
    }

    #[test]
    fn rejects_closures_of_the_wrong_arity() -> syn::Result<()> {
        let output =
            expand(quote!(struct S { A = A(0), B = A => |a, b| B(a.0), &[u8] => |frame| {} }))?;
        assert!(
            output.contains("expected 1 closure parameters, found 2"),
            "{output}"
        );
        assert!(
            output.contains("expected 2 closure parameters, found 1"),
            "{output}"
        );
        Ok(())
    }

    #[test]
    fn rejects_derived_default() -> syn::Result<()> {
        let output = expand(quote!(
            #[derive(Clone, Default)]
            struct S {}
        ))?;
        assert!(
            output.contains("`strata!` implements `Default`"),
            "{output}"
        );
        Ok(())
    }
}
