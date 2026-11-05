#!/usr/bin/env bash
# Minimal bootstrap only; all exchange behavior is implemented in Rust libraries.
set -euo pipefail
cd "$(dirname "$0")/.."
if [[ $(uname -s) != Linux || $(uname -m) != x86_64 ]]; then
    echo 'The first draft validates Linux x86_64. Other hosts are not yet supported.' >&2
    exit 1
fi
mkdir -p .tools
if [[ -x .tools/dotslash ]] && [[ $(.tools/dotslash --version) == 'DotSlash 0.5.9' ]]; then exit 0; fi
archive=$(mktemp)
trap 'rm -f "$archive"' EXIT
curl --fail --location --proto '=https' --tlsv1.2 \
  https://github.com/meta/dotslash/releases/download/v0.5.9/dotslash-linux-musl.x86_64.tar.gz -o "$archive"
printf '%s  %s\n' 5cefa0f258e0a58ae53c7a9a5be3890574ddd33d57c66bc9c143cb411012d72a "$archive" | sha256sum --check --status
tar -xzf "$archive" -C .tools dotslash
chmod +x .tools/dotslash
