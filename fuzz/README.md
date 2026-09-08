# fuzz

Coverage-guided fuzzing for the paths that read untrusted input.

Ordinary tests check the cases someone thought of. These check the ones nobody did — which
matters here because every byte this code reads comes from somewhere we do not control: a
realtime feed off the public internet, or a GTFS archive the user drops into the browser. A
panic in either becomes a WebAssembly trap, which is a dead tab.

## Targets

| Target | Covers |
|---|---|
| `decode_feed` | The protobuf decoder against arbitrary bytes |
| `read_schedule` | The ZIP and CSV path of the static reader |
| `analyze_with_schedule` | The production sequence: decode, collect referenced trips, read the filtered schedule, run the rules |

`analyze_with_schedule` splits its input in two — the first two bytes give the length of the
realtime payload, the rest is the archive — so the fuzzer discovers the split itself.

## Running

```
cargo +nightly fuzz run decode_feed -- -max_total_time=60 -rss_limit_mb=2048 -timeout=10
```

`-rss_limit_mb` and `-timeout` matter as much as the crash check: a decoder that never
panics but exhausts memory or spins forever fails the same contract.

## Seeds and artifacts are tracked, the corpus is not

`seeds/` holds hand-written starting inputs — a minimal valid GTFS archive, and a highly
repetitive one so the fuzzer meets compression behaviour early. Copy them into `corpus/`
before a run:

```
mkdir -p fuzz/corpus/read_schedule
cp fuzz/seeds/read_schedule/* fuzz/corpus/read_schedule/
```

`artifacts/` is tracked too: a crashing input is the only thing that can prove a fix works,
and losing it means rediscovering the regression from scratch.

`corpus/` itself is **not** tracked. It was measured at 24 MB after a single session and it
changes on every run; committing it would bloat the repository for something reproducible.

## What has been measured

No crashes so far: 4.4M runs on `decode_feed`, 4.2M on `analyze_with_schedule`, 14k on
`read_schedule` (slower per run, since each one inflates an archive).

Fuzzing will not produce a zip bomb on its own — the odds of randomly generating one are
negligible — so that case was measured by hand instead. A 2 MB archive expanding to 1.5 GB
(686x) costs 3.82 GB of memory on the unfiltered path and 4.2 MB on the production one,
which is what the caps in `gtfs-static` are calibrated against.
