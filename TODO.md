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

## Redesigns

Left out of the 2026-11-11 `/simplify` pass as too large for a cleanup, then
reviewed against the goals: compile-time guarantees, build speed, codegen,
runtime cost and stack, no allocation; API breaks allowed. Done in this order,
each measured against a snapshot of the tree before it with
`tools/measure/measure.sh`, plus `cargo check` time for 3–6.

- [x] **1. Clear change bits before visiting only under `panic = "unwind"`.**
      The per-signal clears exist so a caught visitor panic loses no reports
      (see _Broken promises_); under `panic = "abort"` nothing can catch it, and
      `poll` zeroes the flags after `report` anyway. The single-signal
      `LayerReport::report`, `Element::report` and `__private::clear` test
      `cfg!(panic = "unwind")`. Code size builder 12498 -> 12156 B, macro 7650
      -> 7478 B; fuel -1-3% on feeds and routes, full bank 11140 -> 10212 (-8%),
      one changed word 854 -> 832; reports unchanged. Peak stack +16 B on the
      builder's feed and route scenarios (343 -> 359), the rest unchanged.
      Dropping `#[inline(always)]` on the single-signal `report` made it worse
      (builder 12440 B, 375 B of stack). `cargo test` runs with unwind, so the
      harness's report counts are the check on the abort path.
- [x] **2. Unnameable builder indices.** The builder's lookups took `()` and
      `(I,)` as indices, so `.input(0_u8).input::<u8, (), ()>(1)` compiled,
      passing `Distinct` and `Routed` through the top layer alone. They are now
      `Here` and `There<I>`, declared in the private `stratum` module; `Slotted`
      and `Within` already were. A `compile_fail` doctest on `input` keeps it.
      Type-level only: no codegen change.
- [x] **3. Coarse `Reading::Stale`** (replaces the redesign of builder memo
      staleness below). A signal is stale when it is or depends on a memo, one
      associated `const CACHED: bool` on `Settle`, `Select`, `ComputeImpl` and
      `__private::Signal`, and the flags are not all clear. Removed: the
      recursive `stale`/`moved` walk, `ComputeImpl::pending` and `moved`,
      `Signal::STALE` and `MOVED`, `nested::intersects`, `Bits::byte`,
      `__private::mask` and `union`, and `Chunk::bit`, `union`, `bytes` and
      `intersects`. Semantic change, approved: a memo no pending feed reaches
      now reads `Stale`. The workload gained `builder_get` and `macro_get` (feed
      the bottom register, read `Summary` and `Alarm`, poll); the two reads
      cost, as `*_get` minus `*_feed_bottom`, builder 50 -> 22 fuel, macro 44
      -> 8. Code size, peak stack and formation size unchanged.
      `cargo check --workspace --all-targets --all-features` after cleaning both
      crates, best of three: 1962 -> 1650 ms, items 1 and 2 included. Superseded
      by _Drop `Reading`_ below: `Reading` and `CACHED` are gone.
- [x] **4. One uniqueness check per layer.** "Every type a formation holds is a
      single signal and fed in a single place" was never stated once: each
      builder method picked its own `Distinct`/`Routed` bounds, and `strata!`
      wrote it three times (a `distinct` per signal, a `routed` per input, bank
      and router, and the `checks` list behind `Nestable`). 490c2b6 fixed an
      `input` that checked `Distinct` but not `Routed`. Now each layer states
      what it holds in one `Sound<I>` impl beside its other impls (`Input<T>`:
      `Distinct<T>` + `Routed<T>`, `Memo`/`View`: `Distinct<Output>`, `Bank`:
      `Routed<Write<T>>`, `Router`: `Routed<&In>`, `Nested<N>`: `N: Nestable`),
      and `deposit` requires it; `input::<T, _, _>` became `input::<T, _>`.
      `strata!` keeps only the `checks` list, rendered as the `Nestable`
      predicates and checked in `Default` by one
      `__private::nestable::<Self, (), _>()` call (see _Review of 4465815_). It
      also rejects a signal type spelled twice, for entries without attributes
      since the macro cannot evaluate `cfg`, with "a signal of this type is
      already declared above" on the second; the trait check stays behind it for
      aliases, and its errors point at the macro call. Code size, fuel, stack
      and formation size unchanged; `cargo check` 1494 -> 1462 ms, within noise.
      `crates/outcrop/src/formation.rs`, `crates/outcrop-macros/src/expand.rs`
- [ ] ~~**5. Bank byte count from a table of impls.**~~ Dropped. The idea was a
      `macro_rules!` table of `impl Pack for [T; N]` giving
      `Chunk<{N.div_ceil(8)}>` up to a cap, so `bank([0_u16; 101])` needed no
      `bank::<13, _, _, _>` and the post-monomorphisation `BYTES` assert went.
      Measured, `cargo check -p outcrop` rebuilding only `outcrop`: 64 impls
      0.36 s / 126 MB, 256 impls 0.82 s / 173 MB, 1024 impls 9.9 s / 849 MB. The
      coherence check across the impls is quadratic, and a 4096-impl table (caps
      of 4095 elements) ran out of memory on a 31 GB machine. A cap of 255
      elements costs every user's build half a second and still refuses larger
      banks, so the explicit byte count and its assert stay. Run such
      experiments under
      `systemd-run --user --scope -p MemoryMax=4G -p MemorySwapMax=0`.
- [x] **6. `strata!` `Default` on borrowed layers.** The chain `Default` builds
      for memo initial values and view lookups moved every value and the whole
      chain into a new tuple per entry, and `unstore`/`unbank` moved them back
      out. Its layers now borrow the locals that keep their values, as
      `Input<T, &'a T>` and `Bank<T, N, BYTES, &'a [T; N]>` (see _Review of
      4465815_), so `unstore`, `unbank`, the `takes` list and `let () = chain`
      are gone and `Self { .. }` uses the locals directly. The frame of the
      generated `Default` in a debug build, for a struct with a `[u16; 1000]`
      bank, a view and a memo (read from `--emit asm` of a probe crate): 28064 B
      -> 2064 B. Release: code size, fuel, reports, peak stack and formation
      size unchanged; `cargo check` 1636 -> 1629 ms, noise.
- [x] **7. Drop `Reading`.** `get` returned `Reading<Ref>`, `Stale` when the
      signal was or depended on a memo and any change was pending. The caller
      makes every feed and poll, so it already knows whether a memo may be
      behind, and the wrapper only cost an `.into_inner()` per read.
      `Formation::get` and `Get::get` now return the `Ref` itself: `&T` for
      inputs and memos, `T` for views and bank slots. Inputs, slots and views
      over inputs read their current value; memos, and views over them, the
      value of the last poll or of their declaration. Removed: `Reading`,
      `Get::get`'s `changed` argument, `CACHED` on `Settle`, `Select`,
      `ComputeImpl` and `__private::Signal`, and `__private::cached` and `get`.
      The workload's `*_get` reads the two values through `black_box` instead of
      counting stale reads, so its reports drop from 151 to 119. The two reads
      cost, as `*_get` minus `*_feed_bottom`, builder 22 -> 8 fuel, macro 8
      -> 6. Code size, the other fuel figures, peak stack and formation size
      unchanged.
- [ ] ~~**Builder memo staleness without walking the dependency tree.**~~
      Replaced by 3: const concatenation sized by the dependency count is not
      expressible on stable Rust. `Formation::get` on a memo or view calls
      `Memo::stale` → `F::moved` → `Select::moved` of every dependency,
      recursively, with no deduplication of shared dependencies: a
      diamond-shaped graph grows exponentially, and LLVM folds it only when it
      inlines the whole tree. A constant list of the bit indices that move the
      memo, built by const concatenation, would make it a loop over a constant
      array. Measure first: `strata!` tried one constant mask with `intersects`
      and lost 30% fuel on a 100-word bank (see _Kept as is_); a list of indices
      tested with `get` avoids walking bank bytes.
      `crates/outcrop/src/stratum.rs`, `crates/outcrop/src/compute.rs`
- [ ] ~~**`strata!` `Default` without moving values through the chain.**~~
      Changed into 6. Each entry does `let chain = input(local, chain)`, moving
      the value and the whole chain into a new tuple, and `unstore`/`unbank`
      move them back out: about n²/2 bytes copied and a stack slot per shadowed
      `chain` without optimisation, which matters for large banks in debug or
      `opt-level = "s"` builds. A borrowed layer (`Settle::Ref = &'a T`,
      `Signal = T`) and a borrowed bank would let `compute` and `distinct` see
      the same types while the values stay in their locals. Measure peak stack
      in a debug build. `crates/outcrop-macros/src/expand.rs`,
      `crates/outcrop/src/__private.rs`
- [ ] ~~**One forwarding impl per fed entry instead of per router × entry.**~~
      Dropped: 300 impls cost 20 ms of `cargo check` and 0.4% of a release
      build, with no runtime cost (see _Kept as is_), leaving nothing to win.
      Revisits the _Kept as is_ entry on router forwarding: a `Below<'s, P>`
      wrapper generic over a const position, bounded by a "declared before `P`"
      trait each fed entry implements once. Only worth it if it keeps the
      "cannot be fed" error at type-check time; the earlier attempt traded it
      for a post-monomorphisation one. `crates/outcrop-macros/src/expand.rs`

## Review of 4465815

Found by `/code-review` of the commit that closed items 4 and 6 of _Redesigns_.
No crash or wrong output; diagnostics, duplication and tests.

Fixed together and measured against a snapshot of 4465815: code size, fuel,
reports, peak stack and formation size unchanged;
`cargo check --workspace --all-targets --all-features` after cleaning both
crates, best of five interleaved runs, 1407 -> 1369 ms, noise; the debug frame
of `Default` for a struct with a `[u16; 1000]` bank, a view and a memo unchanged
at 2120 B on x86_64. Design choices made after a probe by a second reviewer.

- [x] **The `Sound` diagnostic never showed.** A layer holding a type twice is
      an ambiguity error, not a missing impl:
      `Formation::new().input(0_u8).input(1_u8)` gives two E0283 "type
      annotations needed" about `Feed<u8, _>`/`Select<u8, _>`, and stable Rust
      cannot customise those (a const `TypeId` comparison needs unstable const
      `PartialEq`). When an impl was really missing, as in `.nest(M)` with a
      hand-written `Strata` struct, the `Nestable` message came with a `help:`
      listing the private `Sound` impls. The dead `on_unimplemented` is gone and
      `nest` requires `N: Nestable<S, I>` directly, so that case now reads "`M`
      cannot be nested on these strata" with its note and nothing else;
      `deposit` still requires `Sound`, which the `Nested` impl satisfies.
      `crates/outcrop/src/plumbing.rs`
- [x] **The spanned duplicate error was buried.** `strata!` checked the entries
      it flagged as "a signal of this type is already declared above" anyway, so
      `struct S { a: A = A(0), b: A = A(1) }` gave that error plus two E0283,
      five with a memo on `A`. The flagged entries are now left out of the
      expansion, so both cases give that one error, plus an E0609 for each use
      of a dropped field. Duplicates the scan cannot see, aliases or entries
      with attributes, are still rejected by the `nestable` call.
      `crates/outcrop-macros/src/expand.rs`
- [x] **`Borrowed` and `BorrowedBank` copied `Input` and `Bank`.** About 50
      lines, the `Layer`, `Select<E, Slotted>` and `Settle` impls and the
      `Banked` alias, that had to agree on flag layout and slot indexing.
      `Input<T, A = T>` and `Bank<T, N, BYTES, A = [T; N]>` now take their
      storage as a parameter, read through `Borrow`, and `strata!`'s chain uses
      `Input<T, &T>` and `Bank<T, N, BYTES, &[T; N]>`; `Feed`, `Sound` and the
      bank's `LayerReport` stay on owned storage. `borrowed.rs` is gone. Builder
      errors still print `Input<u8>`; only errors from the chain show
      `Input<A, &A>`. `crates/outcrop/src/stratum.rs`
- [x] **`holds` repeated the `Nestable` predicates.** `Default` rendered the
      `checks` list a second time, one obligation per entry. It now makes one
      `__private::nestable::<Self, (), _>()` call. The span the per-entry calls
      seemed to buy was not there: for `type B = A;` declared as `a: A, b: B`,
      or two `#[cfg(all())]` entries of one type, 4465815 and the single call
      both give two E0283 at the macro call, since the `__private` path keeps
      the call-site span. `crates/outcrop-macros/src/expand.rs`
- [x] **The `compile_fail` doctests on `input` passed for other reasons.**
      `input::<u8, ((), ())>(1)` fails whether or not the types repeat, and
      stable rustdoc ignores `compile_fail` error codes, so pinning them
      protects nothing. Each rejected case now sits beside a compiling twin that
      differs only in the repeated type: `.input(0_u8).input(1_u8)` against
      `1_u16`, and the router case against `&'static u16`. The `((), ())`
      doctest stays, for unnameable indices (item 2 of _Redesigns_).
      `crates/outcrop/src/formation.rs`
- [x] **`Default` built the borrowed chain for structs without a memo.** It is
      now built only when the struct declares a memo or a view: views use it
      too, to reject a dependency declared below them with "not a signal
      declared above". `crates/outcrop-macros/src/expand.rs`
- [x] **`strata!` hits the recursion limit earlier than documented.** The docs
      said around 120 signals; measured, the same before and after this section,
      a memo or view with 64 entries or more above it fails with E0275 (63
      compile), and a struct of more than 128 signals fails with "queries
      overflow the depth limit!" from the chain of `__OUTCROP_BITn` constants.
      `#![recursion_limit = "256"]` lifts both (250 inputs, or a memo above 100,
      compile). Accepted and documented on `strata!`: each limit grows from a
      chain the declaration order defines. `crates/outcrop-macros/src/lib.rs`

## Review of 30d52f1

Found by an `effective-rust` review of the whole workspace. Items 1–3 were
reproduced in a scratch crate; item 4 with `cargo +nightly fmt --all --check`.

### Bugs

- [x] **1. A builder formation with a memo was neither `Clone` nor `Copy`.**
      `Memo` derived both, so the derive also required the marker `M`
      (`(fn(&A) -> B, (Here,))`) to be `Clone`, and `Here` and `There<I>` are
      not: `Formation::new().input(A(0)).memo(|a: &A| B(a.0)).clone()` failed
      with E0599. `Memo` now implements them by hand, bounded on `F` and `O`
      alone, as `View` and `Router` do. Tested by `clones_keep_their_own_memos`
      and `copies_keep_their_own_memos`. `crates/outcrop/src/stratum.rs`
- [x] **2. A `strata!` router over `dyn A + B` failed to compile.** The
      expansion spliced the pointee after `&` in `&#pointee` and
      `&'static #pointee`, so `dyn Tr + Send => |f, sink| ..` gave "ambiguous
      `+` in a type". `referent()` now puts a trait object with several bounds
      in parentheses, and only that: parenthesising every type would trigger
      `unused_parens` in the generated code. `impl Trait` cannot name a router
      type, so it needs no case. It looks through `Type::Group` too, since rustc
      drops the invisible group of a `$t:ty` fragment from the expansion. Tested
      by `routers_take_trait_objects_with_several_bounds` and, through the
      `routed!` wrapper, `routers_take_trait_objects_passed_as_a_type_fragment`.
      `crates/outcrop-macros/src/expand.rs`
- [x] **3. A tuple of dependencies passed through a `$d:ty` fragment counted as
      one dependency.** `dependencies()` matched `Type::Tuple` and `Type::Paren`
      but not the `Type::Group` that `macro_rules!` wraps a `ty` fragment in, so
      `m!((A, B))` gave "`(A, B)` is not a signal declared above". It now looks
      through `Type::Group` first. Tested by the `typed!` wrapper and
      `dependencies_passed_as_a_type_fragment_are_split` in `tests/strata.rs`.
      `crates/outcrop-macros/src/expand.rs`

### CI

- [x] **4. The `rust-fmt` and `prettier-fmt` CI jobs failed.**
      `cargo +nightly fmt --all --check` wanted five doc comments rewrapped, in
      `bank.rs`, `formation.rs` (two), `strata.rs` and
      `outcrop-macros/src/lib.rs`; stable `cargo fmt` ignores `wrap_comments`,
      so only nightly sees them. Prettier wanted `TODO.md`,
      `.github/workflows/ci.yml` and
      `docs/decisions/0001-view-change-detection.md` rewrapped. All reformatted
      with those tools, no content change; taplo and typos were already clean.

### Smells

- [x] **5. `///` on a `strata!` entry gave three `unused_doc_comments`
      warnings.** Entry attributes are copied onto the `let` and `if` statements
      of the expansion, so the natural way to document a field broke
      `-D warnings` builds. `parse.rs` now takes `doc` attributes out of the
      entry's attributes and puts them first among the field attributes of an
      input, bank or memo, or the function attributes of a view or router. A
      `doc` inside `cfg_attr` is still copied everywhere. Tested by the
      `documented` module of `tests/strata.rs`, which denies
      `unused_doc_comments`, and
      `puts_doc_comments_on_the_field_or_else_the_function`; the `strata!` docs
      now show `///` instead of `field(doc = ..)`.
      `crates/outcrop-macros/src/parse.rs`
- [x] **6. Unused `syn` feature `visit-mut`.** Nothing used it; removed.
      `crates/outcrop-macros/Cargo.toml`
- [ ] ~~**7. `Check.kind` names `__private` types as strings.**~~ Dropped. An
      enum would still print `Signals` and `Feeds` as identifiers, like every
      other `#private::…` path in the expansion (`ne`, `set`, `Array`, …), and a
      rename in `__private` already breaks `tests/strata.rs`, `tests/nest.rs`
      and the `ui` suite. `crates/outcrop-macros/src/expand.rs`
- [x] **8. `Chunk` defined its operations twice.** The inherent `ALL`, `get`,
      `set` and `clear` repeated `Bits::ALL`, `get`, `set_top` and `clear_top`.
      Only the `Bits` impl is left, and `__private` and the bank feed call it.
      Measured against a snapshot of the tree before: code size, fuel, reports,
      peak stack and formation size unchanged. `crates/outcrop/src/flags.rs`
- [ ] ~~**9. The example's register decoders are conversions.**~~ Dropped.
      `BaudRate::new` and `Parity::new` are the memo functions of the
      `Option<BaudRate>` and `Option<Parity>` signals, passed to `strata!` by
      path. `TryFrom` would add an error type, a `|r| ..try_from(r).ok()`
      closure per signal and lose `const`. `examples/aidoo/src/modbus.rs`
- [x] **10. Macro tests and leftovers.** `crates/outcrop-macros/src/`
  - [x] The eight `rejects_*` tests of parse errors moved from `expand.rs` to a
        new test module in `parse.rs`, which only parses.
  - [x] Test router inputs written `&[u8] => f`, which expanded to `&&[u8]`, are
        now `[u8] => f`.
  - [x] `Memo` and `View` hold `deps: Type`; only `Shape::Derived` boxes it,
        which is all `clippy::large_enum_variant` asks for.
  - [x] `ToTokens for Declaration` existed only for `into_token_stream()` in
        `lib.rs`, which now calls the plain `pub fn expand()`.
  - Kept: `.clone()` of `private` and `chain` in `Expansion` methods, which
    works around borrowing `&mut self`, clones two small token handles.

### Lints

- [x] **11. Align the lints with the plain-`pub` preference.** For all Rust
      projects: plain `pub`, never narrowed to `pub(crate)` or `pub(super)`, and
      `unreachable_pub`, `unnameable_types` and `redundant_pub_crate` never
      enabled. A `pub` item that is not exported from the crate root is already
      crate-private, so restricted visibility only adds clutter. The lint and
      fmt baseline is the az-update workspace config of 2026-11-04, which leaves
      those three lints out. outcrop enables all three in the root `Cargo.toml`.
      That causes the `pub(crate)` and `pub(super)` items in
      `crates/outcrop/src`, the `expect(unnameable_types)` on four modules in
      `lib.rs`, `expect(unreachable_pub)` on `slot` and `read`,
      `expect(clippy::redundant_pub_crate)` in `examples/aidoo/src/main.rs`, and
      the inner `allow`s in `tests/common/mod.rs`. Drop the three lints, switch
      to plain `pub` and remove those attributes. Item 2 of _Redesigns_
      (unnameable builder indices) is unaffected: it relies on the private
      module, not on the lint.

      Done: the three lints, the `expect`s on `lib.rs`, `slot` and `read`, the
      example's `expect` and the `unreachable_pub` allow in `tests/common` are
      gone; `stratum.rs`, `nested.rs` and the Aidoo example use plain `pub`.
      Exception, by choice: inherent methods on types users can reach keep
      `pub(crate)` so they stay out of the public API: `Write::apply`,
      `Element::report` and `Chunk::bytes_mut` (`Chunk` is
      reachable through `__private`). `pub(crate) struct Spelled` in
      `tests/strata.rs` stays too, as input checking that `strata!` passes
      the visibility through. The generated code keeps its
      `#[allow(unnameable_types, ..)]`, for user crates that enable the lint.

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
      `strata!` have them. `Sink`, `Visit`, `Feed`, `Get` and `Report` have
      none, and neither do `Write` and `Element` beyond the `Formation::bank`
      docs.
- [x] **Seal or document `Strata`, `Feed`, `Get` and `Report` (C-SEALED).**
      Documented, not sealed: `strata!` implements them in the user's crate, so
      anything it can name the user can too, and `tests/manual.rs` implements
      them by hand. The `Strata` docs list the rules an impl must keep.
- [ ] **`Element` has public fields (C-STRUCT-PRIVATE).** `index` and `value`
      can never be joined by another field without a breaking change.
      `#[non_exhaustive]` would do, at the cost of callers constructing one.
- [ ] **`Write` name.** It collides with `std::io::Write` and `core::fmt::Write`
      when imported beside them.
- [ ] ~~**`Reading` combinators.**~~ Dropped with `Reading` itself.

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
- [x] **Hand-written `Strata`, `Feed`, `Get`, `Report` and `__private::Signal`
      impls can break all of the above.** Documented with the rules they must
      keep, on `Strata` and `__private::Signal`; breaking them only affects the
      impl's own formations, with no unsafe code to misuse. A wrong
      `Signal::BIT` or mask makes a `Nested` lookup read the wrong bit, calling
      `evaluate`, `report` and `feed` directly skips the order `Formation::poll`
      keeps, and a `Report` that does not clear the bits it visits reports them
      again after a panicking visitor. See C-SEALED above.
      `crates/outcrop/src/nested.rs`

### Partial checks

- [x] **Uniqueness depended on declaration order.** Now rejected at declaration
      in either order: `input` checks feeds as well as signals, at the cost of a
      third type parameter (`input::<T, _, _>`), and `nest` requires
      `__private::Nestable`, which `strata!` implements by checking each of the
      struct's signals and fed types against the formation; an entry compiled
      out by `cfg` swaps its check for one that always holds, through a pair of
      shadowing `macro_rules!`, as `cfg` on `where` predicates is unstable. Bank
      slots remain checked only where read: the `Slot` types of an item are an
      open set. Documented on `Formation::bank` and `nest`.
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
2. **Formation lifecycle**, seen through `get`: built (every bit set, memos hold
   their declared value) → fed (inputs current, memos hold their last value) →
   polled (memos current). The formation never tells the two apart: the caller
   feeds and polls, so it knows. The flags take part in `Formation`'s
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
   just before its signal is visited under `panic = "unwind"` → reset every
   flag. Fixed only inside `Formation::poll`.
7. **Router state:** builder routers keep state in their `FnMut`; `strata!`
   routers cannot capture, so their state lives untracked in statics.
8. **`strata!` `Default` chain:** built only when the struct declares a memo or
   a view: one borrowed layer pushed per entry in declaration order, to compute
   memo initial values and type-check view dependencies; the values stay in
   their locals and move into the struct once the chain is dead.
9. **`strata!` cfg detection:** two shadowing `macro_rules!` give each entry a
   compiled-in or compiled-out state, feeding bit numbering and visitor bounds.

## Kept as is

- Builder banks and nested structs take whole bytes of change bits: the bits
  left in the byte beneath them and in their last byte go unused, and the next
  signal starts a new byte. `input(A).bank([_; 4]).input(B)` takes three bytes
  where one would do (formation 9 B instead of 7), and a bank deposited first
  leaves the empty formation's byte unused (6 B instead of 5). Advancing the
  `Used<_>` cursor by N bits needs const generic arithmetic, unstable in Rust;
  at most two bytes are lost per bank or nested struct. `strata!` packs its
  banks with its other signals. Documented on `Formation::bank` and `nest`.
- The `Layer` impls of `Bank` and `Nested` repeat `Cursor = Used<8>` and
  `below_mut`: sharing them through a helper trait needs a second blanket
  `Layer` impl, which overlaps the one over `Value + Settle<Tail>`.
- `Distinct` and `Routed` look like plain aliases of `Get` and `Feed`, but work
  around rustc#152409, where such a where-bound shadows `Strata::Flags`.
- `slot` and `read` live once, `pub` in `stratum.rs` and re-exported by
  `__private`.
- The loops in `tools/measure/workload` stay hand-written: rewriting them with
  `core::array::from_fn` could shift the baselines the workload measures.
- The Aidoo example keeps its `From<u16>` unit impls and `bit()` matches written
  out: no `macro_rules` in the example, and `as` casts trip `as_conversions`.
- `signal()` and `bank()` in `expand.rs` stay separate apart from `declare()`,
  which holds the marker, bit and signal bookkeeping they share; the rest
  differs (N bits, one generic impl for every slot, reporting `Element`s, no
  per-signal duplicate check).
- `strata!` tests a memo's or view's dependencies one `is_set` each rather than
  one constant mask with `intersects`: measured, the mask saved 0.2% of fuel on
  the workload's 4-byte flags but cost 30% (831 -> 1079) with a 100-word bank,
  as `intersects` walks every byte of the flags and LLVM does not fold the
  mask's zero bytes away. The masks are gone with item 3 of _Redesigns_.
- Each `strata!` router keeps a `Feed` forwarding impl per entry fed before it:
  measured on 100 inputs, three more routers (300 more impls) add about 20 ms to
  `cargo check` (104 -> 124 ms) and 0.4% to a release build, with no runtime
  cost; a generic impl would trade the "cannot be fed" error for a
  post-monomorphisation one, or still need an impl per entry.
