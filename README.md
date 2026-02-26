# Open Job Spec — Rust Contrib

[![CI](https://github.com/openjobspec/ojs-rust-contrib/actions/workflows/ci.yml/badge.svg)](https://github.com/openjobspec/ojs-rust-contrib/actions/workflows/ci.yml)
[![docs.rs](https://img.shields.io/docsrs/ojs)](https://docs.rs/ojs)

Community framework integrations for the [OJS Rust SDK](https://github.com/openjobspec/ojs-rust-sdk).

## Provided Integrations

| Status | Integration | Description |
|--------|-------------|-------------|
| alpha  | [Actix-web](./ojs-actix/README.md) | Actix-web middleware and app data integration |
| alpha  | [Axum](./ojs-axum/README.md) | Axum state extractor and Tower layer |
| alpha  | [Diesel](./ojs-diesel/README.md) | Transactional job enqueue via Diesel connection callbacks |

Status definitions: `alpha` (API may change), `beta` (API stable, not battle-tested), `stable` (production-ready).

## Getting Started

Install any integration crate:

```toml
[dependencies]
ojs-actix = "0.1"
```

Each crate includes an `examples/` directory with a complete working demo using Docker Compose.

## Contributing

See [CONTRIBUTING.md](./CONTRIBUTING.md) for guidelines on adding new contrib crates.

## License

Apache 2.0 — see [LICENSE](./LICENSE).
