# AGENTS.md

## Repo Summary

This repository is a Rust workspace with two browser-facing frontends:

- `wapuku-model`: data/model logic built around Polars.
- `wapuku-resources`: resource loading.
- `wapuku-common-web`: shared WebAssembly/web-worker glue.
- `wapuku-ui`: older WebGPU/winit frontend.
- `wapuku-egui`: egui/eframe frontend.

Each frontend also has its own `www/` folder with an npm/webpack toolchain.

## Current Ground Truth

As of 2026-03-19, the old machine-local Rust path dependencies have been removed from this repo and replaced with registry dependencies where possible.

Key points:

- The workspace no longer depends on local `../../polars`, `../../wasm-bindgen`, or `../../rayon` checkouts.
- `wapuku-model` now uses `polars = 0.36.2` from crates.io.
- wasm crates now use registry `wasm-bindgen` instead of local path patches.
- The direct local `rayon` dependency was removed.

## Important Finding About The Old Rayon/Wasm Setup

The repo had a custom worker/bootstrap layer that looked like a hand-rolled `wasm-bindgen-rayon` setup.

However, the currently used code path was not actually using a Rayon thread pool for app work:

- `PoolWorker::run_in_pool(...)` posts a closure to a dedicated Web Worker.
- `wapuku-common-web::run_in_pool` just executes that closure in the worker.
- The old `get_pool` / Rayon thread-pool bookkeeping had no call sites in the app logic.

Because of that, removing the local `rayon` fork did not remove the app's actual background execution path. The current background model is still "run work in one dedicated worker", not "fan out work across a Rayon pool".

The backup at `../wasm-bindgen-rayon/` is the upstream helper crate repo, not a checkout that contains patched `wasm-bindgen/` and `rayon/` source trees. It was not wired into the workspace in this pass.

## Rust Changes Made

- Removed stale root and crate-level `[patch.crates-io]` entries that pointed at missing local checkouts.
- Removed nested `[workspace]` sections from member crates that were making Cargo treat them as separate workspace roots.
- Replaced path `polars` dependencies with registry `0.36.2`.
- Simplified `wapuku-common-web/src/lib.rs`:
  - kept the JS-facing `init_pool`, `init_worker`, and `run_in_pool` API stable;
  - removed the unused Rayon pool storage / `get_pool` implementation;
  - made `init_pool` a no-op compatibility hook.
- Removed stale `#![feature(async_fn_in_trait)]` crate attributes.
- Removed stale Cargo manifest sections that Cargo was ignoring (`[unstable]`, `[env]`).

## Rust Verification Status

Verified working on the pinned project toolchain (`nightly-2023-12-23-x86_64-unknown-linux-gnu`):

- `cargo check -p wapuku-model`
- `cargo check -p wapuku-common-web`

Still blocked:

- `cargo check -p wapuku-ui --target wasm32-unknown-unknown`
- `cargo check -p wapuku-egui --target wasm32-unknown-unknown`

Those wasm checks now fail for environment reasons, not missing path dependencies:

- `clang` is missing for `lz4-sys` / `zstd-sys` when compiling the Polars stack for `wasm32-unknown-unknown`.

## Frontend / npm Status

Both frontend lockfiles were already refreshed in the prior pass.

Current npm audit status:

- `wapuku-ui/www`: `0` vulnerabilities
- `wapuku-egui/www`: `0` vulnerabilities

Full frontend bundle builds still depend on generated wasm `pkg/` directories:

- `wapuku-ui/pkg`
- `wapuku-egui/pkg`

Until wasm builds succeed, webpack builds fail because imports like `../pkg/wapuku_ui` and `../pkg/wapuku_egui` do not exist.

## Rust Security Status

A current `cargo-audit` scan was run with a modern host toolchain via:

- `/tmp/cargo-audit-stable/bin/cargo-audit audit`

Result on the current lockfile:

- `2` remaining Rust vulnerabilities

Remaining vulnerabilities:

1. `fast-float 0.2.0` (`RUSTSEC-2025-0003`)
   - RustSec reports no patched upstream release.
   - This currently comes in through the Polars stack.
   - Fixing it will require either:
     - a newer Polars release that stops depending on `fast-float`, or
     - a deeper patch/fork strategy (for example swapping to `fast-float2` if compatible).

2. `time 0.3.31` (`RUSTSEC-2026-0009`)
   - A fixed release exists (`>= 0.3.47`).
   - That fix line pulls `time-core` requiring newer Rust/Cargo than the pinned project toolchain.
   - Attempting to keep the `time` fix in the lockfile broke builds on `nightly-2023-12-23`.
   - Conclusion: this one is toolchain-blocked, not semver-blocked.

RustSec informational warnings still remain for unmaintained/yanked/unsound crates, mostly via:

- old Polars subdependencies
- GTK3-related crates pulled by `rfd`
- older ecosystem crates in the egui/desktop path

Those are not all immediate vulnerabilities, but they are good upgrade targets for future cleanup.

## Rust Dependency Refresh Applied

The current `Cargo.lock` was refreshed conservatively and now includes newer compatible versions for several fixable crates, including:

- `bytes 1.11.1`
- `futures-util 0.3.32`
- `h2 0.3.27`
- `iana-time-zone 0.1.65`
- `idna 1.1.0`
- `mio 0.8.11`
- `openssl 0.10.76`
- `reqwest 0.11.27`
- `smallvec 1.13.2`
- `tokio 1.38.2`
- `url 2.5.4`

Rayon was pinned back to versions compatible with the pinned nightly:

- `rayon 1.8.0`
- `rayon-core 1.12.0`

## Toolchain Notes

The repository still pins:

- `nightly-2023-12-23-x86_64-unknown-linux-gnu`

That matters because:

- newer security fixes may silently require newer Rust/Cargo;
- the wasm build uses nightly-only `build-std` flow;
- moving the toolchain forward may unlock more security updates, but should be treated as a separate migration.

There is also a newer installed toolchain on this machine:

- `nightly-2025-02-01-x86_64-unknown-linux-gnu`

It was enough for some modern dependency checks, but not enough to accept `time >= 0.3.47`, which needs Rust 1.88+.

## Recommended Next Steps

1. Install or configure `clang` for `wasm32-unknown-unknown` builds.
2. Re-run the wasm frontend builds to regenerate `pkg/` output.
3. Re-test `npm run build:main` and `npm run build:js-worker` in both frontends.
4. Decide whether the remaining `time` advisory justifies a toolchain upgrade.
5. Decide whether the remaining `fast-float` advisory justifies a larger Polars upgrade or a patched dependency strategy.
6. If reducing informational RustSec warnings matters, investigate major upgrades for `rfd` / `eframe` / GTK3-related dependencies.

## High-Signal Commands

Commands that were useful in this pass:

- `cargo check -p wapuku-model`
- `cargo check -p wapuku-common-web`
- `cargo check -p wapuku-ui --target wasm32-unknown-unknown`
- `cargo check -p wapuku-egui --target wasm32-unknown-unknown`
- `npm audit --package-lock-only --json`
- `/tmp/cargo-audit-stable/bin/cargo-audit audit`
