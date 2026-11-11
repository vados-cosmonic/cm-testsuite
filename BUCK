load("@toolchains//rust:defs.bzl", "host_build_tools")
load("//build-support:cargo.bzl", "cargo_binary", "cargo_test", "cargo_workspace")
load("//third-party:crates.bzl", "CHECKSUMS", "CRATES")

_TOOLS = host_build_tools()

_SOURCES = {path[len("rust/"):]: path for path in glob(["rust/Cargo.toml", "rust/Cargo.lock", "rust/crates/**/Cargo.toml", "rust/crates/**/*.rs", "rust/xtask/Cargo.toml", "rust/xtask/**/*.rs"])}
_SOURCES.update({path: path for path in glob(["examples/**", "examples/**/.cm-testsuite.toml"])})

cargo_workspace(
    name = "artifacts",
    mode = "build",
    srcs = _SOURCES,
    crates = CRATES,
    checksums = CHECKSUMS,
    bash = _TOOLS.bash,
    env = _TOOLS.env,
    visibility = ["PUBLIC"],
)

cargo_binary(
    name = "xtask",
    artifacts = ":artifacts",
    visibility = ["PUBLIC"],
)

[cargo_test(
    name = suite + "-tests",
    suite = suite,
    artifacts = ":artifacts",
    env = _TOOLS.env,
    visibility = ["PUBLIC"],
) for suite in ["unit", "integration", "doc"]]

native.test_suite(
    name = "tests",
    test_deps = [":unit-tests", ":integration-tests", ":doc-tests"],
    visibility = ["PUBLIC"],
)

cargo_workspace(
    name = "lint",
    mode = "lint",
    srcs = _SOURCES,
    crates = CRATES,
    checksums = CHECKSUMS,
    bash = _TOOLS.bash,
    env = _TOOLS.env,
    visibility = ["PUBLIC"],
)

cargo_workspace(
    name = "fmt-check",
    mode = "fmt",
    srcs = _SOURCES,
    crates = {},
    checksums = {},
    bash = _TOOLS.bash,
    env = _TOOLS.env,
    visibility = ["PUBLIC"],
)

# Live publication stays outside the ordinary test suite and requires an opt-in.
cargo_test(
    name = "github-e2e",
    suite = "live",
    artifacts = ":artifacts",
    env = _TOOLS.env,
    visibility = ["PUBLIC"],
)
