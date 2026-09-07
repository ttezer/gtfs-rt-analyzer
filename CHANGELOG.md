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
