use alloc::string::String;
use alloc::vec::Vec;

use proc_macro2::Span;
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
use syn::Path;
use syn::Type;
use syn::spanned::Spanned as _;

use crate::Declaration;
use crate::Entry;
use crate::Kind;

#[derive(Clone)]
struct Signal {
    cfgs: Vec<Attribute>,
    key: String,
    field: Ident,
    byte: TokenStream,
    mask: TokenStream,
    sources: Vec<usize>,
}

impl Signal {
    fn local(&self) -> Ident {
        format_ident!("__outcrop_{}", self.field, span = Span::mixed_site())
    }
}

struct Field<'e> {
    cfgs: &'e [Attribute],
    attrs: &'e [Attribute],
    name: Option<&'e Ident>,
}

impl<'e> From<&'e Entry> for Field<'e> {
    fn from(entry: &'e Entry) -> Self {
        Self {
            cfgs: &entry.cfgs,
            attrs: &entry.attrs,
            name: entry.name.as_ref(),
        }
    }
}

struct Fed {
    cfgs: Vec<Attribute>,
    key: String,
    ty: Type,
}

struct Expansion<'d> {
    ident: &'d Ident,
    krate: &'d Path,
    flags: TokenStream,
    next: TokenStream,
    bits: Vec<TokenStream>,
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
        let flags = Ident::new("__OutcropFlags", Span::mixed_site());
        let mut expansion = Expansion {
            ident: &self.ident,
            krate: &self.krate,
            flags: flags.to_token_stream(),
            next: quote!(0),
            bits: Vec::new(),
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
            let field = Field::from(entry);
            match &entry.kind {
                Kind::Input { ty, init } => expansion.input(&field, ty, init),
                Kind::Derived { ty, deps, f } => expansion.derived(&field, ty, deps, f),
                Kind::Router { ty, f } => expansion.router(&entry.cfgs, position, ty, f),
            }
        }
        if let Some(errors) = expansion.errors {
            return errors.to_compile_error();
        }
        let Self { attrs, vis, .. } = self;
        let cfgs = attrs.iter().filter(|attr| attr.path().is_ident("cfg"));
        let Expansion {
            ident,
            krate,
            next: count,
            bits,
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
        let names = signals.iter().map(|signal| {
            let (cfgs, field, local) = (&signal.cfgs, &signal.field, signal.local());
            quote!(#(#cfgs)* #field: #local)
        });
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

            #(#cfgs)*
            const _: () = {
            #(#bits)*

            type #flags = [u8; (#count).div_ceil(8)];

            #(#helpers)*

            impl ::core::default::Default for #ident {
                fn default() -> Self {
                    #(#defaults)*
                    Self { #(#names,)* }
                }
            }

            impl #krate::Strata for #ident {
                type Flags = #flags;

                const CHANGED: #flags = {
                    let mut flags = [0; (#count).div_ceil(8)];
                    let mut bit = 0;
                    while bit < #count {
                        flags[bit / 8] |= 1 << (bit % 8);
                        bit += 1;
                    }
                    flags
                };

                fn evaluate(&mut self, #changed_flags: &mut #flags) {
                    #(#evaluations)*
                }
            }

            impl<__OutcropVisitor> #krate::Report<__OutcropVisitor> for #ident
            where
                __OutcropVisitor: #(#visits)+*,
            {
                fn report(&self, changed: &#flags, visitor: &mut __OutcropVisitor) {
                    #(#reports)*
                }
            }

            #(#impls)*
            };
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

    fn signal(&mut self, field: &Field<'_>, ty: &Type) -> Option<usize> {
        let Field { cfgs, attrs, name } = *field;
        let key = key(ty);
        if self.signals.iter().any(|signal| signal.key == key) {
            self.error(Error::new(
                ty.span(),
                format_args!("`{key}` is already a signal of this formation"),
            ));
            return None;
        }
        let index = self.signals.len();
        let field = name.cloned().unwrap_or_else(|| format_ident!("s{index}"));
        let bit = format_ident!("__OUTCROP_BIT{index}", span = Span::mixed_site());
        let next = &self.next;
        self.bits.push(quote!(const #bit: usize = #next;));
        self.next = if cfgs.is_empty() {
            quote!(#bit + 1)
        } else {
            let predicates = predicates(cfgs);
            quote!(#bit + ::core::cfg!(all(#(#predicates),*)) as usize)
        };
        let byte = quote!(#bit / 8);
        let mask = quote!(1 << (#bit % 8));
        self.fields.push(quote!(#(#cfgs)* #(#attrs)* #field: #ty));
        let krate = self.krate;
        let visit = quote!(#krate::Visit<#ty>);
        self.reports.push(quote! {
            #(#cfgs)*
            if changed[#byte] & #mask != 0 {
                <__OutcropVisitor as #visit>::visit(visitor, &self.#field);
            }
        });
        if cfgs.is_empty() {
            self.visits.push(visit);
        } else {
            let bound = format_ident!("__OutcropVisit{index}", span = Span::mixed_site());
            let absent = absent(cfgs);
            self.helpers.push(quote! {
                #(#cfgs)*
                trait #bound: #visit {}
                #(#cfgs)*
                impl<__OutcropVisitor: ?Sized + #visit> #bound for __OutcropVisitor {}
                #absent
                trait #bound {}
                #absent
                impl<__OutcropVisitor: ?Sized> #bound for __OutcropVisitor {}
            });
            self.visits.push(bound.into_token_stream());
        }
        self.signals.push(Signal {
            cfgs: cfgs.to_vec(),
            key,
            field,
            byte,
            mask,
            sources: Vec::new(),
        });
        Some(index)
    }

    fn fed(&mut self, cfgs: &[Attribute], ty: &Type) -> Option<Fed> {
        let key = key(ty);
        if self.fed.iter().any(|fed| fed.key == key) {
            self.error(Error::new(
                ty.span(),
                format_args!("`{key}` is already fed to this formation"),
            ));
            return None;
        }
        Some(Fed {
            cfgs: cfgs.to_vec(),
            key,
            ty: ty.clone(),
        })
    }

    fn input(&mut self, field: &Field<'_>, ty: &Type, init: &Expr) {
        let cfgs = field.cfgs;
        let (Some(index), Some(fed)) = (self.signal(field, ty), self.fed(cfgs, ty)) else {
            return;
        };
        let signal = self.signals[index].clone();
        let Signal {
            field, byte, mask, ..
        } = &signal;
        let local = signal.local();
        let (ident, krate, flags) = (self.ident, self.krate, &self.flags);
        let fed_ty = &fed.ty;
        let differs = quote_spanned!(ty.span()=> self.#field != value);
        let implementation = quote! {
            #(#cfgs)*
            impl #krate::Feed<#fed_ty, ()> for #ident {
                fn feed(&mut self, changed: &mut #flags, value: #fed_ty) {
                    if #differs {
                        self.#field = value;
                        changed[#byte] |= #mask;
                    }
                }
            }
        };
        self.impls.push(implementation);
        self.defaults
            .push(quote!(#(#cfgs)* let #local: #ty = #init;));
        self.fed.push(fed);
        self.signals[index].sources.push(index);
        self.get(index, ty, &[]);
    }

    fn derived(&mut self, field: &Field<'_>, ty: &Type, deps: &Type, f: &Expr) {
        let cfgs = field.cfgs;
        let deps = dependencies(deps);
        let mut dirty = Vec::new();
        let mut arguments = Vec::new();
        let mut values = Vec::new();
        let mut sources = Vec::new();
        for dep in &deps {
            let key = key(dep);
            if let Some(index) = self.signals.iter().position(|signal| signal.key == key) {
                let signal = &self.signals[index];
                let (byte, mask) = (&signal.byte, &signal.mask);
                dirty.push(quote!(changed[#byte] & #mask != 0));
                sources.extend_from_slice(&signal.sources);
                let (field, local) = (&signal.field, signal.local());
                arguments.push(quote!(&self.#field));
                values.push(quote!(&#local));
            } else {
                self.error(Error::new(
                    dep.span(),
                    format_args!("`{key}` is not a signal declared above"),
                ));
            }
        }
        let Some(index) = self.signal(field, ty) else {
            return;
        };
        let helper = format_ident!("__outcrop_derive_{index}");
        let parameters = deps.iter().map(|dep| quote!(&#dep)).collect::<Vec<_>>();
        match function(&helper, &parameters, &quote!(-> #ty), f) {
            Ok(function) => self.helpers.push(quote!(#(#cfgs)* #function)),
            Err(error) => self.error(error),
        }
        let signal = self.signals[index].clone();
        let Signal {
            field, byte, mask, ..
        } = &signal;
        let local = signal.local();
        self.defaults
            .push(quote!(#(#cfgs)* let #local: #ty = #helper(#(#values),*);));
        let differs = quote_spanned!(ty.span()=> value != self.#field);
        self.evaluations.push(quote! {
            #(#cfgs)*
            if #(#dirty)||* {
                let value = #helper(#(#arguments),*);
                if #differs {
                    self.#field = value;
                    changed[#byte] |= #mask;
                }
            }
        });
        sources.sort_unstable();
        sources.dedup();
        self.get(index, ty, &sources);
        self.signals[index].sources = sources;
    }

    fn get(&mut self, index: usize, ty: &Type, sources: &[usize]) {
        let (ident, krate, flags) = (self.ident, self.krate, &self.flags);
        let Signal { cfgs, field, .. } = &self.signals[index];
        let reading = quote!(#krate::Reading);
        let (changed, body) = if sources.is_empty() {
            (quote!(_), quote!(#reading::Fresh(&self.#field)))
        } else {
            let stale = sources.iter().map(|source| {
                let Signal { byte, mask, .. } = &self.signals[*source];
                quote!(changed[#byte] & #mask != 0)
            });
            let body = quote! {
                if #(#stale)||* {
                    #reading::Stale(&self.#field)
                } else {
                    #reading::Fresh(&self.#field)
                }
            };
            (quote!(changed), body)
        };
        let implementation = quote! {
            #(#cfgs)*
            impl #krate::Get<#ty, ()> for #ident {
                fn get(&self, #changed: &#flags) -> #reading<&#ty> {
                    #body
                }
            }
        };
        self.impls.push(implementation);
    }

    fn router(&mut self, cfgs: &[Attribute], position: usize, ty: &Type, f: &Expr) {
        let Some(fed) = self.fed(cfgs, ty) else {
            return;
        };
        let (ident, krate, flags) = (self.ident, self.krate, self.flags.clone());
        let view = format_ident!("__OutcropBelow{position}");
        let helper = format_ident!("__outcrop_route_{position}");
        let sink = quote!(&mut #krate::Sink<'_, #view<'_>>);
        match function(&helper, &[quote!(#ty), sink], &TokenStream::new(), f) {
            Ok(function) => self.helpers.push(quote!(#(#cfgs)* #function)),
            Err(error) => self.error(error),
        }
        let below = self.fed.iter().map(
            |Fed {
                 cfgs: fed_cfgs, ty, ..
             }| {
                quote! {
                    #(#cfgs)*
                    #(#fed_cfgs)*
                    impl #krate::Feed<#ty, ()> for #view<'_> {
                        fn feed(&mut self, changed: &mut #flags, value: #ty) {
                            <#ident as #krate::Feed<#ty, ()>>::feed(self.0, changed, value);
                        }
                    }
                }
            },
        );
        let fed_ty = &fed.ty;
        self.impls.push(quote! {
            #(#cfgs)*
            struct #view<'s>(&'s mut #ident);

            #(#cfgs)*
            impl #krate::Strata for #view<'_> {
                type Flags = #flags;

                const CHANGED: #flags = <#ident as #krate::Strata>::CHANGED;

                fn evaluate(&mut self, _: &mut #flags) {}
            }

            #(#below)*

            #(#cfgs)*
            impl #krate::Feed<#fed_ty, ()> for #ident {
                fn feed(&mut self, changed: &mut #flags, value: #fed_ty) {
                    #helper(value, &mut #krate::Sink::new(&mut #view(self), changed));
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
    if let Type::Tuple(tuple) = deps
        && !tuple.elems.is_empty()
    {
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
        .map(|index| format_ident!("__outcrop_a{index}", span = Span::mixed_site()))
        .collect::<Vec<_>>();
    Ok(quote! {
        fn #name(#(#arguments: #parameters),*) #output {
            (#f)(#(#arguments),*)
        }
    })
}

fn predicates(cfgs: &[Attribute]) -> impl Iterator<Item = TokenStream> {
    cfgs.iter().map(|attr| match attr.meta.require_list() {
        Ok(list) => list.tokens.clone(),
        Err(error) => error.to_compile_error(),
    })
}

fn absent(cfgs: &[Attribute]) -> TokenStream {
    let predicates = predicates(cfgs);
    quote!(#[cfg(not(all(#(#predicates),*)))])
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
    fn reads_unit_dependencies_as_the_unit_signal() -> syn::Result<()> {
        let output = expand(quote!(struct S { B = () => f }))?;
        assert!(
            output.contains("`()` is not a signal declared above"),
            "{output}"
        );
        Ok(())
    }

    #[test]
    fn forwards_signal_attributes_to_their_field_only() -> syn::Result<()> {
        let output = expand(quote!(struct S {
            #[serde(rename = "a")] A = A(0),
            #[serde(skip)] B = A => f,
        }))?;
        assert!(
            output.contains("# [serde (rename = \"a\")] s0 : A"),
            "{output}"
        );
        assert!(output.contains("# [serde (skip)] s1 : B"), "{output}");
        assert_eq!(output.matches("# [serde").count(), 2, "{output}");
        Ok(())
    }

    #[test]
    fn names_fields_when_asked_and_by_position_otherwise() -> syn::Result<()> {
        let output = expand(quote!(struct S {
            a: A = A(0),
            B = A => f,
            c: outcrop::C = (A, B) => g,
        }))?;
        assert!(output.contains("a : A"), "{output}");
        assert!(output.contains("s1 : B"), "{output}");
        assert!(output.contains("c : outcrop :: C"), "{output}");
        Ok(())
    }

    #[test]
    fn rejects_named_routers() {
        let error = expand(quote!(struct S { frame: &[u8] => f }))
            .err()
            .map(|error| error.to_string());
        assert_eq!(
            error.as_deref(),
            Some("routers have no field, so they cannot be named")
        );
    }

    #[test]
    fn rejects_attributes_other_than_cfg_on_routers() {
        let error = expand(quote!(struct S { #[serde(skip)] &[u8] => f }))
            .err()
            .map(|error| error.to_string());
        assert_eq!(
            error.as_deref(),
            Some("routers have no field, so only `cfg` attributes are allowed on them")
        );
    }

    #[test]
    fn negates_every_cfg_of_an_entry_for_its_absent_bound() -> syn::Result<()> {
        let output = expand(quote!(struct S { #[cfg(a)] #[cfg(b)] A = A(0) }))?;
        assert!(output.contains("# [cfg (not (all (a , b)))]"), "{output}");
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

    #[test]
    fn uses_the_given_crate_path() -> syn::Result<()> {
        let output = expand(quote!(
            #[strata(crate = layers)]
            struct S { A = A(0), B = A => f, &[u8] => g }
        ))?;
        assert!(output.contains("impl layers :: Strata for S"), "{output}");
        assert!(output.contains("layers :: Sink"), "{output}");
        assert!(!output.contains("outcrop ::"), "{output}");
        assert!(!output.contains("# [strata"), "{output}");
        Ok(())
    }

    #[test]
    fn rejects_repeated_crate_options() {
        let error = expand(quote!(
            #[strata(crate = a)]
            #[strata(crate = b)]
            struct S {}
        ))
        .err()
        .map(|error| error.to_string());
        assert_eq!(error.as_deref(), Some("`crate` is given more than once"));
    }

    #[test]
    fn forwards_cfg_attributes_to_the_generated_items() -> syn::Result<()> {
        let output = expand(quote!(#[cfg(feature = "x")] #[derive(Debug)] struct S { A = A(0) }))?;
        assert_eq!(
            output.matches("# [cfg (feature = \"x\")]").count(),
            2,
            "{output}"
        );
        assert_eq!(output.matches("# [derive (Debug)]").count(), 1, "{output}");
        Ok(())
    }

    #[test]
    fn rejects_unknown_options() {
        let error = expand(quote!(
            #[strata(krate = layers)]
            struct S {}
        ))
        .err()
        .map(|error| error.to_string());
        assert_eq!(
            error.as_deref(),
            Some("expected `crate = path::to::outcrop`")
        );
    }
}
