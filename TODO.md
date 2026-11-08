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
- [ ] **One `signal()` path for inputs and banks in `strata!`.** `bank()` still
      repeats part of `signal()` (report, `signals` push) because `signal()`
      takes a single concrete type. Taking a width and an optional slot generic
      would let inputs be width 1. `crates/outcrop-macros/src/expand.rs`

- [ ] **Nest a `strata!` struct in a builder formation.** Deposit a `Nested<S>`
      layer, as in `Formation::new().input(A(0)).nest(S::default())`. Its
      `[u8; K]` flags go on top of the builder's, its `evaluate` and `report`
      run on that slice, a `Select` impl maps each signal's bit to `base + BIT`
      so builder signals above can depend on it, and a forwarding `Feed` impl
      reaches its inputs and banks. A wrapper is needed because a second blanket
      `Layer` impl would overlap the one over `Value + Settle`. It gives named,
      reusable parts in a larger formation, and since a nested struct is one
      layer for the builder's lookups, a way past the recursion limit. Limits:
      the struct cannot depend on builder signals below it, the same struct
      cannot be nested twice since signals are found by type, and checking a
      dependency inside it tests a mask against an offset slice of the flags.
      `crates/outcrop/src/stratum.rs`

## Performance

- [x] **Pack the builder's bank flags.** `Chunk<BYTES>` packs a change bit per
      element; the builder takes the byte count explicitly, as
      `bank::<13, _, _, _>([0_u16; 101])`, checked against `N.div_ceil(8)` at
      compile time. `crates/outcrop/src/flags.rs`
- [ ] **Recover the builder's codegen after the `Layer` refactor.**
      `tools/measure`, before against after: builder code +302 B (12094 to
      12396, +2.5%), wasm fuel +69 to +77 per round in every builder scenario
      (+1 to 2%), peak stack +16 to +32 B; `strata!` unchanged. The source does
      the same work: `builder_idle`, where everything is inlined, has exactly 69
      more instructions, 31 of them `i32.const 65535` + `i32.and` pairs, LLVM
      keeping the 16-bit flags word in a local and masking it where it used to
      reload it, plus 5 more locals. `#[inline]` on the layer methods made it
      worse (+570 B, same fuel). Next: compare the LLVM IR of `poll` and the
      feeds before and after, and try evaluating through `Layer::below_mut` in a
      different order or without the `&Tail` argument.
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
