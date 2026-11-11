#!/usr/bin/env bash
# Check local prerequisites through the same Just recipe used by every build.
# DotSlash must already be installed; Buck downloads the pinned Rust SDK when a
# build starts, so this entrypoint neither installs Rust nor changes PATH.
set -euo pipefail

# Resolve the checkout from this script, allowing invocation from any directory.
repository_root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)

# Fail before launching repository tools if the required launcher is unavailable.
if ! command -v dotslash >/dev/null 2>&1; then
    printf '%s\n' 'DotSlash is required. Install dotslash and ensure it is on PATH.' >&2
    exit 1
fi

# Use the pinned Just launcher so only DotSlash is needed to enter the workflow.
# The doctor recipe owns platform and system-tool checks to avoid duplicating them.
exec dotslash "$repository_root/bins/just" --justfile "$repository_root/Justfile" doctor
