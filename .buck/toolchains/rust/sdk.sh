#!/usr/bin/env bash
# Assemble a relocatable SDK from Buck's checksum-verified Rust distribution.
# Inputs: archive, host triple, Rust version, and an empty output directory.
# Only compiler/build tools and the host standard library are extracted.
set -euo pipefail

# Diagnose malformed rule invocations before touching the output directory.
if (($# != 4)); then
    printf '%s\n' 'Usage: sdk.sh ARCHIVE HOST_TRIPLE VERSION OUTPUT' >&2
    exit 2
fi
archive=$1
triple=$2
version=$3
out=$4

# Buck renders Windows artifacts with backslashes. GNU tar interprets those as
# escapes in some arguments, so normalize both paths at the MSYS boundary.
if command -v cygpath >/dev/null 2>&1; then
    archive=$(cygpath -au "$archive")
    out=$(cygpath -au "$out")
fi

# Restrict archive member names to supported host distributions. Buck owns the
# version/checksum mapping; these checks catch wiring mistakes and unsafe paths.
case "$triple" in
    aarch64-apple-darwin|x86_64-apple-darwin|aarch64-unknown-linux-gnu|x86_64-unknown-linux-gnu|aarch64-pc-windows-msvc|x86_64-pc-windows-msvc) ;;
    *) printf 'Unsupported Rust SDK host: %s\n' "$triple" >&2; exit 2 ;;
esac
if [[ ! "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || [[ ! -r "$archive" ]]; then
    printf 'Expected a readable archive and a stable Rust version: %s (%s)\n' "$archive" "$version" >&2
    exit 2
fi
for tool in tar xz mkdir; do
    if ! command -v "$tool" >/dev/null 2>&1; then
        printf 'Rust SDK extraction requires %s on PATH.\n' "$tool" >&2
        exit 1
    fi
done

# Buck publishes this directory only after the action succeeds. Refuse to merge
# into an existing SDK so a stale executable cannot mask an incomplete archive.
if [[ -e "$out" ]]; then
    printf 'Rust SDK output already exists: %s\n' "$out" >&2
    exit 1
fi
mkdir -p "$out"
prefix=rust-$version-$triple

# The official archive nests each component under its own directory. Stripping
# two levels merges their bin/lib directories into the same relocatable sysroot.
if ! tar -xJf "$archive" -C "$out" --strip-components=2 \
    "$prefix/rustc/bin" "$prefix/rustc/lib" \
    "$prefix/rust-std-$triple/lib" \
    "$prefix/cargo/bin" "$prefix/rustfmt-preview/bin" "$prefix/clippy-preview/bin"; then
    printf 'Failed to extract Rust %s for %s; check archive availability and disk space.\n' "$version" "$triple" >&2
    exit 1
fi

# Detect incomplete distributions before any downstream build attempts to run.
suffix=
[[ "$triple" != *-windows-msvc ]] || suffix=.exe
for tool in rustc rustdoc cargo rustfmt cargo-fmt clippy-driver cargo-clippy; do
    if [[ ! -f "$out/bin/$tool$suffix" ]]; then
        printf 'Incomplete Rust SDK: missing %s\n' "$out/bin/$tool$suffix" >&2
        exit 1
    fi
done
if [[ ! -d "$out/lib/rustlib/$triple/lib" ]]; then
    printf 'Incomplete Rust SDK: host standard library is missing for %s\n' "$triple" >&2
    exit 1
fi
