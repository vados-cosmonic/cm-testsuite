"""Downloaded Rust, adapted from WebAssembly/wasi-testsuite (see THIRD_PARTY.md).

Use the same release/checksum metadata, but extract only the native build tools,
not documentation or Wasm SDK components that this repository does not need.
"""
load("@wasmono//:defs.bzl", "host_arch", "host_os")
load(":releases.bzl", "RUST_HOST_RELEASES", "rust_host_url")

def host_rust_triple():
    suffixes = {
        "linux": "unknown-linux-gnu",
        "macos": "apple-darwin",
        "windows": "pc-windows-msvc",
    }
    suffix = suffixes.get(host_os())
    if suffix == None:
        fail("Unsupported Rust host operating system: " + host_os())
    triple = host_arch() + "-" + suffix
    if triple not in RUST_HOST_RELEASES:
        fail("No pinned Rust SDK for host: " + triple)
    return triple


def download_rust_host(name, triple):
    native.http_file(
        name = name,
        urls = [rust_host_url(triple)],
        sha256 = RUST_HOST_RELEASES[triple],
        out = "rust.tar.xz",
    )
