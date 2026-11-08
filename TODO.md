# TODO

Items found by the last `/simplify` review of the whole codebase and left out on
purpose.

## Design

- [x] **Layers of any width.** A `Layer<Tail>` trait, implemented by single
      signals (through `Settle`), routers and banks, gives one blanket impl each
      of `Strata`, `LayeredImpl`, `Report` and forwarding `Feed`. `Value` stays
      a separate, non-generic trait: a generic `Settle<Tail>` bound would let
      another crate implement it for `Router`, so the blanket impls would
      overlap. `crates/outcrop/src/stratum.rs`
- [x] **One lookup chain for `strata!` and the builder.** `strata!`'s `Default`
      builds its chain from the builder's `Input`, `View` and `Bank` layers and
      resolves dependencies with `Select`, which now carries the "not a signal
      declared above" diagnostic. `Pick`, `Stored`, `Lazy`, `Derive` and
      `Banked` are gone. A view whose dependency is missing now also fails the
      duplicate checks of the signals above it, one more follow-on error than
      before. `crates/outcrop/src/__private.rs`
- [x] **Nest a `strata!` struct in a builder formation.**
      `Formation::nest(S::default())` deposits a `Nested<S>` layer, behind the
      `macros` feature: the struct's flags go on top of the builder's, its
      evaluate, report and feeds run on that slice, and `Select` maps each of
      its signals to `base + BIT` through the masks `strata!` generates. Limits
      as planned: no dependencies on the builder beneath it, and a type declared
      twice, by nesting the same struct twice or otherwise, is ambiguous when
      depended on rather than rejected at `nest`. `crates/outcrop/src/nested.rs`

## Performance

- [x] **Pack the builder's bank flags.** `Chunk<BYTES>` packs a change bit per
      element; the builder takes the byte count explicitly, as
      `bank::<13, _, _, _>([0_u16; 101])`, checked against `N.div_ceil(8)` at
      compile time. `crates/outcrop/src/flags.rs`
- [x] **Recover the builder's codegen after the `Layer` refactor.** LLVM's
      argument promotion turned the `&Tail` that the single-signal
      `LayerReport::report` reads one field through into a value loaded ahead of
      its `changed.get` branch, which the backend then masked.
      `#[inline(always)]` on that one method brings builder code back to 12110 B
      (12094 B before the refactor, 12396 B after) and peak stack to its old
      values. A blanket `#[inline]` on the layer methods made it worse.
      `crates/outcrop/src/stratum.rs`
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
- `signal()` and `bank()` in `expand.rs` stay separate: since `bits()` and
  `marker()` hold what they share, the rest differs (N bits, one generic impl
  for every slot, reporting `Element`s, no per-signal duplicate check), and
  merging them would need several bank-only parameters.
