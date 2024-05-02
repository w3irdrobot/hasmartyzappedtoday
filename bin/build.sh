#!/usr/bin/env bash
# shameless theft from https://github.com/cbiffle/bare-metal-wasm-example/blob/master/build.sh

set -euxo pipefail

BINARY=dist/assets/dioxus/hasmartyzappedtoday_bg.wasm

dx build --release
wasm-strip ${BINARY}
wasm-opt -o ${BINARY} -Oz ${BINARY}
ls -lh ${BINARY}
