#!/usr/bin/env bash
# Run the Cargo workspace using only Buck-provided Rust and registry inputs.
# Arguments are SDK, workspace, mode, output, then name/archive/checksum triples.
# Publish either the xtask executable or a check log; discard temporary build state.
set -euo pipefail

# Convert Windows artifact paths before handing them to POSIX file utilities.
# On other hosts, retain their spelling so spaces and unusual path names survive.
as_shell_path() {
    if command -v cygpath >/dev/null 2>&1; then
        cygpath -au "$1"
    else
        printf '%s\n' "$1"
    fi
}

# Resolve Buck's paths before changing directories. The output may not exist yet.
sdk=$(cd -- "$(as_shell_path "$1")" && pwd)
source=$(cd -- "$(as_shell_path "$2")" && pwd)
mode=$3
output_path=$(as_shell_path "$4")
mkdir -p "$(dirname -- "$output_path")"
out=$(cd -- "$(dirname -- "$output_path")" && pwd)/$(basename -- "$output_path")
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
    archive=$(as_shell_path "$2")
    checksum=$3
    shift 3

    mkdir -p "$work/vendor/$name"
    cp -R "$archive/." "$work/vendor/$name/"
    printf '{"files":{},"package":"%s"}\n' "$checksum" > "$work/vendor/$name/.cargo-checksum.json"
done

# Native Windows tools need drive-letter paths; POSIX hosts keep their usual paths.
# Retain the caller's compiler environment (MSVC developer shell or Xcode tools).
suffix=
vendor="$work/vendor"
if command -v cygpath >/dev/null 2>&1; then
    suffix=.exe
    vendor=$(cygpath -m "$vendor")
fi

# Replace crates.io with the declared inputs and disable network access. An
# undeclared dependency must fail instead of resolving from a developer's cache.
cat > "$work/cargo-home/config.toml" <<CONFIG
[source.crates-io]
replace-with = "pinned-inputs"

[source.pinned-inputs]
directory = '$vendor'

[net]
offline = true
CONFIG

# Isolate Cargo's cache and build output from other actions and local builds.
export CARGO_HOME="$work/cargo-home"
export CARGO_TARGET_DIR="$work/target"
export CM_TESTSUITE_EXAMPLES="$work/source/examples"

# Select the downloaded tools explicitly, ahead of user-installed Rust proxies.
# System utilities and the native C compiler/linker remain host prerequisites.
export RUSTC="$sdk/bin/rustc"
export RUSTDOC="$sdk/bin/rustdoc"
export RUSTFMT="$sdk/bin/rustfmt$suffix"
if [[ -n "$suffix" ]]; then
    export CM_TESTSUITE_EXAMPLES="$(cygpath -m "$CM_TESTSUITE_EXAMPLES")"
    export CARGO_HOME="$(cygpath -m "$CARGO_HOME")"
    export CARGO_TARGET_DIR="$(cygpath -m "$CARGO_TARGET_DIR")"
    export RUSTC="$(cygpath -m "$sdk/bin/rustc.exe")"
    export RUSTDOC="$(cygpath -m "$sdk/bin/rustdoc.exe")"
    export RUSTFMT="$(cygpath -m "$sdk/bin/rustfmt.exe")"

    # Git Bash also ships a link.exe. Select MSVC's linker beside cl.exe rather
    # than letting rustc pick the unrelated filesystem utility earlier on PATH.
    msvc_bin=$(dirname -- "$(command -v cl.exe)")
    msvc_linker=$(cygpath -m "$msvc_bin/link.exe")
    export CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER="$msvc_linker"
    export CARGO_TARGET_AARCH64_PC_WINDOWS_MSVC_LINKER="$msvc_linker"
fi
export PATH="$sdk/bin:$PATH"

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
        "$sdk/bin/cargo$suffix" build --locked --offline -p xtask
        cp "$CARGO_TARGET_DIR/debug/xtask$suffix" "$out"
        ;;

    test)
        "$sdk/bin/cargo$suffix" test --locked --offline --workspace > "$out" 2>&1 || {
            cat "$out"
            exit 1
        }
        cat "$out"
        ;;

    lint)
        "$sdk/bin/cargo$suffix" clippy --locked --offline --workspace --all-targets -- -D warnings > "$out" 2>&1 || {
            cat "$out"
            exit 1
        }
        cat "$out"
        ;;

    fmt)
        "$sdk/bin/cargo$suffix" fmt --all -- --check > "$out" 2>&1 || {
            cat "$out"
            exit 1
        }
        ;;
esac
