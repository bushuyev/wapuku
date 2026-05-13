#!/bin/sh

set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
cd "$SCRIPT_DIR"

RUST_TOOLCHAIN=${WAPUKU_RUST_TOOLCHAIN:-nightly}

cargo_toolchain() {
    cargo +"$RUST_TOOLCHAIN" "$@"
}

build_vendor_wasm_bindgen() {
    (
        cd "$1"
        cargo +"$RUST_TOOLCHAIN" build --locked --bin wasm-bindgen
    )
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

cargo_toolchain metadata --manifest-path ../Cargo.toml --format-version 1 --filter-platform wasm32-unknown-unknown --locked \
    | python3 -c 'import json, sys; data = json.load(sys.stdin); pkgs = [p for p in data["packages"] if p["name"] == "getrandom" and p["version"] == "0.2.17"]; assert pkgs, "getrandom 0.2.17 missing from metadata"; manifest = pkgs[0]["manifest_path"]; print("getrandom 0.2.17 manifest:", manifest); assert "/vendor/getrandom/" in manifest, manifest'

#RUSTFLAGS='-C target-feature=+atomics,+bulk-memory,+mutable-globals' \
cargo_toolchain build --manifest-path ../Cargo.toml -p wapuku-egui --locked --target-dir ./target --target wasm32-unknown-unknown --release -Z build-std=std,panic_abort --features getrandom/js

if [ -x ../vendor/wbg114/cli/target/debug/wasm-bindgen ]; then
    ../vendor/wbg114/cli/target/debug/wasm-bindgen \
        --out-dir ./pkg \
        --target web \
        ./target/wasm32-unknown-unknown/release/wapuku_egui.wasm
elif [ -f ../vendor/wbg114/cli/Cargo.toml ]; then
    build_vendor_wasm_bindgen ../vendor/wbg114/cli
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
    build_vendor_wasm_bindgen ../vendor/wasm-bindgen-cli/crates/cli
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
