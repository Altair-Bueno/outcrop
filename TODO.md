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
      `PartialEq`, `Eq` and `Hash`, but `Input`, `Bank`, `Memo` and `View`
      implement none of them, so only formations of `strata!` structs have them;
      `Default` is implemented for the empty formation alone. Memos and views
      hold closures and can never be `PartialEq`; `Input<T>` and `Bank` could
      derive all four for input-only formations.
- [ ] **Doc examples (C-EXAMPLE).** The crate root, `Formation`, `Slot` and
      `strata!` have them. `Reading`, `Sink`, `Visit`, `Feed`, `Get` and
      `Report` have none, and neither do `Write` and `Element` beyond the
      `Formation::bank` docs.
- [ ] **Seal or document `Strata`, `Feed`, `Get` and `Report` (C-SEALED).** They
      are public and open, so a hand-written impl can break the change-bit
      invariants. Seal them, or state that implementing them is unsupported.
- [ ] **`Element` has public fields (C-STRUCT-PRIVATE).** `index` and `value`
      can never be joined by another field without a breaking change.
      `#[non_exhaustive]` would do, at the cost of callers constructing one.
- [ ] **`Write` name.** It collides with `std::io::Write` and `core::fmt::Write`
      when imported beside them.
- [ ] **`Reading` combinators.** No `map` or `as_ref`.

[guidelines]: https://rust-lang.github.io/api-guidelines/checklist.html

## Invariants

Invariants the crates rely on without enforcing them, found by an audit of
`crates/`. Items marked _verified_ were reproduced with a scratch test.

### Broken promises

- [x] **A change bit means "written with a different value", not "differs from
      the last report".** Inputs and banks compare a feed with the current
      value, so `feed(A(1)); feed(A(0))` between polls reports `A(0)`. Kept: the
      crate docs, `feed`, `poll` and `Write` now say so, with a doctest on
      `feed`. `crates/outcrop/src/formation.rs`
- [x] **`PartialEq` was assumed reflexive.** An input holding `f32::NAN` was
      reported on every feed, and a NaN memo on every recomputation. Inputs,
      memos and banks now require `Eq` in both APIs.
      `crates/outcrop/src/formation.rs`
- [x] **`strata!` views took at most eight dependencies, memos any number.**
      `Default` checked views with `ComputeImpl`, as the builder does, but
      picked a memo's dependencies one by one. It now computes memos through
      `ComputeImpl` too, so memos and views take at most eight in both APIs,
      with the builder's diagnostic. `crates/outcrop/src/__private.rs`
- [x] **A panicking visitor lost reports.** `poll` took the flags before
      `report`, so under `panic=unwind` the changes not yet visited were never
      reported. `Report::report` now takes the flags by `&mut` and clears each
      bit just before visiting its signal, and `poll` resets the spare bits
      afterwards. The signal the visitor panicked on is not reported again, so a
      visitor that always panics on it cannot block the rest. Cost: code size
      builder 12108 -> 12498 B, macro 7486 -> 7650 B; fuel +1-3%, a fully
      changed 100-word bank +9% (10222 -> 11140); peak stack -16 B on banks.
      Storing the remaining bits once per element instead cut the full bank to
      10870 but raised a single changed word from 854 to 896, so it was dropped.
      `crates/outcrop/src/formation.rs`

### Held by construction only

- [x] **`Layer::LEN`, `Cursor` and `Flags` had to agree.** `Bank` and `Nested`
      set `LEN` by hand. Each cursor now states the bits it leaves spare in the
      top byte, and `Layer::LEN` and `LayeredImpl::LEN` default to
      `Flags::BITS - Cursor::SPARE`, so no layer writes `LEN` any more.
      `crates/outcrop/src/plumbing.rs`
- [x] **Bit indexes are never range-checked.** Accepted. `(Tail, u8)::set_top`
      and `clear_top` underflow below `Tail::BITS`, and `u8::get`, `set_top` and
      `clear_top` with an index of 8 or more panic in debug and alias another
      bit in release. But every caller passes its layer's `LEN`, now derived
      from the flags, or a `Chunk` index, which the array bounds-checks, and
      `Bits` is private. A release-mode check would cost code size and fuel on
      every bit operation. `crates/outcrop/src/flags.rs`
- [ ] **Hand-written `Strata`, `Feed`, `Get`, `Report` and `__private::Signal`
      impls can break all of the above.** A wrong `Signal::BIT` or mask makes a
      `Nested` lookup read the wrong bit, calling `evaluate`, `report` and
      `feed` directly skips the order `Formation::poll` keeps, and a `Report`
      that does not clear the bits it visits reports them again after a
      panicking visitor. See C-SEALED above. `crates/outcrop/src/nested.rs`

### Partial checks

- [ ] **Uniqueness depends on declaration order.** An input after a nested
      struct or bank holding its type is rejected at declaration, but a nested
      struct after such an input, a bank whose `Slot`s clash with earlier
      signals, or an input clashing with an earlier router's `&In` only fail
      with E0283 where used or fed. `nest` checks no feed collisions at all.
- [x] **Compile-time asserts are post-monomorphisation.** Accepted and
      documented: the slot `INDEX < N` and `BYTES == N.div_ceil(8)` asserts pass
      `cargo check` and fail only `cargo build`, and never fire for unreachable
      code. Checking them before needs `generic_const_exprs`. A slot index of
      `N` or more aliased the next signal's bit, caught only by the assert in
      `read`; the assert now lives in `slot`, the `const fn` computing the
      slot's bit in both APIs, so every use of the bit trips it. `compile_fail`
      doctests cover both APIs.
- [x] **A `strata!` field named `chain` failed to compile.** Its local
      `__outcrop_chain` clashed with the chain binding (E0416). Field locals are
      now `__outcrop_field_*`, which no other generated name starts with. The
      helper functions keep their call-site spans: a mixed-site span only makes
      locals hygienic, not items. `crates/outcrop-macros/src/expand.rs`
- [x] **`strata!` attribute forwarding.** Only plain `#[cfg]` on the struct
      reached the generated impls, so `#[cfg_attr(.., cfg(..))]` removed the
      struct and left its impls behind. The struct's attributes now stay on the
      struct alone, and `cfg` or `cfg_attr` on the macro call gates the whole
      expansion. `crates/outcrop-macros/src/expand.rs`

### Implicit state machines

Documented here so that changes keep them consistent.

1. **Change bit per signal:** clean → pending (declared, or fed a different
   value) → set by evaluation (memo changed, or a view's dependency moved) →
   reported and cleared by `poll`.
2. **Formation lifecycle**, seen through `Reading`: built (every bit set, memos
   stale) → fed (dependent memos stale) → polled (fresh). Freshness is derived
   from the flags, never stored. The flags take part in `Formation`'s
   `PartialEq` and `Hash`, so formations are compared after polling them, as
   documented on `Formation` and `strata!`.
3. **Memo cache:** valid, stale, recomputed equal (no bit) or recomputed changed
   (bit). Equal to `f(deps)` only after a poll; `From` and `nest` set every bit,
   repairing fields written directly in the declaring module.
4. **Bit allocator:** `Used<0>`…`Used<8>`, then a new byte. Banks and nested
   structs jump to `Used<8>`, leaving the current byte's spare bits unused.
5. **Builder vs closed:** `Layered` strata can be extended, `strata!` structs
   cannot; enforced by the trait bound.
6. **`poll` protocol:** skip if clean → evaluate → report, clearing each bit
   just before its signal is visited → reset the spare bits. Fixed only inside
   `Formation::poll`.
7. **Router state:** builder routers keep state in their `FnMut`; `strata!`
   routers cannot capture, so their state lives untracked in statics.
8. **`strata!` `Default` chain:** pushed in declaration order, popped in
   reverse, ending in `let () = chain`; enforced at compile time.
9. **`strata!` cfg detection:** two shadowing `macro_rules!` give each entry a
   compiled-in or compiled-out state, feeding bit numbering and visitor bounds.

## Kept as is

- `nested::intersects` walks every bit of a nested signal's mask rather than
  only the set ones with `trailing_zeros`: the mask is a constant, so LLVM folds
  the walk away. Measured with the `nest_bank_*` scenarios, which match the
  builder's (842 and 10222 fuel), and again with a 1000-word nested bank, the
  `trailing_zeros` walk left fuel, code size and stack unchanged.
- Builder banks and nested structs take whole bytes of change bits: the bits
  left in the byte beneath them and in their last byte go unused, and the next
  signal starts a new byte. `input(A).bank([_; 4]).input(B)` takes three bytes
  where one would do (formation 9 B instead of 7), and a bank deposited first
  leaves the empty formation's byte unused (6 B instead of 5). Advancing the
  `Used<_>` cursor by N bits needs const generic arithmetic, unstable in Rust;
  at most two bytes are lost per bank or nested struct. `strata!` packs its
  banks with its other signals. Documented on `Formation::bank` and `nest`.
- The `Layer` impls of `Bank` and `Nested` repeat `Cursor = Used<8>`, `below`
  and `below_mut`: sharing them through a helper trait needs a second blanket
  `Layer` impl, which overlaps the one over `Value + Settle<Tail>`.
- `Distinct` and `Routed` look like plain aliases of `Get` and `Feed`, but work
  around rustc#152409, where such a where-bound shadows `Strata::Flags`.
- The two copies of `slot` and `read` in `stratum.rs` and `__private.rs`:
  sharing a free function from a private module trips `redundant_pub_crate` or
  `unreachable_pub`.
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
