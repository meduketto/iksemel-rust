#!/bin/sh

cargo fmt --all --check
cargo clippy
cargo test
