# Changelog

All notable changes to this project are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `gtfs-rt-model`: dependency-free protobuf wire-format reader with anomaly reporting.
  Reports truncated payloads, over-long varints, unexpected and reserved wire types,
  deprecated group encoding, declared lengths beyond the buffer, and zero field numbers.
  The reader stops at the first fault rather than guessing at re-alignment.
- Anomaly model separating wire-level framing faults from schema-level observations.
- Full GTFS-Realtime message model: 26 messages and 12 enums decoded from proto2,
  including `Shape`, `Stop` and `TripModifications`. Every field is optional in the
  type system; proto2 `required` is enforced as an anomaly so a missing field degrades
  the result instead of discarding it.
- Schema-level anomalies: unknown and extension-range field numbers, missing required
  fields, `FeedEntity` carrying several payloads or none, unrecognised enum values,
  and invalid UTF-8. Enum gaps in the specification are preserved rather than filled,
  so a producer sending `TripDescriptor.ScheduleRelationship = 4` is reported.
- `decode_feed_message` entry point returning the decoded feed alongside its anomalies.
- `proxy/`: a restricted CORS proxy as a Cloudflare Worker, for feeds that do not send
  `Access-Control-Allow-Origin`. It fetches allow-listed addresses only, returns the bytes
  untouched as `application/x-protobuf`, stores nothing, and leaves all parsing and
  validation to the browser.
- `inspect` example for decoding a `.pb` file during development.

### Fixed

- Unknown stop ids are reported even when the referenced trip has no rows in
  `stop_times.txt`. The check previously sat behind an early return, so a static feed with
  broken stop times silently hid unknown stops.
- New `RT_TIME_ON_NON_STOPPING_UPDATE`: a stop marked `SKIPPED` or `NO_DATA` that still
  carries an arrival or departure prediction.
- Caps on the static reader: a row ceiling for the streamed `stop_times.txt` and a size
  ceiling for the tables read whole. Calibrated against measurement — a 2 MB archive
  expanding to 1.5 GB cost 3.82 GB on the unfiltered path before the caps and 1.03 GB after,
  while a real MBTA feed passes untouched. The ceilings are checked at compile time against
  measured feed sizes, so narrowing one below a real feed fails the build.
- Fuzzing covers the ZIP and CSV path of the static reader and the full production sequence,
  alongside the existing protobuf target. Seeds and crash artifacts are tracked; the
  generated corpus is not.
- The static reader indexes only the trips a snapshot actually references, and streams
  `stop_times.txt` instead of holding it in memory. Measured on MBTA (33 MB archive,
  168.145 trips, 4.494.139 stop-time rows, of which a snapshot referenced 883): peak memory
  fell from 2.27 GB to 174 MB and parsing from 2504 ms to 468 ms. In the browser the combined
  analysis went from 19.9 s to 6.1 s, producing an identical report.
- `StopTime` no longer carries `trip_id`; the records are only ever reached through the
  per-trip index, so storing the key on every row cost 4.5 million redundant allocations.
- The consistency rules are reachable from the browser. `analyze_feed_with_schedule` takes a
  realtime payload and a static GTFS archive and returns the consistency report alongside the
  realtime one; the UI accepts a schedule and renders it.
