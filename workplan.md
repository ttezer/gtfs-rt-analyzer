# Work plan

Findings from a review of the current tree, ordered by what blocks what. Each item is
committed on its own.

## 1. Run the quality gates in CI — *blocking everything else*

There are 105 tests and none of them run on push. Two workflows exist (`pages.yml`,
`schedule-scores.yml`) and neither invokes `cargo test`, `cargo clippy` or `vitest`, so a
regression reaches the published site without anything turning red.

This comes first: the changes below alter the rule layer and the WebAssembly boundary, and
a gate added afterwards would not have covered them.

- [x] Workflow running `cargo test --workspace`, `cargo clippy --workspace --all-targets -D warnings`, the proxy's `vitest` suite and `tsc --noEmit`

- [x] `cargo fmt --all -- --check` as a gate. Deferred at first because the tree did not
  satisfy rustfmt and the reformatting would have touched files unrelated to the work above;
  done afterwards as its own change. 114 sites across 16 files. Test results are byte-identical
  before and after, so the reformatting changed no behaviour.

## 2. Correct the consistency rules

- [x] **`stop_id` membership no longer depends on `stop_times`.** `check_stop_time_update`
  returned early when a trip had no stop times, which also silenced the check for whether
  a `stop_id` exists in `stops.txt`. Two independent claims, one of them conditioned on the
  other's data being present.
- [x] **Read `StopTimeUpdate.schedule_relationship`** — though not for the reason the review
  gave. The review called it a false-positive source and expected the reference checks to be
  relaxed for `SKIPPED`. Checking the specification showed the opposite: all four values
  assume the stop exists in static `stop_times.txt`, so relaxing anything would have opened a
  hole. What the field is good for is the inverse check — a stop marked `SKIPPED` or `NO_DATA`
  should not carry an arrival or departure prediction.
- [x] **Cover the two untested rules.** `RT_STOP_NOT_IN_STATIC` and
  `RT_STOP_SEQUENCE_NOT_IN_STATIC` had no test, so nothing proved they could fire at all.

## 3. Make the rules reachable from the browser

`gtfs-rt-wasm` depends only on `gtfs-rt-model`. The static reader and the consistency rules
— the product's core claim — are compiled, tested and committed, but not wired to anything
the browser can call. `analyze_feed` decodes protobuf and stops there.

- [x] `gtfs-rt-wasm` depends on `gtfs-static` and `gtfs-rt-rules`
- [x] An exported entry point that takes a realtime payload plus a static GTFS archive and
  returns the consistency report
- [x] UI accepts a static feed and renders the report

## 4. Stop the WebAssembly bundle from drifting

`ui/pkg/*.wasm` is committed and built by hand, and Pages publishes `ui/` verbatim. Today
the bundle matches its sources, but once item 3 lands, a forgotten rebuild would leave the
published rules behind the repository with no gate to catch it.

- [x] Solved at the root instead: the bundle is no longer committed. The deploy workflow
  builds it from source, so there is nothing to drift.

A byte-comparison gate was measured first and rejected. The build is reproducible on one
machine, but `wasm-opt` comes from the local toolchain (Homebrew v130 here) while CI would
use the one `wasm-pack` fetches — so identical sources would produce different bytes and the
gate would fail for the wrong reason. Removing the committed copy makes the question moot.

## 5. Fuzzing and input limits — added after review

- [x] Fuzz targets for the ZIP/CSV path (`read_schedule`) and the production sequence
  (`analyze_with_schedule`), alongside the existing `decode_feed`. The fuzz crate moved from
  `crates/rt-model/fuzz` to the repository root, since it now covers three crates.
- [x] Fuzz runs in CI, time-boxed to 60 s per target with memory and timeout limits — a
  decoder that never panics but exhausts memory or spins forever fails the same contract.
- [x] Seeds tracked, generated corpus not. Measured at 24 MB after one session.
- [x] Caps on the static reader, calibrated by measuring a real zip bomb rather than guessing.

Measured along the way: fuzzing will not generate a zip bomb on its own, so that case was
constructed by hand. The production path turned out to be nearly immune already — filtering
means the rows are never retained — but the unfiltered entry point was not, and the cost in
CPU remained either way.
