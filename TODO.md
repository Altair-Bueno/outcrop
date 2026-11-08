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

- [ ] **A change bit means "written with a different value", not "differs from
      the last report".** _Verified._ Inputs and banks compare a feed with the
      current value, so `feed(A(1)); feed(A(0))` between polls reports `A(0)`
      although nothing changed, and so does a router feeding an input twice. The
      `poll` docs promise only changed values. Either compare with the
      last-reported value (a stored copy) or reword the docs.
      `crates/outcrop/src/stratum.rs`, `crates/outcrop/src/bank.rs`
- [ ] **`PartialEq` is assumed reflexive.** _Verified._ An input holding
      `f32::NAN` is reported on every feed, and a NaN memo on every
      recomputation. Require `Eq`, or document it.
- [ ] **`strata!` views take at most eight dependencies.** _Verified._ The
      `strata!` docs claim any number, but `Default` builds views with
      `__private::view`, bounded by `ComputeImpl`, which stops at eight. Memos
      have no limit. `crates/outcrop/src/__private.rs`
- [ ] **A panicking visitor loses reports.** `poll` takes the flags before
      `report`, so under `panic=unwind` the changes not yet visited are never
      reported, although the memos were updated. Clear bits as they are
      reported, or document that visitors must not panic.
      `crates/outcrop/src/formation.rs`

### Held by construction only

- [ ] **`Layer::LEN`, `Cursor` and `Flags` must agree.** A `Used<8>` cursor must
      mean `LEN == Flags::BITS`; `Bank` and `Nested` set them by hand and
      nothing ties them together. `crates/outcrop/src/plumbing.rs`
- [ ] **Bit indexes are never range-checked.** `(Tail, u8)::set_top` underflows
      below `Tail::BITS`, and `u8::get`/`set_top` with an index of 8 or more
      panic in debug and alias another bit in release.
      `crates/outcrop/src/flags.rs`
- [ ] **`strata!` evaluates in declaration order.** The generated `evaluate`
      compiles with a dependency declared below; only the chain built by
      `Default` rejects it. `crates/outcrop-macros/src/expand.rs`
- [ ] **`Flags::default()` means "nothing changed".** `poll` skips on it. Spare
      bits past the last signal are set by `CHANGED` and bank deposits, harmless
      only because poll clears them and bank reports mask them out.
- [ ] **Hand-written `Strata`, `Feed`, `Get`, `Report` and `__private::Signal`
      impls can break all of the above.** A wrong `Signal::BIT` or mask makes a
      `Nested` lookup read the wrong bit, and calling `evaluate`, `report` and
      `feed` directly skips the order `Formation::poll` keeps. See C-SEALED
      above. `crates/outcrop/src/nested.rs`

### Partial checks

- [ ] **Uniqueness depends on declaration order.** An input after a nested
      struct or bank holding its type is rejected at declaration, but a nested
      struct after such an input, a bank whose `Slot`s clash with earlier
      signals, or an input clashing with an earlier router's `&In` only fail
      with E0283 where used or fed. `nest` checks no feed collisions at all.
- [ ] **Compile-time asserts are post-monomorphisation.** _Verified._ The slot
      `INDEX < N` and `BYTES == N.div_ceil(8)` asserts pass `cargo check` and
      fail only `cargo build`, and never fire for unreachable code. In
      `strata!`, a slot index in a bank's spare bits computes a `BIT` aliasing
      the next signal; only the assert in `read` stops it.
- [ ] **A `strata!` field named `chain` fails to compile.** _Verified._ Its
      local `__outcrop_chain` clashes with the chain binding (E0416). The
      `__outcrop_memo_N`, `__outcrop_view_N` and `__outcrop_route_N` helpers
      also use call-site spans, unlike every other generated name.
      `crates/outcrop-macros/src/expand.rs`
- [ ] **`strata!` attribute forwarding.** Only plain `#[cfg]` on the struct
      reaches the generated `const _` block, not `#[cfg_attr(.., cfg(..))]`.

### Implicit state machines

Documented here so that changes keep them consistent.

1. **Change bit per signal:** clean → pending (declared, or fed a different
   value) → set by evaluation (memo changed, or a view's dependency moved) →
   reported and cleared by `poll`.
2. **Formation lifecycle**, seen through `Reading`: built (every bit set, memos
   stale) → fed (dependent memos stale) → polled (fresh). Freshness is derived
   from the flags, never stored.
3. **Memo cache:** valid, stale, recomputed equal (no bit) or recomputed changed
   (bit). Equal to `f(deps)` only after a poll; `From` and `nest` set every bit,
   repairing fields written directly in the declaring module.
4. **Bit allocator:** `Used<0>`…`Used<8>`, then a new byte. Banks and nested
   structs jump to `Used<8>`, leaving the current byte's spare bits unused.
5. **Builder vs closed:** `Layered` strata can be extended, `strata!` structs
   cannot; enforced by the trait bound.
6. **`poll` protocol:** skip if clean → evaluate → clear → report, fixed only
   inside `Formation::poll`.
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
