# Open Job Spec — Rust Contrib
![Stability: beta](https://img.shields.io/badge/stability-beta-yellow.svg)

[![CI](https://github.com/openjobspec/ojs-rust-contrib/actions/workflows/ci.yml/badge.svg)](https://github.com/openjobspec/ojs-rust-contrib/actions/workflows/ci.yml)
[![docs.rs](https://img.shields.io/docsrs/ojs)](https://docs.rs/ojs)

Community framework integrations for the [OJS Rust SDK](https://github.com/openjobspec/ojs-rust-sdk).

## Provided Integrations

| Status | Integration | Description |
|--------|-------------|-------------|
| security-blocked | [Actix-web](./ojs-actix/README.md) | Source and tests retained; 0.5.0 publication withheld pending a patched Actix HTTP/2 dependency |
| alpha  | [Axum](./ojs-axum/README.md) | Axum state extractor and Tower layer |
| alpha  | [Diesel](./ojs-diesel/README.md) | Transactional job enqueue via Diesel connection callbacks |

Status definitions: `alpha` (API may change), `beta` (API stable, not
battle-tested), `stable` (production-ready), `security-blocked` (source and
tests retained, but package publication is disabled by an unresolved upstream
advisory).

## Getting Started

Install any integration crate:

```toml
[dependencies]
ojs-axum = "0.5.0"
```

Each crate includes an `examples/` directory with a complete working demo using Docker Compose.

## Compatibility

Version 0.5.0 requires Rust 1.88 or newer and tracks Actix-web 4.15,
Axum 0.8.9, and Diesel 2.3.12. The minimum versions include upstream
security fixes unavailable on the previous Rust 1.75 dependency set.

`ojs-actix` is excluded from the 0.5.0 crates.io release. A normal Actix
application commonly enables Actix-web's default HTTP/2 feature, which still
selects unpatched `h2` 0.3 and triggers RUSTSEC-2026-0258. No released Actix
dependency path using patched `h2` 0.4 is currently available. Track
[actix/actix-web#4199](https://github.com/actix/actix-web/issues/4199).

## Contributing

See [CONTRIBUTING.md](./CONTRIBUTING.md) for guidelines on adding new contrib crates.

## License

Apache 2.0 — see [LICENSE](./LICENSE).
