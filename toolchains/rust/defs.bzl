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

def host_build_tools():
    """Capture the native Windows shell/compiler environment supplied by Just."""
    if host_os() != "windows":
        return struct(bash = "bash", env = {})

    bash = read_root_config("host_tools", "bash")
    path = read_root_config("host_tools", "path")
    lib = read_root_config("host_tools", "lib")
    include = read_root_config("host_tools", "include")
    if not bash or not path or not lib or not include:
        fail("Use `just build` or `just check` from a Visual Studio Git Bash shell; Just passes the native tools to Buck.")
    return struct(bash = bash, env = {"PATH": path, "LIB": lib, "INCLUDE": include})
