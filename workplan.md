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

`cargo fmt --check` is deliberately not a gate yet: the tree does not currently satisfy
rustfmt, and adding the gate would require reformatting files this work does not otherwise
touch. Worth doing as its own change.

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

- [ ] `gtfs-rt-wasm` depends on `gtfs-static` and `gtfs-rt-rules`
- [ ] An exported entry point that takes a realtime payload plus a static GTFS archive and
  returns the consistency report
- [ ] UI accepts a static feed and renders the report

## 4. Stop the WebAssembly bundle from drifting

`ui/pkg/*.wasm` is committed and built by hand, and Pages publishes `ui/` verbatim. Today
the bundle matches its sources, but once item 3 lands, a forgotten rebuild would leave the
published rules behind the repository with no gate to catch it.

- [ ] CI rebuilds the bundle and fails when the committed copy differs
