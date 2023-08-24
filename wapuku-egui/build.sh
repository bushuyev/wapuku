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

rm -rf pkg
rm -rf www/dist

#RUSTFLAGS='-C target-feature=+atomics,+bulk-memory,+mutable-globals' \
cargo +nightly-2023-07-27-x86_64-unknown-linux-gnu build --target-dir ./target --target wasm32-unknown-unknown --release -Z build-std=std,panic_abort

(cd ../../wasm-bindgen && cargo +nightly run --package wasm-bindgen-cli --bin wasm-bindgen -- --out-dir ../wapuku/wapuku-egui/pkg/  --target web   ../wapuku/wapuku-egui/target/wasm32-unknown-unknown/release/wapuku_egui.wasm)
(cd www && npm run build:main && npm run build:worker)
#wasm2wat --enable-threads ./target/wasm32-unknown-unknown/release/wapuku_egui.wasm > ./pkg/wapuku_egui.wat
## Note the usage of `--target no-modules` here which is required for passing
## the memory import to each wasm module.

  
#wasm2wat --enable-threads ./pkg/wapuku_egui_bg.wasm > ./pkg/wapuku_egui_bg.wat
