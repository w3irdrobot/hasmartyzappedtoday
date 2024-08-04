#!/usr/bin/env bash

rm -rf build
mkdir build
cargo build --target=aarch64-unknown-linux-gnu --release
cp target/aarch64-unknown-linux-gnu/release/hasmartyzappedtoday build/
cp -r assets build/

echo 'built assets are available in the "build" directory'
