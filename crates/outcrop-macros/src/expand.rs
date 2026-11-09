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
use syn::Path;
use syn::Type;
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

struct Signal<'d> {
    attrs: &'d [Attribute],
    field: Option<&'d Ident>,
}

struct Fed<'d> {
    attrs: &'d [Attribute],
    ty: TokenStream,
    marker: Ident,
}

fn local(field: &Ident) -> Ident {
    format_ident!("__outcrop_field_{field}", span = Span::mixed_site())
}

fn mixed(name: &str) -> Ident {
    Ident::new(name, Span::mixed_site())
}

struct Expansion<'d> {
    ident: &'d Ident,
    krate: &'d Path,
    private: TokenStream,
    flags: Ident,
    chain: Ident,
    next: TokenStream,
    bits: Vec<TokenStream>,
    signals: Vec<Signal<'d>>,
    fed: Vec<Fed<'d>>,
    fields: Vec<TokenStream>,
    builds: Vec<TokenStream>,
    takes: Vec<TokenStream>,
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
        let krate = &self.krate;
        let mut expansion = Expansion {
            ident: &self.ident,
            krate,
            private: quote!(#krate::__private),
            flags: mixed("__OutcropFlags"),
            chain: mixed("__outcrop_chain"),
            next: quote!(0usize),
            bits: Vec::new(),
            signals: Vec::new(),
            fed: Vec::new(),
            fields: Vec::new(),
            builds: Vec::new(),
            takes: Vec::new(),
            helpers: Vec::new(),
            evaluations: Vec::new(),
            impls: Vec::new(),
            reports: Vec::new(),
            visits: Vec::new(),
            errors: reject_default(&self.attrs),
        };
        expansion.entries(&self.entries);
        if let Some(errors) = expansion.errors {
            return errors.to_compile_error();
        }
        let Self { attrs, vis, .. } = self;
        let count = mixed("__OUTCROP_BITS");
        let Expansion {
            ident,
            krate,
            flags,
            chain,
            next,
            bits,
            fields,
            builds,
            takes,
            helpers,
            evaluations,
            impls,
            reports,
            visits,
            signals,
            private,
            ..
        } = expansion;
        let names = signals.iter().filter_map(|signal| {
            let field = signal.field?;
            let (attrs, local) = (&signal.attrs, local(field));
            Some(quote!(#(#attrs)* #field: #local))
        });
        let takes = takes.iter().rev();
        let get = lookup_by_type(ident, krate, &private, &flags);
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

            const _: () = {
            #(#bits)*
            const #count: usize = #next;

            type #flags = #private::Chunk<{ #count.div_ceil(8) }>;

            #(#helpers)*

            impl ::core::default::Default for #ident {
                fn default() -> Self {
                    let #chain = ();
                    #(#builds)*
                    #(#takes)*
                    let () = #chain;
                    Self { #(#names,)* }
                }
            }

            impl #krate::Strata for #ident {
                type Flags = #flags;

                const CHANGED: #flags = #private::all();

                fn evaluate(&mut self, #changed_flags: &mut #flags) {
                    #(#evaluations)*
                }
            }

            #get

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

impl<'d> Expansion<'d> {
    fn entries(&mut self, entries: &'d [Entry]) {
        for (position, entry) in entries.iter().enumerate() {
            let attrs = &entry.attrs;
            match &entry.kind {
                Kind::Input(input) => self.input(attrs, input),
                Kind::Bank(bank) => self.bank(attrs, bank),
                Kind::Memo(memo) => self.memo(attrs, memo),
                Kind::View(view) => self.view(attrs, view),
                Kind::Router(router) => self.router(attrs, position, router),
            }
        }
    }

    /// Reserves `width` change bits for the entry numbered `index`, and
    /// bounds the visitor by `visit` if the entry is compiled in. Returns the
    /// constant holding its first bit.
    fn bits(
        &mut self,
        attrs: &[Attribute],
        index: usize,
        width: Option<&TokenStream>,
        visit: TokenStream,
    ) -> Ident {
        let bit = format_ident!("__OUTCROP_BIT{index}", span = Span::mixed_site());
        let next = &self.next;
        self.bits.push(quote!(const #bit: usize = #next;));
        let width = width.map(|width| quote!(* (#width)));
        if attrs.is_empty() {
            self.next = width.map_or_else(|| quote!(#bit + 1), |width| quote!(#bit + 1 #width));
            self.visits.push(visit);
        } else {
            let compiled = format_ident!("__outcrop_compiled{index}", span = Span::mixed_site());
            let bound = format_ident!("__OutcropVisit{index}", span = Span::mixed_site());
            self.bits.push(quote! {
                #[allow(unused_macros)]
                macro_rules! #compiled {
                    (present) => { false };
                    (bound) => {
                        trait #bound {}
                        impl<__OutcropVisitor: ?Sized> #bound for __OutcropVisitor {}
                    };
                }
                #(#attrs)*
                #[allow(unused_macros)]
                macro_rules! #compiled {
                    (present) => { true };
                    (bound) => {
                        trait #bound: #visit {}
                        impl<__OutcropVisitor: ?Sized + #visit> #bound for __OutcropVisitor {}
                    };
                }
                #compiled!(bound);
            });
            self.next = quote!(#bit + #compiled!(present) as usize #width);
            self.visits.push(bound.into_token_stream());
        }
        bit
    }

    /// Declares the change bit, index type and report of a new signal, and
    /// returns its bit and index type.
    fn signal(
        &mut self,
        attrs: &'d [Attribute],
        ty: &Type,
        field: Option<&'d Ident>,
    ) -> (Ident, Ident) {
        let index = self.signals.len();
        let marker = format_ident!("__OutcropSignal{index}", span = Span::mixed_site());
        let (krate, private) = (self.krate, self.private.clone());
        let visit = quote!(#krate::Visit<#ty>);
        let bit = self.bits(attrs, index, None, visit.clone());
        self.marker(attrs, &marker);
        let (chain, signal) = (&self.chain, quote!(#private::Signal<#ty, #marker>));
        self.reports.push(quote! {
            #(#attrs)*
            if #private::is_set(changed, #bit) {
                <__OutcropVisitor as #visit>::visit(
                    visitor,
                    ::core::borrow::Borrow::<#ty>::borrow(&<Self as #signal>::value(self)),
                );
            }
        });
        self.builds.push(quote! {
            #(#attrs)*
            #private::distinct::<#ty, _, _>(&#chain);
        });
        self.signals.push(Signal { attrs, field });
        (bit, marker)
    }

    /// Implements the lookup of a signal by its type, used by the signals
    /// above it, `get` and `poll`.
    fn lookup(
        &mut self,
        attrs: &[Attribute],
        ty: &Type,
        (bit, marker): (&Ident, &Ident),
        (stale, moved): (TokenStream, TokenStream),
        (output, value): (TokenStream, TokenStream),
    ) {
        let (ident, flags, private) = (self.ident, &self.flags, &self.private);
        self.impls.push(quote! {
            #(#attrs)*
            impl #private::Signal<#ty, #marker> for #ident {
                const BIT: usize = #bit;
                const STALE: #flags = #stale;
                const MOVED: #flags = #moved;
                type Ref<'a> = #output where Self: 'a;

                fn value(&self) -> Self::Ref<'_> {
                    #value
                }
            }
        });
    }

    /// The current value of each dependency, borrowed from `self`.
    fn arguments(&self, deps: &[&Type]) -> Vec<TokenStream> {
        let private = &self.private;
        deps.iter()
            .map(|dep| {
                quote! {
                    ::core::borrow::Borrow::<#dep>::borrow(&#private::value::<Self, #dep, _>(self))
                }
            })
            .collect()
    }

    /// The value of each dependency, picked from the signals built so far.
    fn picks(&self, deps: &[&Type], chain: &TokenStream) -> Vec<TokenStream> {
        let private = &self.private;
        deps.iter()
            .map(|dep| {
                quote! {
                    ::core::borrow::Borrow::<#dep>::borrow(&#private::select::<#dep, _, _>(#chain))
                }
            })
            .collect()
    }

    /// Whether any dependency changed since the last poll.
    fn dirty(&self, deps: &[&Type]) -> TokenStream {
        let private = &self.private;
        let checks = deps
            .iter()
            .map(|dep| quote!(#private::is_set(changed, #private::bit::<Self, #dep, _>())));
        quote!(#(#checks)||*)
    }

    fn helper(
        &mut self,
        attrs: &[Attribute],
        function: &Function,
        name: &Ident,
        parameters: &[TokenStream],
        output: &TokenStream,
        bound: &TokenStream,
    ) {
        let Function { attrs: targeted, f } = function;
        let function = self::function(name, parameters, output, f, bound);
        self.helpers
            .push(quote!(#(#attrs)* #(#[#targeted])* #function));
    }

    fn derivation(
        &mut self,
        attrs: &[Attribute],
        function: &Function,
        name: &Ident,
        deps: &[&Type],
        ty: &Type,
    ) {
        let parameters = deps.iter().map(|dep| quote!(&#dep)).collect::<Vec<_>>();
        let bound = quote!(::core::ops::Fn(#(#parameters),*) -> #ty);
        self.helper(attrs, function, name, &parameters, &quote!(-> #ty), &bound);
    }

    fn field(&mut self, attrs: &[Attribute], field: &Field, ty: &Type) {
        let Field {
            name,
            attrs: targeted,
        } = field;
        self.fields
            .push(quote!(#(#attrs)* #(#[#targeted])* #name: #ty));
    }

    /// Declares the type telling apart the lookups of one entry.
    fn marker(&mut self, attrs: &[Attribute], marker: &Ident) {
        self.impls.push(quote! {
            #(#attrs)*
            #[doc(hidden)]
            #[allow(unnameable_types, missing_docs, missing_debug_implementations)]
            pub struct #marker;
        });
    }

    /// Moves the top entry out of the chain built by `Default`, binding
    /// what `split` returns to `(pattern, chain)`.
    fn take(&mut self, attrs: &[Attribute], pattern: &TokenStream, split: &TokenStream) {
        let chain = &self.chain;
        self.takes
            .push(quote!(#(#attrs)* let (#pattern, #chain) = #split;));
    }

    /// Moves a stored value out of the chain built by `Default`.
    fn unstore(&mut self, attrs: &[Attribute], local: &Ident) {
        let (private, chain) = (self.private.clone(), self.chain.clone());
        self.take(attrs, &quote!(#local), &quote!(#private::unstore(#chain)));
    }

    fn input(&mut self, attrs: &'d [Attribute], input: &'d Input) {
        let Input { field, ty, init } = input;
        let name = &field.name;
        let local = local(name);
        let (private, chain) = (self.private.clone(), self.chain.clone());
        self.builds.push(quote! {
            #(#attrs)*
            let #local: #ty = #init;
            #(#attrs)*
            let #chain = #private::input(#local, #chain);
        });
        let (bit, marker) = self.signal(attrs, ty, Some(name));
        self.field(attrs, field, ty);
        self.lookup(
            attrs,
            ty,
            (&bit, &marker),
            (quote!(#private::union([])), quote!(#private::mask(#bit))),
            (quote!(&'a #ty), quote!(&self.#name)),
        );
        let (ident, krate, flags) = (self.ident, self.krate, &self.flags);
        let ne = quote_spanned!(ty.span()=> ::core::cmp::PartialEq::ne);
        self.impls.push(quote! {
            #(#attrs)*
            impl #krate::Feed<#ty, #marker> for #ident {
                fn feed(&mut self, changed: &mut #flags, value: #ty) {
                    if #ne(&self.#name, &value) {
                        self.#name = value;
                        #private::set(changed, #bit);
                    }
                }
            }
        });
        self.unstore(attrs, &local);
        self.fed.push(Fed {
            attrs,
            ty: ty.to_token_stream(),
            marker,
        });
    }

    fn bank(&mut self, attrs: &'d [Attribute], bank: &'d Input) {
        let Input { field, ty, init } = bank;
        let name = &field.name;
        let local = local(name);
        let index = self.signals.len();
        let marker = format_ident!("__OutcropSignal{index}", span = Span::mixed_site());
        let (ident, krate, flags) = (self.ident, self.krate, self.flags.clone());
        let (private, chain) = (self.private.clone(), self.chain.clone());
        let array = quote_spanned!(ty.span()=> <#ty as #private::Array>);
        let item = quote!(#array::Item);
        let len = quote!(#array::LEN);
        let slot = mixed("__OutcropSlot");
        let write = quote!(#krate::Write<'_, #item>);
        let visit = quote!(for<'__outcrop> #krate::Visit<#krate::Element<'__outcrop, #item>>);
        self.builds.push(quote! {
            #(#attrs)*
            let #local: #ty = #init;
            #(#attrs)*
            let #chain = #private::bank::<{ #len.div_ceil(8) }, _, _, _>(#local, #chain);
            #(#attrs)*
            #private::routed::<Self, #krate::Write<'static, #item>, _>();
        });
        let base = self.bits(attrs, index, Some(&len), visit.clone());
        self.field(attrs, field, ty);
        let signal = quote!(#private::Signal<#slot, #marker>);
        self.marker(attrs, &marker);
        self.impls.push(quote! {
            #(#attrs)*
            impl<#slot: #krate::Slot<Item = #item>> #signal for #ident {
                const BIT: usize = #private::slot::<#slot>(#base);
                const STALE: #flags = #private::union([]);
                const MOVED: #flags = #private::mask(<Self as #signal>::BIT);
                type Ref<'a> = #slot where Self: 'a;

                fn value(&self) -> #slot {
                    #private::read(&self.#name)
                }
            }

            #(#attrs)*
            impl #krate::Feed<#write, #marker> for #ident {
                fn feed(&mut self, changed: &mut #flags, value: #write) {
                    #private::write(&mut self.#name, changed, #base, value);
                }
            }
        });
        self.reports.push(quote! {
            #(#attrs)*
            #private::report(&self.#name, changed, #base, visitor);
        });
        let chain = &self.chain;
        self.take(attrs, &quote!(#local), &quote!(#private::unbank(#chain)));
        self.signals.push(Signal {
            attrs,
            field: Some(name),
        });
        self.fed.push(Fed {
            attrs,
            ty: write,
            marker,
        });
    }

    fn memo(&mut self, attrs: &'d [Attribute], memo: &'d Memo) {
        let Memo {
            field,
            function,
            ty,
            deps,
        } = memo;
        let deps = dependencies(deps);
        let name = &field.name;
        let local = local(name);
        let helper = format_ident!("__outcrop_memo_{}", self.signals.len());
        let (private, chain) = (self.private.clone(), self.chain.clone());
        let picks = self.picks(&deps, &quote!(&#chain));
        self.builds.push(quote! {
            #(#attrs)*
            let #local: #ty = #helper(#(#picks),*);
            #(#attrs)*
            let #chain = #private::input(#local, #chain);
        });
        let (bit, marker) = self.signal(attrs, ty, Some(name));
        self.field(attrs, field, ty);
        self.derivation(attrs, function, &helper, &deps, ty);
        let moved = deps
            .iter()
            .map(|dep| quote!(#private::moved::<Self, #dep, _>()));
        let stale = quote!(#private::union([#(#moved),*]));
        let moved = quote!(#private::union([#private::mask(#bit), #stale]));
        self.lookup(
            attrs,
            ty,
            (&bit, &marker),
            (stale, moved),
            (quote!(&'a #ty), quote!(&self.#name)),
        );
        let dirty = self.dirty(&deps);
        let arguments = self.arguments(&deps);
        let ne = quote_spanned!(ty.span()=> ::core::cmp::PartialEq::ne);
        self.evaluations.push(quote! {
            #(#attrs)*
            if #dirty {
                let value = #helper(#(#arguments),*);
                if #ne(&value, &self.#name) {
                    self.#name = value;
                    #private::set(changed, #bit);
                }
            }
        });
        self.unstore(attrs, &local);
    }

    fn view(&mut self, attrs: &'d [Attribute], view: &'d View) {
        let View { function, ty, deps } = view;
        let deps = dependencies(deps);
        let index = self.signals.len();
        let helper = format_ident!("__outcrop_view_{index}");
        let (private, chain) = (self.private.clone(), self.chain.clone());
        self.builds.push(quote! {
            #(#attrs)*
            let #chain = #private::view(#helper, #chain);
        });
        let (bit, marker) = self.signal(attrs, ty, None);
        self.derivation(attrs, function, &helper, &deps, ty);
        let stale = deps
            .iter()
            .map(|dep| quote!(#private::stale::<Self, #dep, _>()));
        let moved = deps
            .iter()
            .map(|dep| quote!(#private::moved::<Self, #dep, _>()));
        let arguments = self.arguments(&deps);
        self.lookup(
            attrs,
            ty,
            (&bit, &marker),
            (
                quote!(#private::union([#(#stale),*])),
                quote!(#private::union([#private::mask(#bit), #(#moved),*])),
            ),
            (quote!(#ty), quote!(#helper(#(#arguments),*))),
        );
        let dirty = self.dirty(&deps);
        self.evaluations.push(quote! {
            #(#attrs)*
            if #dirty {
                #private::set(changed, #bit);
            }
        });
        let chain = self.chain.clone();
        self.take(attrs, &quote!(_), &quote!(#chain));
    }

    fn router(&mut self, attrs: &'d [Attribute], position: usize, router: &'d Router) {
        let Router {
            function,
            ty: pointee,
        } = router;
        let ty = quote!(&#pointee);
        let view = format_ident!("__OutcropBelow{position}", span = Span::mixed_site());
        let marker = format_ident!("__OutcropRouter{position}", span = Span::mixed_site());
        let helper = format_ident!("__outcrop_route_{position}");
        let (krate, private) = (self.krate, self.private.clone());
        let sink = quote!(&mut #krate::Sink<'_, #view<'_>>);
        self.helper(
            attrs,
            function,
            &helper,
            &[quote!(#ty), sink.clone()],
            &TokenStream::new(),
            &quote!(::core::ops::FnMut(#ty, #sink)),
        );
        let (ident, flags) = (self.ident, &self.flags);
        let below = self.fed.iter().map(
            |Fed {
                 attrs: fed_attrs,
                 ty,
                 marker,
             }| {
                quote! {
                    #(#attrs)*
                    #(#fed_attrs)*
                    impl #krate::Feed<#ty, #marker> for #view<'_> {
                        fn feed(&mut self, changed: &mut #flags, value: #ty) {
                            <#ident as #krate::Feed<#ty, #marker>>::feed(self.0, changed, value);
                        }
                    }
                }
            },
        );
        self.impls.push(quote! {
            #(#attrs)*
            struct #view<'s>(&'s mut #ident);

            #(#attrs)*
            impl #krate::Strata for #view<'_> {
                type Flags = #flags;

                const CHANGED: #flags = <#ident as #krate::Strata>::CHANGED;

                fn evaluate(&mut self, _: &mut #flags) {}
            }

            #(#below)*

            #(#attrs)*
            impl #krate::Feed<#ty, #marker> for #ident {
                fn feed(&mut self, changed: &mut #flags, value: #ty) {
                    #helper(value, &mut #krate::Sink::new(&mut #view(self), changed));
                }
            }
        });
        self.marker(attrs, &marker);
        self.builds.push(quote! {
            #(#attrs)*
            #private::routed::<Self, #ty, _>();
        });
        self.fed.push(Fed { attrs, ty, marker });
    }
}

/// Implements `Get` for every signal the struct implements `Signal` for.
fn lookup_by_type(
    ident: &Ident,
    krate: &Path,
    private: &TokenStream,
    flags: &Ident,
) -> TokenStream {
    let (t, i) = (mixed("__OutcropT"), mixed("__OutcropI"));
    let signal = quote!(#private::Signal<#t, #i>);
    quote! {
        impl<#t, #i> #krate::Get<#t, #i> for #ident
        where
            Self: #signal,
        {
            type Ref<'a> = <Self as #signal>::Ref<'a> where Self: 'a;

            fn get(&self, changed: &#flags) -> #krate::Reading<Self::Ref<'_>> {
                #private::get(self, changed)
            }
        }
    }
}

fn dependencies(deps: &Type) -> Vec<&Type> {
    if let Type::Tuple(tuple) = deps
        && !tuple.elems.is_empty()
    {
        tuple.elems.iter().collect()
    } else if let Type::Paren(paren) = deps {
        Vec::from([&*paren.elem])
    } else {
        Vec::from([deps])
    }
}

/// A function calling `f` with its arguments, after checking `f` against
/// `bound` as `Formation::memo`, `view` and `route` do, which also types the
/// parameters of a closure.
fn function(
    name: &Ident,
    parameters: &[TokenStream],
    output: &TokenStream,
    f: &Expr,
    bound: &TokenStream,
) -> TokenStream {
    let arguments = (0..parameters.len())
        .map(|index| format_ident!("__outcrop_a{index}", span = Span::mixed_site()))
        .collect::<Vec<_>>();
    let (typed, generic, function) = (
        mixed("__outcrop_typed"),
        mixed("__OutcropF"),
        mixed("__outcrop_f"),
    );
    quote! {
        fn #name(#(#arguments: #parameters),*) #output {
            fn #typed<#generic: #bound>(#function: #generic) -> #generic {
                #function
            }
            #[allow(unused_mut)]
            let mut #function = #typed(#f);
            #function(#(#arguments),*)
        }
    }
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
    fn targets_field_attributes_at_the_field_only() -> syn::Result<()> {
        let output = expand(quote!(struct S {
            #[strata(field(serde(rename = "a")))] a: A = A(0),
            #[strata(field(serde(skip)))] b: B = A => f,
        }))?;
        assert!(
            output.contains("# [serde (rename = \"a\")] a : A"),
            "{output}"
        );
        assert!(output.contains("# [serde (skip)] b : B"), "{output}");
        assert_eq!(output.matches("# [serde").count(), 2, "{output}");
        Ok(())
    }

    #[test]
    fn stores_named_signals_and_views_the_rest() -> syn::Result<()> {
        let output = expand(quote!(struct S {
            a: A = A(0),
            b: outcrop::B = A => f,
            C = (A, outcrop::B) => g,
        }))?;
        assert!(output.contains("a : A ,"), "{output}");
        assert!(output.contains("b : outcrop :: B"), "{output}");
        assert!(!output.contains(" : C ,"), "{output}");
        Ok(())
    }

    #[test]
    fn rejects_unnamed_inputs() {
        let error = expand(quote!(struct S { A = A(0) }))
            .err()
            .map(|error| error.to_string());
        assert_eq!(
            error.as_deref(),
            Some("inputs are stored, so they need a field name: `name: Type = initial`")
        );
    }

    #[test]
    fn puts_fn_attributes_on_the_function_and_field_attributes_on_the_field() -> syn::Result<()> {
        let output = expand(quote!(struct S {
            a: A = A(0),
            #[strata(fn(inline), field(serde(skip)))] b: B = A => f,
            #[strata(fn(inline, cold))] C = A => f,
            #[strata(fn(inline))] &[u8] => f,
        }))?;
        assert!(output.contains("# [serde (skip)] b : B"), "{output}");
        assert!(
            output.contains("# [inline] fn __outcrop_memo_1"),
            "{output}"
        );
        assert!(
            output.contains("# [inline] # [cold] fn __outcrop_view_2"),
            "{output}"
        );
        assert!(
            output.contains("# [inline] fn __outcrop_route_3"),
            "{output}"
        );
        assert!(!output.contains("strata"), "{output}");
        Ok(())
    }

    #[test]
    fn copies_bare_attributes_onto_every_generated_item() -> syn::Result<()> {
        let output = expand(quote!(struct S {
            #[deny(x)] a: A = A(0),
            #[deny(y)] B = A => f,
            #[deny(z)] [u8] => g,
        }))?;
        for item in [
            "# [deny (x)] a : A",
            "# [deny (x)] let __outcrop_field_a",
            "# [deny (x)] a : __outcrop_field_a",
            "# [deny (x)] if :: outcrop :: __private :: is_set (changed",
            "# [deny (x)] impl :: outcrop :: __private :: Signal < A",
            "# [deny (x)] let (__outcrop_field_a , __outcrop_chain) = :: outcrop :: __private :: unstore",
            "# [deny (x)] impl :: outcrop :: Feed < A",
            "# [deny (y)] fn __outcrop_view_1",
            "# [deny (y)] if :: outcrop :: __private :: is_set (changed",
            "# [deny (y)] let __outcrop_chain = :: outcrop :: __private :: view (__outcrop_view_1",
            "# [deny (y)] let (_ , __outcrop_chain) = __outcrop_chain",
            "# [deny (y)] impl :: outcrop :: __private :: Signal < B",
            "# [deny (z)] fn __outcrop_route_2",
            "# [deny (z)] impl :: outcrop :: Feed < & [u8]",
            "# [deny (z)] :: outcrop :: __private :: routed :: < Self , & [u8] , _ >",
        ] {
            assert!(output.contains(item), "{item} in {output}");
        }
        Ok(())
    }

    #[test]
    fn rejects_field_attributes_on_routers() {
        let error = expand(quote!(struct S { #[strata(field(serde(skip)))] &[u8] => f }))
            .err()
            .map(|error| error.to_string());
        assert_eq!(
            error.as_deref(),
            Some("routers have no field, so `field(..)` does not apply to them")
        );
    }

    #[test]
    fn puts_fn_attributes_on_the_wrapper_of_a_function_path() -> syn::Result<()> {
        let output = expand(quote!(struct S { a: A = A(0), #[strata(fn(inline))] B = A => f }))?;
        assert!(
            output.contains(
                "# [inline] fn __outcrop_view_1 (__outcrop_a0 : & A) -> B { fn __outcrop_typed < __OutcropF : :: core :: ops :: Fn (& A) -> B > (__outcrop_f : __OutcropF) -> __OutcropF { __outcrop_f } # [allow (unused_mut)] let mut __outcrop_f = __outcrop_typed (f) ; __outcrop_f (__outcrop_a0) }"
            ),
            "{output}"
        );
        Ok(())
    }

    #[test]
    fn rejects_fn_attributes_on_inputs() {
        let error = expand(quote!(
            struct S { #[strata(fn(inline))] a: A = A(0) }
        ))
        .err()
        .map(|error| error.to_string());
        assert_eq!(
            error.as_deref(),
            Some("inputs have no function, so `fn(..)` does not apply to them")
        );
    }

    #[test]
    fn rejects_unknown_attribute_targets() {
        let error = expand(quote!(
            struct S { #[strata(impl(inline))] a: A = A(0) }
        ))
        .err()
        .map(|error| error.to_string());
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
            let error = expand(input).err().map(|error| error.to_string());
            assert_eq!(
                error.as_deref(),
                Some("only inputs can be banks: `#[strata(bank)] name: [T; N] = initial`")
            );
        }
    }

    #[test]
    fn reserves_a_bit_per_element_of_a_bank() -> syn::Result<()> {
        let output = expand(quote!(
            struct S {
            #[strata(bank)] a: [u16; 4] = [0; 4],
            #[cfg(x)] #[strata(bank)] b: [u8; 2] = [0; 2],
            c: C = C(0),
        }
        ))?;
        assert!(
            output.contains("const __OUTCROP_BIT1 : usize = __OUTCROP_BIT0 + 1 * (< [u16 ; 4] as :: outcrop :: __private :: Array > :: LEN)"),
            "{output}"
        );
        assert!(
            output.contains("const __OUTCROP_BIT2 : usize = __OUTCROP_BIT1 + __outcrop_compiled1 ! (present) as usize * (< [u8 ; 2] as :: outcrop :: __private :: Array > :: LEN)"),
            "{output}"
        );
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
    fn counts_bits_and_bounds_visitors_through_the_entry_attributes() -> syn::Result<()> {
        let output = expand(quote!(
            struct S { #[cfg(a)] #[cfg_attr(b, cfg(c))] a: A = A(0) }
        ))?;
        assert!(
            output.contains(
                "# [cfg (a)] # [cfg_attr (b , cfg (c))] # [allow (unused_macros)] macro_rules ! __outcrop_compiled0"
            ),
            "{output}"
        );
        assert!(
            output.contains("__OUTCROP_BIT0 + __outcrop_compiled0 ! (present) as usize"),
            "{output}"
        );
        assert!(output.contains("__outcrop_compiled0 ! (bound)"), "{output}");
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
            struct S { a: A = A(0), b: B = A => f, &[u8] => g }
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
    fn keeps_struct_attributes_on_the_struct() -> syn::Result<()> {
        let output = expand(quote!(
            #[cfg(feature = "x")]
            #[derive(Debug)]
            struct S { a: A = A(0) }
        ))?;
        assert_eq!(
            output.matches("# [cfg (feature = \"x\")]").count(),
            1,
            "{output}"
        );
        assert!(
            output.contains("# [cfg (feature = \"x\")] # [derive (Debug)] struct S"),
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
