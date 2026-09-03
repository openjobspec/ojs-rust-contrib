# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.5.0] - 2026-09-02

Repository tags follow the OJS release train. Individual integration crate
versions are declared independently in their Cargo manifests.

### Fixed

- Added the registry version requirement required to package the OJS SDK dependency
- Raised the MSRV to Rust 1.88 and upgraded Actix-web, Axum, Diesel, and `time` to releases containing the required security fixes
- Disabled Actix's default HTTP/2 feature in repository validation, but confirmed Cargo feature unification can restore vulnerable `h2` 0.3 in normal applications
- Withheld `ojs-actix` 0.5.0 from crates.io and release automation pending an upstream Actix path to patched `h2` 0.4
- Made Actix worker start/stop ownership atomic, generation-aware, and cancellation-safe
- Actix worker shutdown now invokes the SDK graceful-drain path and waits for active jobs to finish
- Fixed Axum SSE interval wakeups and removed panic-prone cron response conversion
- Standalone examples now resolve the same OJS 0.5.0 SDK identity as their integration crates

### Changed

- CI now uses locked build/test/clippy gates, package file-set checks, publish dry-run preflight, a clean consumer smoke check, and build/test coverage for every excluded example workspace

## [0.4.1] - 2026-04-21

### Documentation

- Added the missing v0.4.0 changelog entry for the Rust integrations

## [0.4.0] - 2026-04-20

### Changed

- Actix-web, Axum, and Diesel integration updates
- Aligned with ojs v0.4.0 (Rust 1.75+ MSRV maintained)

## [0.9.0] - 2026-02-20

Release candidate for v1.0.

### Stabilized

- **ojs-actix** — Actix-web middleware and extractor for OJS, promoted to 0.9.0
- **ojs-axum** — Axum state extractor, Tower layer, and shutdown utilities for OJS, promoted to 0.9.0
- **ojs-diesel** — Transactional outbox pattern via Diesel for OJS, promoted to 0.9.0

### Changed

- All crate versions bumped from 0.1.0 to 0.9.0
- Expanded integration test coverage across all crates
