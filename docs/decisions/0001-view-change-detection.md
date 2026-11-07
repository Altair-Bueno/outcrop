# 0001: How views detect changes

- **Status:** accepted, 2026-11-07
- **Chosen:** conservative change detection, commit `061f2ab` on master
- **Alternatives kept:** branches `views/static-gate` (`3925583`) and `views/previous-inputs` (`6b1ea79`). Both branch off `5151753`, the commit before `061f2ab`.

## Context

A view (`Formation::view`, or an unnamed entry in `strata!`) stores nothing and is computed every time it is read. Before `061f2ab`, views were reported only when their value changed. Since there is no stored value to compare against, the old value had to be recomputed. That happened in two places:

- **`feed`:** the builder's `Feed for (E, Tail)` computed a snapshot of **every** view above the fed input whose bit was clear, before passing the value down. It compared the views again afterwards. It did this even for views that don't depend on the fed input, and all the snapshots were alive on the stack at the same time, one per nested `feed` call.
- **`evaluate`:** when a memo beneath a view might change, the view was computed before and after evaluating the strata beneath it.

`strata!` already limited snapshots to views whose ancestors include the fed input. Most of the cost was therefore in the builder.

### A bug found along the way

Before `061f2ab`, the builder missed a view change when the same input was fed twice before a `poll` and the first feed didn't change the view. Example with `anti_freeze` (bit 0 of the register), fed 0x22 → 0x24 → 0x25: the second feed was never compared. The builder only compared a view when the flags beneath changed during the feed, and the input's bit was already set by the first feed. `strata!` was not affected. Regression tests: `views_see_the_last_of_several_feeds` and `views_above_routers_see_the_last_of_several_routed_values` in `crates/outcrop/tests/view.rs`, and `views_see_the_last_of_several_feeds` in `crates/outcrop/tests/strata.rs`.

## Options

### A. Static gate: `views/static-gate` (`3925583`)

Change detection stays exact. Builder feeds go through an internal `Deliver<T, I>` trait that carries the fed input's index at compile time; for a router feed the target is "anything beneath the router". A view is snapshotted and compared during a feed only if `ComputeImpl::fed_by(target)` is true, meaning a dependency is the target or is a view reachable from it. Memo edges don't count, because memos only change during `poll`, where `pending` handles them. The check folds to a constant, so views the feed can't reach cost nothing. Feeding an input the value it already holds returns before computing any view. In `strata!`, `ancestors` became `sources`, which follows view edges only.

- Fixes the bug above.
- Public API unchanged, apart from compiler errors now mentioning `plumbing::Deliver`.
- **Weak spot:** routers. The router closure can feed anything beneath it, so every view above a router is still compared on every routed frame. Making this exact would require the public `Feed`/`Sink` signatures to say which inputs a router feeds.
- `evaluate` still keeps a view's old value alive while it evaluates the strata beneath. That's why the builder's stack remains about 2.5× the conservative one.

### B. Conservative: chosen, on master (`061f2ab`)

Views are never compared. During `poll`, a view's bit is set whenever a dependency's bit is set. This is applied bottom-up, so it passes through views and memos. Memos still compare their own value, so propagation stops at the first memo whose value didn't change.

- `feed` only stores the value and sets the bit. `evaluate` computes nothing for views.
- Views no longer need `PartialEq` on their output, in both the builder and `strata!`.
- **Cost:** a view can be reported even though its value didn't change, and memos above it recompute. They are still not reported if their own value is unchanged.
- **Staleness:** a memo is `Stale` whenever any signal beneath it changed since the last poll, looking through views. A view is `Stale` only when a memo it depends on is.

### C. Previous inputs: `views/previous-inputs` (`6b1ea79`)

Change detection stays exact, and it moves into `poll`. `Input<T>` becomes `{ value: T, previous: Option<T> }`. The first feed after a poll moves the old value into `previous` with `mem::replace`, so no `Clone` bound is needed. `poll` computes each affected view from the previous values and from the current ones, and compares them. Views over memos are snapshotted from the previous state before those memos are recomputed. In `strata!`, the struct gains an `__outcrop_previous_<name>: Option<T>` field, gated with `cfg`, for each input that some view reads.

- Exact even when an input is fed A → B → A between polls: the views above it are not reported.
- **Costs:**
  - RAM: one `Option<T>` per input (+36 B on the workload builder, +32 B on the macro). Requiring `T: Clone` and storing a plain `T` would roughly halve this.
  - A poll with nothing fed costs about 3× more.
  - The hidden fields appear in the `Debug`/`PartialEq`/`Hash`/serde derives of `strata!` structs, and users can no longer build those structs with a struct literal.
  - `get` on a memo over a view computes that view twice.
- While building it, the agent found that the old "pending" check missed a memo over a view once view bits stopped being set at feed time. It now looks through views to the inputs (test `views_follow_memos_over_views`). Any future design that defers view bits to `poll` needs the same care. B handles it with its "any signal beneath changed" staleness rule.

## Measurements

Harness: `tools/measure`. Run `tools/measure/measure.sh <outcrop checkout> <label>`. The workload (`tools/measure/workload/src/lib.rs`) has:
- 9 inputs, of which 8 are registers and one has no dependents;
- a router;
- 13 views returning 16–32-byte values, including views over views and a view over a memo;
- a `Flag` view (one bit of a register) that never changes in the `feed_flat` scenario;
- 3 memos.

It is built with both the builder and `strata!`. All numbers are the cost of one extra feed+poll round, computed as 32 rounds minus 16 rounds, divided by 16. Each cell reads wasm instructions (wasmi fuel) · peak native aarch64 stack · native ns.

| | master before `061f2ab` | A: static-gate | **B: conservative** | C: previous-inputs |
|---|---|---|---|---|
| Code size, builder / macro (thumbv7em `.text`, bytes) | 52.8k / 19.4k | 24.0k / 19.1k | **12.2k / 7.6k** | 21.1k / 15.8k |
| Formation RAM, builder / macro | 64 / 60 B | 64 / 60 B | 64 / 60 B | 100 / 92 B |
| Builder, poll with nothing fed | 3.6k · 711 B · 2.6 ns | 3.6k · 711 B · 2.6 ns | 3.5k · 775 B · 5.2 ns | 9.4k · 823 B · 10.4 ns |
| Builder, feed the bottom input | 20.0k · 1927 B · 83 ns | 6.1k · 983 B · 26 ns | **4.9k · 343 B · 23 ns** | 11.1k · 567 B · 38 ns |
| Builder, feed an input no view depends on | 19.9k · 1927 B · 61 ns | **3.6k** · 407 B · **5 ns** | **3.6k · 343 B** · 8 ns | 9.5k · 535 B · 12 ns |
| Builder, feed the top register | 19.8k · 1927 B · 78 ns | 6.1k · 871 B · 22 ns | **4.9k · 343 B · 16 ns** | 10.9k · 567 B · 30 ns |
| Builder, route frames to all registers | 102k · 1959 B · 348 ns | 92k · 1575 B · 303 ns | **6.6k · 359 B · 47 ns** | 14.0k · 567 B · 100 ns |
| Macro, poll with nothing fed | 4.9k · 263 B · 5.2 ns | 4.9k · 263 B · 6.5 ns | 4.9k · 207 B · 7.8 ns | 10.9k · 327 B · 7.8 ns |
| Macro, feed the bottom input | 7.0k · 407 B · 30 ns | 6.1k · 359 B · 27 ns | **4.9k · 223 B · 17 ns** | 11.2k · 343 B · 31 ns |
| Macro, feed an input no view depends on | 4.9k · 279 B · 5.2 ns | 4.9k · 279 B · 5.2 ns | 4.9k · 223 B · 7.8 ns | 10.9k · 343 B · 7.8 ns |
| Macro, route frames to all registers | 61k · 999 B · 98 ns | 60k · 951 B · 78 ns | **6.6k · 223 B · 46 ns** | 14.0k · 359 B · 103 ns |
| Reports over 16 rounds, `feed_flat` / route | 86 / 406 | 86 / 406 | 102 / 422 | 86 / 406 |

The extra reports under B are `Flag`, reported every round although its value never changes. The `Alarm` memo above it recomputes but is not reported.

### Measurement caveats

- **wasmi fuel:** wasmi charges fuel for compiling each function lazily on its first call. Without a warm-up call, fuel totals shifted 2–9× just from adding unrelated exports to the module. The harness makes one warm-up call per scenario and reports the extra-round difference. Treat fuel as an approximate wasm instruction count, not native instructions. valgrind isn't available on macOS arm64.
- **Stack:** the stack is measured by filling unused stack with a pattern before the scenario runs. The top 64 bytes below the measuring frame are not checked, and the figure is net of an empty baseline.
- **Native ns:** each figure is the fastest of 500 runs on the host, so expect ±1–3 ns of noise on the small numbers.
- **Code size:** fat LTO, `opt-level = 3`, `panic = "abort"`, minus an empty binary.

## Decision

**B: conservative.**

1. It is best or tied on every measure of time and space in both front ends: about 4× smaller code than master and 2–3× smaller than A or C, at most 359 B of peak stack, and routing an order of magnitude cheaper.
2. It gives each derived signal one job. **A view is cheap and may report a change that didn't happen. A memo stores its value and reports a change only when there is one.** To get exact reporting for a view, wrap it in a memo. Comparing a view's old and new value means computing it twice. Leaving that comparison to a memo is cheaper than doing it inside the view, even counting the recomputation of memos above a view that didn't really change.
3. Its code is the simplest: it removes the snapshot machinery (`Settle::snapshot`/`differs`, `observe` in `strata!`) rather than adding to it. That simplicity is how it avoids the several-feeds bug.
4. It drops the `PartialEq` requirement on view outputs.

A was rejected because it keeps all the comparison machinery and still does badly on routers, which is how Modbus frames reach the inputs. C was rejected for its per-input RAM cost, its 3× more expensive idle poll, and the hidden fields in `strata!` structs.

## When to revisit

- **If extra view reports become a problem**, for example a visitor that does I/O on every report and has views that change rarely:
  - first, turn those views into memos;
  - if that's not enough, A is the best exact option when inputs are mostly fed directly;
  - C is the best exact option when routers feed most inputs and RAM is not tight.
- **If A is revived**, it could become exact for routers by giving `Sink` a type-level record of which inputs the router feeds. That would be a public API change.
- **If C is revived**, consider `T: Clone` with a plain `previous: T` instead of `Option<T>`, and reuse its fix that makes the pending check look through views.
- **Ideas compatible with B:**
  - A feed that leaves the value unchanged already does nothing under B, because inputs compare before setting their bit. A's early return adds nothing here.
  - Individual views could opt in to exact detection, for example with a `view_exact` that reuses A's static gate, while B stays the default.
