"""Downloaded Rust, adapted from WebAssembly/wasi-testsuite (see THIRD_PARTY.md).

Use the same release/checksum metadata, but extract only the native build tools,
not documentation or Wasm SDK components that this repository does not need.
"""
load("@wasmono//:defs.bzl", "host_arch", "host_os")
load(":releases.bzl", "RUST_HOST_RELEASES", "rust_host_url")

def host_rust_triple():
    if host_arch() != "x86_64" or host_os() != "linux":
        fail("The initial cm-testsuite build supports Linux x86_64")
    return "x86_64-unknown-linux-gnu"

def download_rust_host(name, triple):
    native.http_file(
        name = name,
        urls = [rust_host_url(triple)],
        sha256 = RUST_HOST_RELEASES[triple],
        out = "rust.tar.xz",
    )
