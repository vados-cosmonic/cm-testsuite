#!/usr/bin/env bash
set -euo pipefail
sdk=$(realpath "$1")
source=$(realpath "$2")
mode=$3
out=$(realpath -m "$4")
shift 4
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
mkdir -p "$work/source" "$work/vendor" "$work/cargo-home"
cp -LR "$source/." "$work/source/"
while (($#)); do
    name=$1
    archive=$2
    checksum=$3
    shift 3
    mkdir -p "$work/vendor/$name"
    cp -R "$archive/." "$work/vendor/$name/"
    printf '{"files":{},"package":"%s"}\n' "$checksum" > "$work/vendor/$name/.cargo-checksum.json"
done
cat > "$work/cargo-home/config.toml" <<CONFIG
[source.crates-io]
replace-with = "pinned-inputs"
[source.pinned-inputs]
directory = "$work/vendor"
[net]
offline = true
CONFIG
export CARGO_HOME="$work/cargo-home"
export CARGO_TARGET_DIR="$work/target"
export RUSTC="$sdk/bin/rustc"
export RUSTDOC="$sdk/bin/rustdoc"
export RUSTFMT="$sdk/bin/rustfmt"
export PATH="$sdk/bin:/usr/bin:/bin"
export LD_LIBRARY_PATH="$sdk/lib${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
export DYLD_LIBRARY_PATH="$sdk/lib${DYLD_LIBRARY_PATH:+:$DYLD_LIBRARY_PATH}"
mkdir -p "$(dirname "$out")"
cd "$work/source"
case "$mode" in
    build) "$sdk/bin/cargo" build --locked --offline -p xtask; cp "$CARGO_TARGET_DIR/debug/xtask" "$out" ;;
    test) "$sdk/bin/cargo" test --locked --offline --workspace > "$out" 2>&1 || { cat "$out"; exit 1; }; cat "$out" ;;
    lint) "$sdk/bin/cargo" clippy --locked --offline --workspace --all-targets -- -D warnings > "$out" 2>&1 || { cat "$out"; exit 1; }; cat "$out" ;;
    fmt) "$sdk/bin/cargo" fmt --all -- --check > "$out" 2>&1 || { cat "$out"; exit 1; } ;;
esac
