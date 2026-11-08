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
use crate::Field;
use crate::Function;
use crate::Input;
use crate::Kind;
use crate::Memo;
use crate::Router;
use crate::View;

struct Signal<'d> {
    attrs: &'d [Attribute],
    key: String,
    storage: Storage<'d>,
    bit: Ident,
    stale: Vec<usize>,
    moved: Vec<usize>,
}

enum Storage<'d> {
    Field(&'d Ident),
    Computed { helper: Ident, deps: Vec<usize> },
}

fn local(field: &Ident) -> Ident {
    format_ident!("__outcrop_{field}", span = Span::mixed_site())
}

impl Signal<'_> {
    fn is_changed(&self) -> TokenStream {
        let bit = &self.bit;
        quote!(changed[#bit / 8] & 1 << (#bit % 8) != 0)
    }

    fn mark(&self) -> TokenStream {
        let bit = &self.bit;
        quote!(changed[#bit / 8] |= 1 << (#bit % 8);)
    }
}

struct Fed<'d> {
    attrs: &'d [Attribute],
    key: String,
    ty: &'d Type,
}

struct Expansion<'d> {
    ident: &'d Ident,
    krate: &'d Path,
    flags: TokenStream,
    next: TokenStream,
    bits: Vec<TokenStream>,
    signals: Vec<Signal<'d>>,
    fed: Vec<Fed<'d>>,
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
            next: quote!(0usize),
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
        expansion.entries(&self.entries);
        if let Some(errors) = expansion.errors {
            return errors.to_compile_error();
        }
        let Self { attrs, vis, .. } = self;
        let cfgs = attrs.iter().filter(|attr| attr.path().is_ident("cfg"));
        let count = Ident::new("__OUTCROP_BITS", Span::mixed_site());
        let Expansion {
            ident,
            krate,
            next,
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
        let names = signals.iter().filter_map(|signal| {
            let Storage::Field(field) = &signal.storage else {
                return None;
            };
            let (attrs, local) = (&signal.attrs, local(field));
            Some(quote!(#(#attrs)* #field: #local))
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
            const #count: usize = #next;

            type #flags = [::core::primitive::u8; #count.div_ceil(8)];

            #(#helpers)*

            impl ::core::default::Default for #ident {
                fn default() -> Self {
                    #(#defaults)*
                    Self { #(#names,)* }
                }
            }

            impl #krate::Strata for #ident {
                type Flags = #flags;

                const CHANGED: #flags = [::core::primitive::u8::MAX; #count.div_ceil(8)];

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

impl<'d> Expansion<'d> {
    fn entries(&mut self, entries: &'d [Entry]) {
        for (position, entry) in entries.iter().enumerate() {
            let attrs = &entry.attrs;
            match &entry.kind {
                Kind::Input(input) => self.input(attrs, input),
                Kind::Memo(memo) => self.memo(attrs, memo),
                Kind::View(view) => self.view(attrs, view),
                Kind::Router(router) => self.router(attrs, position, router),
            }
        }
    }

    fn error(&mut self, error: Error) {
        match &mut self.errors {
            Some(errors) => errors.combine(error),
            None => self.errors = Some(error),
        }
    }

    fn signal(&mut self, attrs: &'d [Attribute], ty: &Type, storage: Storage<'d>) -> Option<usize> {
        let key = key(ty);
        if self.signals.iter().any(|signal| signal.key == key) {
            self.error(Error::new(
                ty.span(),
                format_args!("`{key}` is already a signal of this formation"),
            ));
            return None;
        }
        let index = self.signals.len();
        let bit = format_ident!("__OUTCROP_BIT{index}", span = Span::mixed_site());
        let krate = self.krate;
        let visit = quote!(#krate::Visit<#ty>);
        let next = &self.next;
        self.bits.push(quote!(const #bit: usize = #next;));
        if attrs.is_empty() {
            self.next = quote!(#bit + 1);
            self.visits.push(visit.clone());
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
            self.next = quote!(#bit + #compiled!(present) as usize);
            self.visits.push(bound.into_token_stream());
        }
        self.signals.push(Signal {
            attrs,
            key,
            storage,
            bit,
            stale: Vec::new(),
            moved: Vec::from([index]),
        });
        let value = self.value(index, false);
        let is_changed = self.signals[index].is_changed();
        self.reports.push(quote! {
            #(#attrs)*
            if #is_changed {
                <__OutcropVisitor as #visit>::visit(visitor, &#value);
            }
        });
        Some(index)
    }

    fn value(&self, index: usize, local: bool) -> TokenStream {
        match &self.signals[index].storage {
            Storage::Computed { helper, deps } => {
                let arguments = deps.iter().map(|dep| self.value(*dep, local));
                quote!(#helper(#(&#arguments),*))
            }
            Storage::Field(field) if local => self::local(field).into_token_stream(),
            Storage::Field(field) => quote!(self.#field),
        }
    }

    fn dependencies(&mut self, deps: &'d Type) -> (Vec<&'d Type>, Vec<usize>) {
        let deps = dependencies(deps);
        let mut indices = Vec::new();
        for &dep in &deps {
            let key = key(dep);
            if let Some(index) = self.signals.iter().position(|signal| signal.key == key) {
                indices.push(index);
            } else {
                self.error(Error::new(
                    dep.span(),
                    format_args!("`{key}` is not a signal declared above"),
                ));
            }
        }
        (deps, indices)
    }

    fn helper(
        &mut self,
        attrs: &[Attribute],
        function: &Function,
        name: &Ident,
        parameters: &[TokenStream],
        output: &TokenStream,
    ) {
        let Function { attrs: targeted, f } = function;
        match self::function(name, parameters, output, f) {
            Ok(function) => self
                .helpers
                .push(quote!(#(#attrs)* #(#[#targeted])* #function)),
            Err(error) => self.error(error),
        }
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
        self.helper(attrs, function, name, &parameters, &quote!(-> #ty));
    }

    fn field(&mut self, attrs: &[Attribute], field: &Field, ty: &Type) {
        let Field {
            name,
            attrs: targeted,
        } = field;
        self.fields
            .push(quote!(#(#attrs)* #(#[#targeted])* #name: #ty));
    }

    fn link(&mut self, index: usize, deps: &[usize]) {
        let mut stale = Vec::new();
        let mut moved = Vec::from([index]);
        let memo = matches!(self.signals[index].storage, Storage::Field(_));
        for &dep in deps {
            let signal = &self.signals[dep];
            if memo {
                stale.extend_from_slice(&signal.moved);
            } else {
                stale.extend_from_slice(&signal.stale);
                moved.extend_from_slice(&signal.moved);
            }
        }
        if memo {
            moved.extend_from_slice(&stale);
        }
        for list in [&mut stale, &mut moved] {
            list.sort_unstable();
            list.dedup();
        }
        let signal = &mut self.signals[index];
        signal.stale = stale;
        signal.moved = moved;
    }

    fn fed(&mut self, attrs: &'d [Attribute], ty: &'d Type) -> Option<Fed<'d>> {
        let key = key(ty);
        if self.fed.iter().any(|fed| fed.key == key) {
            self.error(Error::new(
                ty.span(),
                format_args!("`{key}` is already fed to this formation"),
            ));
            return None;
        }
        Some(Fed { attrs, key, ty })
    }

    fn input(&mut self, attrs: &'d [Attribute], input: &'d Input) {
        let Input { field, ty, init } = input;
        let storage = Storage::Field(&field.name);
        let (Some(index), Some(fed)) = (self.signal(attrs, ty, storage), self.fed(attrs, ty))
        else {
            return;
        };
        self.field(attrs, field, ty);
        let (ident, krate, flags) = (self.ident, self.krate, &self.flags);
        let (name, local, mark) = (&field.name, local(&field.name), self.signals[index].mark());
        let differs = quote_spanned!(ty.span()=> ::core::cmp::PartialEq::ne(&self.#name, &value));
        self.impls.push(quote! {
            #(#attrs)*
            impl #krate::Feed<#ty, ()> for #ident {
                fn feed(&mut self, changed: &mut #flags, value: #ty) {
                    if #differs {
                        self.#name = value;
                        #mark
                    }
                }
            }
        });
        self.defaults
            .push(quote!(#(#attrs)* let #local: #ty = #init;));
        self.fed.push(fed);
        self.get(index, ty);
    }

    fn memo(&mut self, attrs: &'d [Attribute], memo: &'d Memo) {
        let Memo {
            field,
            function,
            ty,
            deps,
        } = memo;
        let (types, deps) = self.dependencies(deps);
        let Some(index) = self.signal(attrs, ty, Storage::Field(&field.name)) else {
            return;
        };
        self.field(attrs, field, ty);
        let helper = format_ident!("__outcrop_memo_{index}");
        self.derivation(attrs, function, &helper, &types, ty);
        self.link(index, &deps);
        let (name, local) = (&field.name, local(&field.name));
        let values = deps.iter().map(|dep| self.value(*dep, true));
        let default = quote!(#(#attrs)* let #local: #ty = #helper(#(&#values),*););
        self.defaults.push(default);
        let dirty = self.changed(&deps);
        let arguments = deps.iter().map(|dep| self.value(*dep, false));
        let mark = self.signals[index].mark();
        let differs = quote_spanned!(ty.span()=> ::core::cmp::PartialEq::ne(&value, &self.#name));
        let evaluation = quote! {
            #(#attrs)*
            if #dirty {
                let value = #helper(#(&#arguments),*);
                if #differs {
                    self.#name = value;
                    #mark
                }
            }
        };
        self.evaluations.push(evaluation);
        self.get(index, ty);
    }

    fn changed(&self, indices: &[usize]) -> TokenStream {
        let checks = indices
            .iter()
            .map(|index| self.signals[*index].is_changed());
        quote!(#(#checks)||*)
    }

    fn view(&mut self, attrs: &'d [Attribute], view: &'d View) {
        let View { function, ty, deps } = view;
        let (types, deps) = self.dependencies(deps);
        let helper = format_ident!("__outcrop_view_{}", self.signals.len());
        self.derivation(attrs, function, &helper, &types, ty);
        let storage = Storage::Computed {
            helper,
            deps: deps.clone(),
        };
        let Some(index) = self.signal(attrs, ty, storage) else {
            return;
        };
        self.link(index, &deps);
        let dirty = self.changed(&deps);
        let mark = self.signals[index].mark();
        let evaluation = quote! {
            #(#attrs)*
            if #dirty {
                #mark
            }
        };
        self.evaluations.push(evaluation);
        self.get(index, ty);
    }

    fn get(&mut self, index: usize, ty: &Type) {
        let (ident, krate, flags) = (self.ident, self.krate, &self.flags);
        let Signal {
            attrs,
            stale,
            storage,
            ..
        } = &self.signals[index];
        let reading = quote!(#krate::Reading);
        let (output, value) = if let Storage::Computed { .. } = storage {
            (quote!(#ty), self.value(index, false))
        } else {
            let value = self.value(index, false);
            (quote!(&'a #ty), quote!(&#value))
        };
        let (changed, body) = if stale.is_empty() {
            (quote!(_), quote!(#reading::Fresh(#value)))
        } else {
            let stale = self.changed(stale);
            let body = quote! {
                if #stale {
                    #reading::Stale(#value)
                } else {
                    #reading::Fresh(#value)
                }
            };
            (quote!(changed), body)
        };
        let implementation = quote! {
            #(#attrs)*
            impl #krate::Get<#ty, ()> for #ident {
                type Ref<'a> = #output where Self: 'a;

                fn get(&self, #changed: &#flags) -> #reading<Self::Ref<'_>> {
                    #body
                }
            }
        };
        self.impls.push(implementation);
    }

    fn router(&mut self, attrs: &'d [Attribute], position: usize, router: &'d Router) {
        let Router { function, ty } = router;
        let Some(fed) = self.fed(attrs, ty) else {
            return;
        };
        let view = format_ident!("__OutcropBelow{position}");
        let helper = format_ident!("__outcrop_route_{position}");
        let krate = self.krate;
        let sink = quote!(&mut #krate::Sink<'_, #view<'_>>);
        self.helper(
            attrs,
            function,
            &helper,
            &[quote!(#ty), sink],
            &TokenStream::new(),
        );
        let (ident, flags) = (self.ident, &self.flags);
        let below = self.fed.iter().map(
            |Fed {
                 attrs: fed_attrs,
                 ty,
                 ..
             }| {
                quote! {
                    #(#attrs)*
                    #(#fed_attrs)*
                    impl #krate::Feed<#ty, ()> for #view<'_> {
                        fn feed(&mut self, changed: &mut #flags, value: #ty) {
                            <#ident as #krate::Feed<#ty, ()>>::feed(self.0, changed, value);
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
            impl #krate::Feed<#ty, ()> for #ident {
                fn feed(&mut self, changed: &mut #flags, value: #ty) {
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
        let output = expand(quote!(struct S { a: A = A(0), b: A = A(1), A = A => f }))?;
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
        let output = expand(quote!(struct S { B = A => f, a: A = A(0) }))?;
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
        assert!(output.contains("a : A"), "{output}");
        assert!(output.contains("b : outcrop :: B"), "{output}");
        assert!(!output.contains(": C"), "{output}");
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
            #[deny(z)] &[u8] => g,
        }))?;
        for item in [
            "# [deny (x)] a : A",
            "# [deny (x)] let __outcrop_a",
            "# [deny (x)] a : __outcrop_a",
            "# [deny (x)] if changed",
            "# [deny (x)] impl :: outcrop :: Feed < A",
            "# [deny (x)] impl :: outcrop :: Get < A",
            "# [deny (y)] fn __outcrop_view_1",
            "# [deny (y)] if changed",
            "# [deny (y)] impl :: outcrop :: Get < B",
            "# [deny (z)] fn __outcrop_route_2",
            "# [deny (z)] impl :: outcrop :: Feed < & [u8]",
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
                "# [inline] fn __outcrop_view_1 (__outcrop_a0 : & A) -> B { (f) (__outcrop_a0) }"
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
        assert_eq!(error.as_deref(), Some("expected `fn(..)` or `field(..)`"));
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
    fn rejects_closures_of_the_wrong_arity() -> syn::Result<()> {
        let output =
            expand(quote!(struct S { a: A = A(0), B = A => |a, b| B(a.0), &[u8] => |frame| {} }))?;
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
    fn forwards_cfg_attributes_to_the_generated_items() -> syn::Result<()> {
        let output = expand(quote!(
            #[cfg(feature = "x")]
            #[derive(Debug)]
            struct S { a: A = A(0) }
        ))?;
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
