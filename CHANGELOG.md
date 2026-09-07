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
