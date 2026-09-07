# gtfs-rt-analyzer

An independent GTFS-Realtime analyzer written in Rust.

> **Status: early development.** The protobuf decoder, a WebAssembly report adapter and
> a static browser UI exist today. Static GTFS cross-checks and validation rules are not
> implemented yet. Nothing here is stable.

## Why another one

The GTFS-Realtime tooling landscape is unusually concentrated. The only maintained
validator descends from a single codebase (CUTR/USF → MobilityData); it is written in
Java, requires Maven and a database, describes itself as being in an *early alpha state*,
and its issue tracker carries 52 unimplemented rule requests that have been open since 2022.

Separately, several good browser-based tools exist for *inspecting* realtime feeds — but
none of them validate. Nothing bridges inspection and validation, and there is no
GTFS-Realtime validator in Rust at all.

## What this aims to answer

The core question is **static ↔ realtime consistency**: does the realtime feed still
describe the schedule it claims to? This is where operators hurt most — a static feed
that stops being updated while the realtime feed keeps publishing trip IDs that no longer
exist produces "ghost buses": vehicles reporting positions that no journey planner can
match to a trip.

Three layers are planned, in this order:

1. **Protobuf and specification correctness** — a precondition, not the product
2. **Static ↔ realtime consistency** — the core claim
3. **Published quality thresholds** — the ones measurable from a single snapshot

Continuous monitoring is deliberately out of scope for now.

## `gtfs-rt-model` — the decoder

A dependency-free GTFS-Realtime protobuf decoder. It is not a validator: it has no notion
of rules or severity. It reads a payload and, alongside the decoded message, reports the
structural anomalies it saw while reading.

This is the reason it is hand-written rather than generated. Protobuf libraries are
deliberately **tolerant** — they skip unknown fields, ignore mismatched wire types, accept
missing proto2 `required` fields and disregard trailing bytes. A realtime validator's job
is partly to report exactly those things. Both existing Rust GTFS-RT crates are built on
`prost`, so both discard this information, and both need `protoc` at build time.

What the decoder reports:

| Level | Anomalies |
|---|---|
| Payload | HTML/XML/JSON body detected as `not_protobuf` |
| Wire | truncated payload · trailing garbage · varint over 10 bytes · unexpected wire type · deprecated group encoding · reserved wire type · declared length beyond buffer · zero field number · nesting past the depth cap |
| Schema | unknown field number · extension-range field · missing proto2 `required` field · `FeedEntity` with several payloads or none · unknown enum value · invalid UTF-8 |

Guarantees: no panics on any input, deterministic output for identical bytes, and partial
results are returned rather than discarded when a payload is broken.

## Building

```
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

`gtfs-rt-model` has zero dependencies; `cargo tree` should show a single line.

The browser adapter is built separately for WebAssembly:

```
cargo build --target wasm32-unknown-unknown --release -p gtfs-rt-wasm
```

The static browser UI is in `ui/`. Build its WebAssembly package with `wasm-pack`,
then serve the directory over HTTP:

```
wasm-pack build crates/rt-wasm --target web --release --out-dir ../../ui/pkg
python3 -m http.server 4173 --directory ui
```

The UI prefers direct fetch, reports CORS/timeout/upstream states separately, falls back
to the restricted proxy when configured, and keeps only compact report history in memory.

The decoder also has a libFuzzer target. From the repository root:

```
cargo +nightly fuzz list --fuzz-dir crates/rt-model/fuzz
cargo +nightly fuzz run decode_feed --fuzz-dir crates/rt-model/fuzz
```

The workspace itself stays on stable Rust; cargo-fuzz uses nightly for sanitizer
instrumentation.

The integration mutation tests run with the normal workspace test command and
cover truncation, single-bit flips and common boundary/tail mutations.

## License

MIT
