#!/usr/bin/env bash
set -euo pipefail
archive=$1
triple=$2
version=$3
out=$4
prefix=rust-$version-$triple
mkdir -p "$out"
# Merge the selected distribution components directly into one native sysroot.
tar -xJf "$archive" -C "$out" --strip-components=2 \
    "$prefix/rustc/bin" "$prefix/rustc/lib" \
    "$prefix/rust-std-$triple/lib" \
    "$prefix/cargo/bin" "$prefix/rustfmt-preview/bin" "$prefix/clippy-preview/bin"
