#!/bin/sh

set -ex

# A couple of steps are necessary to get this build working which makes it slightly
# nonstandard compared to most other builds.
#
# * First, the Rust standard library needs to be recompiled with atomics
#   enabled. to do that we use Cargo's unstable `-Zbuild-std` feature.
#
# * Next we need to compile everything with the `atomics` and `bulk-memory`
#   features enabled, ensuring that LLVM will generate atomic instructions,
#   shared memory, passive segments, etc.

RUSTFLAGS='-C target-feature=+atomics,+bulk-memory,+mutable-globals' \
  cargo +nightly build --target-dir ./target --target wasm32-unknown-unknown --release -Z build-std=std,panic_abort

(cd /enc/my-dev/wasm-bindgen && cargo run --package wasm-bindgen-cli --bin wasm-bindgen -- --out-dir /enc/my-dev/wapuku/wapuku-ui/pkg/  --target web   /enc/my-dev/wapuku/wapuku-ui/target/wasm32-unknown-unknown/release/wapuku_ui.wasm)
(cd www && npm run build:main && npm run build:worker)
#wasm2wat --enable-threads ./target/wasm32-unknown-unknown/release/wasm_threads_template.wasm > ./pkg/wasm_threads_template.wat
## Note the usage of `--target no-modules` here which is required for passing
## the memory import to each wasm module.
#cargo run -p wasm-bindgen-cli -- \
#  ./target/wasm32-unknown-unknown/release/wasm_threads_template.wasm \
#  --out-dir pkg \
#  --target web \
#  --keep-lld-exports
#  
#wasm2wat --enable-threads ./pkg/wasm_threads_template_bg.wasm > ./pkg/wasm_threads_template_bg.wat
