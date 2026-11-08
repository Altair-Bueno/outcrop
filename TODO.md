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
- [x] **Pack `strata!` flags in a `Chunk`.** `strata!` declares its flags as
      `Chunk<N>`, the type builder banks use, so nesting a struct reuses the
      `Bits` impl of `(Tail, Chunk<N>)` and takes structs of any width, where
      the bare `[u8; N]` it replaced capped them at 256 signal bits. `Chunk`
      holds the mask, union and intersection `__private` delegates to. Code
      size, fuel and stack unchanged. `crates/outcrop/src/flags.rs`

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
- [x] **Report banks by byte.** Bank reports skip bytes with no change bit set
      and jump to each set bit with `trailing_zeros`, in both APIs. With one
      word of 100 changing per round, fuel 2915 -> 842 (-71%); with every word
      changing, unchanged (10228 -> 10222); peak stack +16 B.
      `crates/outcrop/src/bank.rs`

## API guidelines

Deviations from the [Rust API Guidelines][guidelines] found by the
`effective-rust` audit.

- [ ] **Package metadata (C-METADATA).** `license`, `repository` and `readme`
      are missing, and there is no README or changelog. Blocks publishing;
      `clippy::cargo_common_metadata` is allowed in the root `Cargo.toml` until
      a licence is chosen.
- [ ] **Derives `Formation` cannot use (C-COMMON-TRAITS).** `Formation` derives
      `PartialEq`, `Eq`, `Hash` and `Default`, but `Input`, `Bank`, `Memo` and
      `View` implement none of them, so no built formation has them. Memos and
      views hold closures and can never be `PartialEq`; `Input<T>` and `Bank`
      could derive all four for input-only formations.
- [ ] **Doc examples (C-EXAMPLE).** Only `formation.rs` has them. `Reading`,
      `Sink`, `Visit`, `Feed`, `Get` and `Report` have none, and neither do
      `Write` and `Element` beyond the `Formation::bank` docs.
- [ ] **Seal or document `Strata`, `Feed`, `Get` and `Report` (C-SEALED).**
      They are public and open, so a hand-written impl can break the
      change-bit invariants. Seal them, or state that implementing them is
      unsupported.
- [ ] **`Element` has public fields (C-STRUCT-PRIVATE).** `index` and `value`
      can never be joined by another field without a breaking change.
      `#[non_exhaustive]` would do, at the cost of callers constructing one.
- [ ] **`Write` name.** It collides with `std::io::Write` and
      `core::fmt::Write` when imported beside them.
- [ ] **`Reading` combinators.** No `map` or `as_ref`.

[guidelines]: https://rust-lang.github.io/api-guidelines/checklist.html

## Kept as is

- `nested::intersects` walks every bit of a nested signal's mask rather than
  only the set ones with `trailing_zeros`: the mask is a constant, so LLVM folds
  the walk away. Measured with the `nest_bank_*` scenarios, which match the
  builder's (842 and 10222 fuel), and again with a 1000-word nested bank, the
  `trailing_zeros` walk left fuel, code size and stack unchanged.
- The `Layer` impls of `Bank` and `Nested` repeat `LEN`, `Cursor = Used<8>`,
  `below` and `below_mut`: sharing them through a helper trait needs a second
  blanket `Layer` impl, which overlaps the one over `Value + Settle<Tail>`.
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
- `strata!` tests a memo's or view's dependencies one `is_set` each rather than
  one constant mask with `intersects`: measured, the mask saved 0.2% of fuel on
  the workload's 4-byte flags but cost 30% (831 -> 1079) with a 100-word bank,
  as `intersects` walks every byte of the flags and LLVM does not fold the
  mask's zero bytes away.
- Each `strata!` router keeps a `Feed` forwarding impl per entry fed before it:
  measured on 100 inputs, three more routers (300 more impls) add about 20 ms to
  `cargo check` (104 -> 124 ms) and 0.4% to a release build, with no runtime
  cost; a generic impl would trade the "cannot be fed" error for a
  post-monomorphisation one, or still need an impl per entry.
