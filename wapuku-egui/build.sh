#!/bin/sh

set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
cd "$SCRIPT_DIR"

RUST_TOOLCHAIN=${WAPUKU_RUST_TOOLCHAIN:-nightly}

cargo_toolchain() {
    cargo +"$RUST_TOOLCHAIN" "$@"
}

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
cargo_toolchain build --locked --target-dir ./target --target wasm32-unknown-unknown --release -Z build-std=std,panic_abort --features getrandom/js

if [ -x ../vendor/wbg114/cli/target/debug/wasm-bindgen ]; then
    ../vendor/wbg114/cli/target/debug/wasm-bindgen \
        --out-dir ./pkg \
        --target web \
        ./target/wasm32-unknown-unknown/release/wapuku_egui.wasm
elif [ -f ../vendor/wbg114/cli/Cargo.toml ]; then
    cargo_toolchain build --locked --manifest-path ../vendor/wbg114/cli/Cargo.toml --bin wasm-bindgen
    ../vendor/wbg114/cli/target/debug/wasm-bindgen \
        --out-dir ./pkg \
        --target web \
        ./target/wasm32-unknown-unknown/release/wapuku_egui.wasm
elif command -v wasm-bindgen >/dev/null 2>&1; then
    wasm-bindgen --out-dir ./pkg --target web ./target/wasm32-unknown-unknown/release/wapuku_egui.wasm
elif [ -x ../vendor/wasm-bindgen-cli/crates/cli/target/debug/wasm-bindgen ]; then
    ../vendor/wasm-bindgen-cli/crates/cli/target/debug/wasm-bindgen \
        --out-dir ./pkg \
        --target web \
        ./target/wasm32-unknown-unknown/release/wapuku_egui.wasm
elif [ -f ../vendor/wasm-bindgen-cli/crates/cli/Cargo.toml ]; then
    cargo_toolchain build --locked --manifest-path ../vendor/wasm-bindgen-cli/crates/cli/Cargo.toml --bin wasm-bindgen
    ../vendor/wasm-bindgen-cli/crates/cli/target/debug/wasm-bindgen \
        --out-dir ./pkg \
        --target web \
        ./target/wasm32-unknown-unknown/release/wapuku_egui.wasm
elif [ -d ../../wasm-bindgen ]; then
    (
        cd ../../wasm-bindgen
        cargo +"$RUST_TOOLCHAIN" build --locked --package wasm-bindgen-cli --bin wasm-bindgen
        ./target/debug/wasm-bindgen \
            --out-dir ../wapuku/wapuku-egui/pkg \
            --target web \
            ../wapuku/wapuku-egui/target/wasm32-unknown-unknown/release/wapuku_egui.wasm
    )
else
    echo "wapuku-egui: need a wasm-bindgen CLI binary or ../../wasm-bindgen checkout" >&2
    exit 1
fi

(cd www && npm run build:main && npm run build:worker)
#wasm2wat --enable-threads ./target/wasm32-unknown-unknown/release/wapuku_egui.wasm > ./pkg/wapuku_egui.wat
## Note the usage of `--target no-modules` here which is required for passing
## the memory import to each wasm module.

  
#wasm2wat --enable-threads ./pkg/wapuku_egui_bg.wasm > ./pkg/wapuku_egui_bg.wat
