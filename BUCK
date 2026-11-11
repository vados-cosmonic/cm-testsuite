load("//build-support:cargo.bzl", "cargo_workspace")
load("//third-party:crates.bzl", "CHECKSUMS", "CRATES")

_SOURCES = {path[len("rust/"):]: path for path in glob(["rust/Cargo.toml", "rust/Cargo.lock", "rust/crates/**/Cargo.toml", "rust/crates/**/*.rs", "rust/xtask/Cargo.toml", "rust/xtask/**/*.rs"])}
_SOURCES.update({path: path for path in glob(["examples/**", "examples/**/.cm-testsuite.toml"])})

cargo_workspace(name = "xtask", mode = "build", srcs = _SOURCES, crates = CRATES, checksums = CHECKSUMS, visibility = ["PUBLIC"])

cargo_workspace(name = "tests", mode = "test", srcs = _SOURCES, crates = CRATES, checksums = CHECKSUMS, visibility = ["PUBLIC"])

cargo_workspace(name = "lint", mode = "lint", srcs = _SOURCES, crates = CRATES, checksums = CHECKSUMS, visibility = ["PUBLIC"])

cargo_workspace(name = "fmt-check", mode = "fmt", srcs = _SOURCES, crates = CRATES, checksums = CHECKSUMS, visibility = ["PUBLIC"])
