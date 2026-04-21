#!/bin/sh

set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
DIST_DIR="$SCRIPT_DIR/dist"
PKG_DIR="$SCRIPT_DIR/../pkg"
ensure_build() {
    if [ ! -f "$PKG_DIR/wapuku_egui.js" ] || [ ! -f "$PKG_DIR/wapuku_egui_bg.wasm" ]; then
        echo "wapuku-egui: wasm pkg is missing; running ../build.sh" >&2
        (
            cd "$SCRIPT_DIR/.."
            ./build.sh
        )
        return
    fi

    if [ ! -f "$DIST_DIR/index.html" ] || [ ! -f "$DIST_DIR/index.js" ] || [ ! -f "$DIST_DIR/wasm-worker.js" ]; then
        echo "wapuku-egui: web assets are missing; rebuilding dist/" >&2
        (
            cd "$SCRIPT_DIR"
            npm run build:main
            npm run build:worker
        )
    fi
}

ensure_build

cd "$SCRIPT_DIR"
exec node ./serve.mjs
