#!/usr/bin/env bash
# Run the Cargo workspace using only Buck-provided Rust and registry inputs.
# Arguments are SDK, workspace, mode, output, then name/archive/checksum triples.
# Publish either the xtask executable or a check log; discard temporary build state.
set -euo pipefail

# Resolve Buck's paths before changing directories. The output may not exist yet.
sdk=$(realpath "$1")
source=$(realpath "$2")
mode=$3
out=$(realpath -m "$4")
shift 4

# Each action gets its own writable workspace and Cargo state. Dereference Buck's
# source symlinks so Cargo never writes into the declared input tree.
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
mkdir -p "$work/source" "$work/vendor" "$work/cargo-home"
cp -LR "$source/." "$work/source/"

# Buck already verifies each registry archive against Cargo.lock. Cargo's
# directory-source metadata records that package checksum; the empty file map
# avoids recomputing per-file checksums for these verified, freshly copied inputs.
while (($#)); do
    name=$1
    archive=$2
    checksum=$3
    shift 3

    mkdir -p "$work/vendor/$name"
    cp -R "$archive/." "$work/vendor/$name/"
    printf '{"files":{},"package":"%s"}\n' "$checksum" > "$work/vendor/$name/.cargo-checksum.json"
done

# Replace crates.io with the declared inputs and disable network access. An
# undeclared dependency must fail instead of resolving from a developer's cache.
cat > "$work/cargo-home/config.toml" <<CONFIG
[source.crates-io]
replace-with = "pinned-inputs"

[source.pinned-inputs]
directory = "$work/vendor"

[net]
offline = true
CONFIG

# Isolate Cargo's cache and build output from other actions and local builds.
export CARGO_HOME="$work/cargo-home"
export CARGO_TARGET_DIR="$work/target"

# Select the downloaded tools explicitly and exclude user-installed Rust proxies.
# System utilities and the native C compiler/linker remain host prerequisites.
export RUSTC="$sdk/bin/rustc"
export RUSTDOC="$sdk/bin/rustdoc"
export RUSTFMT="$sdk/bin/rustfmt"
export PATH="$sdk/bin:/usr/bin:/bin"

# The relocatable SDK carries shared libraries needed by its executables.
export LD_LIBRARY_PATH="$sdk/lib${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
export DYLD_LIBRARY_PATH="$sdk/lib${DYLD_LIBRARY_PATH:+:$DYLD_LIBRARY_PATH}"

# Buck owns the published output; Cargo builds only inside the temporary workspace.
mkdir -p "$(dirname "$out")"
cd "$work/source"

# Keep check logs as Buck outputs and print failures before the temporary workspace
# is removed. A successful build publishes only the executable used by Buck run.
case "$mode" in
    build)
        "$sdk/bin/cargo" build --locked --offline -p xtask
        cp "$CARGO_TARGET_DIR/debug/xtask" "$out"
        ;;

    test)
        "$sdk/bin/cargo" test --locked --offline --workspace > "$out" 2>&1 || {
            cat "$out"
            exit 1
        }
        cat "$out"
        ;;

    lint)
        "$sdk/bin/cargo" clippy --locked --offline --workspace --all-targets -- -D warnings > "$out" 2>&1 || {
            cat "$out"
            exit 1
        }
        cat "$out"
        ;;

    fmt)
        "$sdk/bin/cargo" fmt --all -- --check > "$out" 2>&1 || {
            cat "$out"
            exit 1
        }
        ;;
esac
