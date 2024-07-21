#!/usr/bin/env bash

rm -rf build
mkdir build
cargo build --release
cp target/release/hasmartyzappedtoday build/
cp -r assets build/

echo 'built assets are available in the "build" directory'
