# TODO

Items found by the last `/simplify` review of the whole codebase and left out on
purpose.

## Design

- [ ] **Layers of any width.** The generic `(E, Tail)` impls of `Feed`,
      `Report`, `Select` and `Stack` only cover strata of one signal, so banks
      and routers carry their own `Strata`, `LayeredImpl`, `Select`, `Feed` and
      `Report` impls. A layer trait with a width (0 for routers, 1 for inputs,
      memos and views, N for banks) and a per-layer report hook would fold them
      into one set of impls, with an input being a bank of width 1. Needs care
      with coherence, since the `Settle` bound is what keeps the current impls
      apart. `crates/outcrop/src/stratum.rs`
- [ ] **One lookup chain for `strata!` and the builder.** `strata!` builds
      `Default` from its own `Stored`, `Lazy` and `Banked` chain, resolved by
      `Pick` and `Derive`, which mirror the builder's `Input`, `View`, `Bank`,
      `Select` and `Settle`. Building that chain from the builder's types would
      remove about 90 lines and one diagnostic, but needs a `Select` that does
      not require `LayeredImpl`. `crates/outcrop/src/__private.rs`
- [ ] **One `signal()` path for inputs and banks in `strata!`.** `bank()` still
      repeats part of `signal()` (report, `signals` push) because `signal()`
      takes a single concrete type. Taking a width and an optional slot generic
      would let inputs be width 1. `crates/outcrop-macros/src/expand.rs`

## Performance

- [ ] **Pack the builder's bank flags.** `Chunk<N>` stores a `bool` per element,
      a byte per bit, and a bank closes the current flag byte. Stable Rust
      cannot compute `N.div_ceil(8)` in a type; a second const parameter for the
      byte count, checked by a `const` assertion, would pack them.
      `crates/outcrop/src/flags.rs`
- [ ] **Report banks by byte.** Bank reports test every element's bit on every
      poll that reports anything; skipping zero bytes and walking set bits with
      `trailing_zeros` would test about a byte per eight elements.
      `crates/outcrop/src/bank.rs`
- [ ] **Test a memo's dependencies with one mask.** `strata!` checks each
      dependency with its own `is_set`; a `const` mask of all of them tested
      with one `intersects` would replace k checks with one.
      `crates/outcrop-macros/src/expand.rs`
- [ ] **Fewer router forwarding impls.** Each router in `strata!` emits a `Feed`
      impl for every entry fed before it, quadratic in code size; one generic
      impl gated by a sealed marker trait would do.
      `crates/outcrop-macros/src/expand.rs`

## Kept as is

- `Distinct` and `Routed` look like plain aliases of `Get` and `Feed`, but work
  around rustc#152409, where such a where-bound shadows `Strata::Flags`.
- The two copies of `read` in `stratum.rs` and `__private.rs`: sharing a free
  function from a private module trips `redundant_pub_crate` or
  `unreachable_pub`.
- `arguments` and `picks` in `expand.rs` stay separate: merging them needs a
  closure building tokens, which reads worse than two short named helpers.
- The loops in `tools/measure/workload` stay hand-written: rewriting them with
  `core::array::from_fn` could shift the baselines the workload measures.
- The Aidoo example keeps its `From<u16>` unit impls and `bit()` matches written
  out: no `macro_rules` in the example, and `as` casts trip `as_conversions`.
