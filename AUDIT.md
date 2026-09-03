# ojs-rust-contrib — 0.5.0 Release Audit

| Field | Value |
|---|---|
| Repository | `ojs-rust-contrib` |
| Branch | `refactor/clean-code-srp` |
| Scope | `ojs-actix`, `ojs-axum`, `ojs-diesel`, manifests, tests, release automation, and standalone examples |
| Working tree | Intentional source and manifest changes remain unstaged; no commit or push |
| Release | `0.5.0` |
| MSRV | Rust `1.88` |

## Summary

- `ojs-actix` separates handler construction from generation-aware lifecycle
  ownership. Concurrent starts have one owner; stop/start overlap is rejected;
  stale cleanup cannot clear a newer generation; and cancelled stop futures
  reap their task ownership.
- Actix stop now calls the SDK's latched `Worker::shutdown()` and keeps awaiting
  the existing `Worker::start()` future. A focused regression test proves stop
  waits for active work to drain, the handler is not aborted, and the job is
  ACKed rather than shutdown-NACKed.
- `ojs-axum` fixes the interval-stream lost wakeup and returns typed cron JSON
  responders instead of performing panic-prone intermediate conversion.
- All integration crates and examples target the coordinated `0.5.0` source
  API and depend on `ojs = "0.5.0"`. Only `ojs-axum` and `ojs-diesel` are
  publishable in this release; `ojs-actix` is explicitly `publish = false`.
- Pre-release CI checks out the local SDK and writes an ephemeral Cargo source
  patch. Committed manifests contain registry dependencies and no sibling path
  replacement.
- The repository intentionally has no committed `Cargo.lock`: registry
  `ojs 0.5.0` does not exist before the coordinated SDK release, so a committed
  lock generated through a local patch would not represent final registry
  source resolution.

## Security and compatibility

The previous Rust 1.75 dependency set had five RustSec vulnerabilities and four
unsoundness warnings across Diesel, `time`, and Actix's optional HTTP/2 stack.
Version 0.5.0 raises the MSRV to Rust 1.88 so patched Diesel and `time`
releases can be used:

| Package | 0.5.0 floor | Reason |
|---|---:|---|
| `actix-web` | `4.15.0` | Current Rust 1.88 framework release |
| `actix-http` | `3.13.5` | Current patched Actix HTTP release |
| `time` | `0.3.55` | Fixes RUSTSEC-2026-0009 |
| `axum` | `0.8.9` | Current compatible Axum release |
| `diesel` | `2.3.12` | Fixes PostgreSQL protocol/COPY and published unsoundness advisories |

Actix-web's `http2` default feature is intentionally not enabled in repository
validation because Actix HTTP still maps it to the unpatched `h2` 0.3 line.
Compression, cookies, Unicode routing, macros, compatibility routing, and
WebSocket defaults remain enabled. This does **not** resolve the consumer
advisory: normal applications commonly depend on Actix-web with defaults, and
Cargo feature unification restores HTTP/2 and `h2` 0.3. No released patched
Actix path exists, so `ojs-actix` is excluded from 0.5.0 publication and release
automation pending actix/actix-web#4199.

The audit job generates an ephemeral lock from the final manifests plus the
pre-release local SDK source patch, then discards it. That checked graph reports
zero findings with Actix HTTP/2 disabled, but its package count and incidental
transitive versions are deliberately not treated as a committed release
contract. It is not a zero-advisory claim for normal Actix consumers.

## Findings implemented

| ID | Area | Resolution |
|---|---|---|
| F1 | Actix lifecycle ownership | Generation-aware `Stopped` / `Running` / `Stopping` transitions and exclusive join ownership |
| F2 | Actix graceful drain | Stop invokes SDK shutdown and awaits the original start future through active-job drain |
| F3 | Axum event wakeups | Interval stream re-polls until a waker is registered |
| F4 | Axum cron responses | Typed `Json<CronEntry>` / `Json<Vec<CronEntry>>` responders remove conversion panics |
| F5 | Security/MSRV | Rust 1.88 plus patched Axum, Diesel, and `time` floors; Actix consumer blocker documented |
| F6 | Example publication identity | All excluded examples use registry `ojs = "0.5.0"` and matching local integration crate version constraints |
| F7 | Release readiness | SHA-pinned CI, package/publish dry-runs for Axum/Diesel, consumer smoke, tag/version validation, SBOM, checksums, and provenance; Actix publication guard |

## Final validation

All Cargo commands used rustup's toolchain via
`PATH="$HOME/.cargo/bin:$PATH"`.

| Gate | Result |
|---|---|
| `cargo fmt --all -- --check` | PASS |
| Stable latest-dependency `cargo build --workspace --all-targets --all-features` | PASS |
| Stable latest-dependency `cargo test --workspace --all-features` | PASS; 81 passed, 6 ignored, 0 failed |
| Stable latest-dependency `cargo clippy --workspace --all-targets --all-features -- -D warnings` | PASS; 0 warnings |
| `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features` | PASS |
| Fresh-resolution Rust 1.88 build/test workspace matrix | PASS; 81 passed, 6 ignored, 0 failed |
| Actix, Axum, and Diesel example build/test on Rust 1.88 | PASS |
| Actix, Axum, and Diesel example build/test on stable | PASS |
| `cargo package` for publishable Axum/Diesel crates using the ephemeral local SDK patch | PASS |
| `cargo publish --dry-run --no-verify` for publishable Axum/Diesel crates using the ephemeral local SDK patch | PASS |
| Clean consumer resolving all three integration crates | PASS |
| Ephemeral-lock `cargo audit` with Actix HTTP/2 disabled | PASS; 0 findings in the checked graph; not representative of default-feature Actix consumers |
| Normal Actix consumer resolver/audit (`actix-web` defaults enabled) | EXPECTED BLOCK; resolves `h2 0.3.27` and reports RUSTSEC-2026-0258 |
| `actionlint` for CI and release workflows | PASS |

## Release boundary

The committed manifests target registry package `ojs = "0.5.0"`. Before that
crate is published, local and CI validation uses a temporary Cargo
`[patch.crates-io]` entry pointing at the checked-out SDK; the patch is deleted
after local validation and is created only inside CI jobs. Stable and Rust 1.88
jobs each perform a fresh dependency resolution rather than claiming an
unavailable registry lock. Only the audit job materializes an ephemeral lock
because `cargo audit` requires one. The tag-driven release workflow runs after
the SDK registry release, generates a final-source registry lock within that
release job, performs package and publish checks for `ojs-axum` and
`ojs-diesel`, rejects tag/package mismatches, and asserts that `ojs-actix`
remains `publish = false`.
